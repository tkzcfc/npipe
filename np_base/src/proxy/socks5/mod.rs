pub mod target_addr;

use crate::net::{SendMessageFuncType, WriterMessage};
use crate::proxy::common::SessionCommonInfo;
use crate::proxy::inlet::{InletDataEx, InletProxyType};
use crate::proxy::socks5::target_addr::TargetAddr;
use crate::proxy::ProxyMessage;
use anyhow::anyhow;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::{error, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;

const SOCKS5_VERSION: u8 = 0x05;

const SOCKS5_AUTH_METHOD_NONE: u8 = 0x00;
// const SOCKS5_AUTH_METHOD_GSSAPI: u8 = 0x01; // not support
const SOCKS5_AUTH_METHOD_PASSWORD: u8 = 0x02;
const SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE: u8 = 0xff;

const SOCKS5_CMD_TCP_CONNECT: u8 = 0x01;
const SOCKS5_CMD_TCP_BIND: u8 = 0x02;
const SOCKS5_CMD_UDP_ASSOCIATE: u8 = 0x03;

const SOCKS5_ADDR_TYPE_IPV4: u8 = 0x01;
const SOCKS5_ADDR_TYPE_DOMAIN_NAME: u8 = 0x03;
const SOCKS5_ADDR_TYPE_IPV6: u8 = 0x04;

///
///tcp流程:
///       有密码模式
///               Init -> Verification -> Connect -> Connecting -> RunWithTcp
///       无密码模式
///               Init -> Connect -> Connecting -> RunWithTcp
///
///udp流程:
///       有密码模式
///               Init -> Verification -> Connect -> Connecting -> RunWithUdp
///       无密码模式
///               Init -> Connect -> Connecting -> RunWithUdp
///
#[derive(Debug)]
enum Status {
    Init,
    Verification,
    Connect,
    Connecting(bool),
    /// 运行中
    RunWithTcp,
    /// 运行中
    RunWithUdp(Arc<UdpSocket>),
}

pub struct Socks5Context {
    status: Status,
    buffer: Vec<u8>,
    write_msg_tx: mpsc::UnboundedSender<WriterMessage>,
    output: Sender<ProxyMessage>,
    data_ex: Arc<InletDataEx>,
    target_addr: Option<TargetAddr>,
    session_id: u32,
    addr: SocketAddr,
    common_data: SessionCommonInfo,

    read_input_task_handle: Option<(JoinHandle<()>, oneshot::Sender<()>)>,
    udp_task_handle: Option<(JoinHandle<()>, oneshot::Sender<()>)>,
}

impl Socks5Context {
    pub async fn new(
        write_msg_tx: mpsc::UnboundedSender<WriterMessage>,
        output: Sender<ProxyMessage>,
        session_id: u32,
        addr: SocketAddr,
        data_ex: Arc<InletDataEx>,
        common_data: SessionCommonInfo,
    ) -> (Arc<RwLock<Self>>, mpsc::UnboundedSender<ProxyMessage>) {
        let (proxy_msg_tx, mut proxy_msg_rx) = mpsc::unbounded_channel::<ProxyMessage>();

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let context = Arc::new(RwLock::new(Self {
            status: Status::Init,
            buffer: vec![],
            write_msg_tx,
            output,
            data_ex,
            target_addr: None,
            session_id,
            addr,
            common_data,
            read_input_task_handle: None,
            udp_task_handle: None,
        }));

        let context_cloned = context.clone();
        let handle = tokio::spawn(async move {
            let task = async {
                loop {
                    if let Some(message) = proxy_msg_rx.recv().await {
                        if let Err(err) = context_cloned
                            .write()
                            .await
                            .on_recv_proxy_message(message)
                            .await
                        {
                            error!("on_recv_proxy_message: {err}")
                        }
                    }
                }
            };

            select! {
                _= task =>{},
                _= shutdown_rx =>{}
            }
        });

        context.write().await.read_input_task_handle = Some((handle, shutdown_tx));

        (context, proxy_msg_tx)
    }

    pub async fn recv_frame(&mut self, mut frame: Vec<u8>) -> anyhow::Result<()> {
        match &self.status {
            Status::Init => {
                self.buffer.extend_from_slice(&frame);
                self.on_socks5_init()?;
            }
            Status::Verification => {
                self.buffer.extend_from_slice(&frame);
                self.on_socks5_verification()?;
            }
            Status::Connect => {
                self.buffer.extend_from_slice(&frame);
                self.on_socks5_connect().await?;
            }
            Status::Connecting(_) => {
                warn!("Status::Connecting should not receive other data");
            }
            Status::RunWithTcp => {
                frame = self.common_data.encode_data_and_limiting(frame).await?;
                self.output
                    .send(ProxyMessage::I2oSendData(self.session_id, frame))
                    .await?;
            }
            Status::RunWithUdp(_) => {
                warn!("SOCKS5_CMD_UDP_ASSOCIATE mode should not receive other data");
            }
        }

        Ok(())
    }

    fn on_socks5_init(&mut self) -> anyhow::Result<()> {
        // +----+----------+----------+
        // |VER | NMETHODS | METHODS  |
        // +----+----------+----------+
        // | 1  |    1     | 1 to 255 |
        // +----+----------+----------+

        if self.buffer.len() < 3 {
            return Ok(());
        }

        if self.buffer[0] == SOCKS5_VERSION {
            let num_of_methods = self.buffer[1] as usize;
            let num_of_package = num_of_methods + 2;

            // 消息未接收完成
            if self.buffer.len() < num_of_package {
                return Ok(());
            }

            let methods = &self.buffer[2..num_of_package];

            let method = if self.data_ex.username.is_empty() && self.data_ex.password.is_empty() {
                // 不需要密码
                if methods.contains(&SOCKS5_AUTH_METHOD_NONE) {
                    SOCKS5_AUTH_METHOD_NONE
                } else {
                    SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE
                }
            } else {
                // 密码认证
                if methods.contains(&SOCKS5_AUTH_METHOD_PASSWORD) {
                    SOCKS5_AUTH_METHOD_PASSWORD
                } else {
                    SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE
                }
            };

            if method != SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE {
                let response: Vec<u8> = vec![SOCKS5_VERSION, method];
                self.write_msg_tx
                    .send(WriterMessage::Send(response, true))?;

                self.buffer.drain(0..num_of_package);
                self.status = if method == SOCKS5_AUTH_METHOD_NONE {
                    Status::Connect
                } else {
                    Status::Verification
                };
                return Ok(());
            }
        }

        // 无法认证
        let response: Vec<u8> = vec![SOCKS5_VERSION, SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE];
        self.write_msg_tx
            .send(WriterMessage::Send(response, true))?;
        self.write_msg_tx
            .send(WriterMessage::CloseDelayed(Duration::from_millis(10)))?;
        Ok(())
    }

    fn on_socks5_verification(&mut self) -> anyhow::Result<()> {
        // +----+-----+-----------+------+----------+
        // |VER | UNL |  UNM      | PWL  | PWD      |
        // +----+-----+-----------+------+----------+
        // | 1  |  1  |  Variable |  1   | Variable |
        // +----+-----+-----------+------+----------+

        if self.buffer.len() < 4 {
            return Ok(());
        }
        let ver = self.buffer[0];
        let unl: usize = self.buffer[1] as usize;
        let pwl: usize = self.buffer[2 + unl] as usize;
        let num_of_package = unl + pwl + 3;

        let unm = &self.buffer[2..(2 + unl)];
        let pwd = &self.buffer[(3 + unl)..(3 + unl + pwl)];

        // Response
        // +----+-----+
        // |VER | RET |
        // +----+-----+
        // | 1  |  1  |
        // +----+-----+
        // 0x00 表示成功，0x01 表示失败
        if unm == self.data_ex.username.as_bytes() && pwd == self.data_ex.password.as_bytes() {
            let response: Vec<u8> = vec![ver, 0x00];
            self.write_msg_tx
                .send(WriterMessage::Send(response, true))?;

            self.buffer.drain(0..num_of_package);
            self.status = Status::Connect;
        } else {
            let response: Vec<u8> = vec![ver, 0x01];
            self.write_msg_tx
                .send(WriterMessage::Send(response, true))?;
            self.write_msg_tx
                .send(WriterMessage::CloseDelayed(Duration::from_secs(1)))?;
        }

        Ok(())
    }

    async fn on_socks5_connect(&mut self) -> anyhow::Result<()> {
        // +----+-----+-------+------+----------+----------+
        // |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
        // +----+-----+-------+------+----------+----------+
        // | 1  |  1  | X'00' |  1   | Variable |    2     |
        // +----+-----+-------+------+----------+----------+
        if self.buffer.len() < 8 {
            return Ok(());
        }
        let head_data: Vec<_> = self.buffer.drain(0..4).collect();

        let ver = head_data[0];
        let cmd = head_data[1];
        let rsv = head_data[2];
        let address_type = head_data[3];

        let mut support = true;
        if ver != SOCKS5_VERSION || rsv != 0x00 {
            support = false;
        }

        if support {
            match cmd {
                SOCKS5_CMD_TCP_CONNECT | SOCKS5_CMD_UDP_ASSOCIATE => {
                    let addr_result = target_addr::read_address(&self.buffer, address_type)?;

                    if let Some((mut target_addr, addr_data_len)) = addr_result {
                        let is_tcp = match cmd {
                            SOCKS5_CMD_TCP_CONNECT => true,
                            SOCKS5_CMD_UDP_ASSOCIATE => {
                                let mut addr = self.addr.clone();
                                addr.set_port(target_addr.port());
                                target_addr = TargetAddr::Ip(addr);
                                false
                            }
                            _ => {
                                panic!("unknown cmd:{cmd}")
                            }
                        };

                        self.output
                            .send(ProxyMessage::I2oConnect(
                                self.session_id,
                                InletProxyType::SOCKS5.to_u8(),
                                is_tcp,
                                self.common_data.is_compressed,
                                target_addr.to_string(),
                                self.common_data.encryption_method.to_string(),
                                BASE64_STANDARD.encode(&self.common_data.encryption_key),
                                self.addr.to_string(),
                            ))
                            .await?;

                        self.target_addr = Some(target_addr);
                        if addr_data_len != self.buffer.len() {
                            warn!("Address data length error, address data length: {}, actual length: {}", addr_data_len, self.buffer.len());
                        }

                        self.buffer.clear();
                        self.status = Status::Connecting(is_tcp);
                    } else {
                        // 还原数据
                        self.buffer.splice(0..0, head_data);
                    }
                    return Ok(());
                }
                // not support
                SOCKS5_CMD_TCP_BIND => {}
                _ => {}
            }
        }

        let response: Vec<u8> = vec![
            SOCKS5_VERSION,
            0x07, // 0x07不支持的命令
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        self.write_msg_tx
            .send(WriterMessage::Send(response, true))?;
        self.write_msg_tx
            .send(WriterMessage::CloseDelayed(Duration::from_secs(1)))?;
        Ok(())
    }

    pub async fn on_destroy(&mut self) {
        cancel_task(self.read_input_task_handle.take()).await;
        cancel_task(self.udp_task_handle.take()).await;
    }

    async fn on_recv_proxy_message(&mut self, proxy_message: ProxyMessage) -> anyhow::Result<()> {
        match proxy_message {
            ProxyMessage::O2iConnect(_session_id, success, error_msg) => {
                if !success {
                    error!("socks5 connect error: {error_msg}");
                }
                self.on_recv_o2i_connect(success).await?;
            }

            ProxyMessage::O2iRecvData(session_id, data) => {
                self.on_recv_o2i_recv_data(session_id, data).await?;
            }
            ProxyMessage::O2iRecvDataFrom(session_id, data, peer_addr) => {
                self.on_recv_o2i_recv_data_from(session_id, data, peer_addr)
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_recv_o2i_recv_data(
        &self,
        session_id: u32,
        mut data: Vec<u8>,
    ) -> anyhow::Result<()> {
        match self.status {
            Status::RunWithTcp => {
                let data_len = data.len();
                data = self.common_data.decode_data(data)?;

                // 写入完毕回调
                let output = self.output.clone();
                let callback: SendMessageFuncType = Box::new(move || {
                    let output = output.clone();
                    Box::pin(async move {
                        let _ = output
                            .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
                            .await;
                    })
                });

                self.write_msg_tx
                    .send(WriterMessage::SendAndThen(data, callback))?;
            }
            _ => {
                warn!(
                    "on_recv_o2i_recv_data Socks5 error status: {:?}",
                    self.status
                );
            }
        }
        Ok(())
    }
    async fn on_recv_o2i_recv_data_from(
        &self,
        session_id: u32,
        mut data: Vec<u8>,
        peer_addr: String,
    ) -> anyhow::Result<()> {
        match &self.status {
            Status::RunWithUdp(udp_socket) => {
                let data_len = data.len();
                data = self.common_data.decode_data(data)?;

                let peer_addr = TargetAddr::Ip(peer_addr.parse()?);
                let addr_bytes = peer_addr.to_be_bytes()?;

                /*
                +----+------+------+----------+----------+----------+
                |RSV | FRAG | ATYP | DST.ADDR | DST.PORT |   DATA   |
                +----+------+------+----------+----------+----------+
                | 2  |  1   |  1   | Variable |    2     | Variable |
                +----+------+------+----------+----------+----------+
                */
                let mut response: Vec<u8> = Vec::with_capacity(addr_bytes.len() + data.len() + 5);
                response.extend_from_slice(&[0x0u8, 0x0u8, 0x0u8]);
                response.extend(addr_bytes);
                response.extend(data);

                udp_socket.send(&response).await?;

                let _ = self
                    .output
                    .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
                    .await;
            }
            _ => {
                warn!(
                    "on_recv_o2i_recv_data Socks5 error status: {:?}",
                    self.status
                );
            }
        }
        Ok(())
    }

    async fn on_recv_o2i_connect(&mut self, success: bool) -> anyhow::Result<()> {
        match self.status {
            Status::Connecting(is_tcp) if self.target_addr.is_some() => {
                let response: Vec<u8> = if success {
                    if is_tcp {
                        self.status = Status::RunWithTcp;
                        vec![
                            SOCKS5_VERSION,
                            0x00,
                            0x00,
                            0x01,
                            0x00,
                            0x00,
                            0x00,
                            0x00,
                            0x00,
                            0x00,
                        ]
                    } else {
                        if let Ok(data) = self.udp_bind().await {
                            data
                        } else {
                            vec![
                                SOCKS5_VERSION,
                                0x04, // 0x04主机不可达
                                0x00,
                                0x01,
                                0x00,
                                0x00,
                                0x00,
                                0x00,
                                0x00,
                                0x00,
                            ]
                        }
                    }
                } else {
                    vec![
                        SOCKS5_VERSION,
                        0x04, // 0x04主机不可达
                        0x00,
                        0x01,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                    ]
                };
                let _ = self.write_msg_tx.send(WriterMessage::Send(response, true));
            }
            _ => {}
        }
        Ok(())
    }

    async fn udp_bind(&mut self) -> anyhow::Result<Vec<u8>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        if let Some(ref target_addr) = self.target_addr {
            match target_addr {
                TargetAddr::Ip(addr) => {
                    socket.connect(addr).await?;
                }
                TargetAddr::Domain(host, port) => {
                    socket.connect(format!("{host}:{port}")).await?;
                }
            }
        }

        let mut buf = vec![SOCKS5_VERSION, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        buf.extend_from_slice(&socket.local_addr()?.port().to_be_bytes());

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let socket = Arc::new(socket);
        let socket_cloned = socket.clone();
        let output = self.output.clone();
        let write_msg_tx = self.write_msg_tx.clone();
        let session_id = self.session_id;
        let common_data = self.common_data.clone();

        let handle = tokio::spawn(async move {
            let task = async {
                let mut buf = [0; 65535]; // 最大允许的UDP数据包大小
                loop {
                    let result = socket.recv_from(&mut buf).await;
                    if result.is_err() {
                        continue;
                    }

                    let (amt, _addr) = result.unwrap();
                    if let Err(err) =
                        recv_udp_data(session_id, amt, &buf, &output, &common_data).await
                    {
                        warn!("Socks5 UDP error: {err}");
                        break;
                    }
                }
                let _ = write_msg_tx.send(WriterMessage::Close);
            };

            select! {
                _= task =>{},
                _= shutdown_rx =>{}
            }
        });

        self.udp_task_handle = Some((handle, shutdown_tx));
        self.status = Status::RunWithUdp(socket_cloned);

        Ok(buf)
    }
}

#[inline]
async fn recv_udp_data(
    session_id: u32,
    amt: usize,
    buf: &[u8],
    output: &Sender<ProxyMessage>,
    common_data: &SessionCommonInfo,
) -> anyhow::Result<()> {
    // +----+------+------+----------+----------+----------+
    // |RSV | FRAG | ATYP | DST.ADDR | DST.PORT |   DATA   |
    // +----+------+------+----------+----------+----------+
    // | 2  |  1   |  1   | Variable |    2     | Variable |
    // +----+------+------+----------+----------+----------+
    if amt < 11 {
        return Err(anyhow!("received data of illegal length"));
    }

    let received_data = Vec::from(&buf[..amt]);
    let address_type = received_data[3];
    match target_addr::read_address(&received_data[4..], address_type) {
        Ok(Some((addr, addr_data_len))) => {
            let start = 4 + addr_data_len;
            let data = common_data.decode_data(buf[start..amt].to_vec())?;

            output
                .send(ProxyMessage::I2oSendToData(
                    session_id,
                    data,
                    addr.to_string(),
                ))
                .await?;
        }
        _ => {
            return Err(anyhow!("address resolution failed"));
        }
    }

    Ok(())
}

#[inline]
async fn cancel_task(task_handle: Option<(JoinHandle<()>, oneshot::Sender<()>)>) {
    if let Some((task_handle, shutdown_tx)) = task_handle {
        if let Err(_) = timeout(Duration::from_secs(2), async {
            let _ = shutdown_tx.send(());
            let _ = task_handle.await;
        })
        .await
        {
            error!("The task is not completed within 2 seconds");
        }
    }
}
