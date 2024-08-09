use crate::net::session_delegate::SessionDelegate;
use crate::net::{tcp_server, udp_server};
use crate::net::{SendMessageFuncType, WriterMessage};
use crate::proxy::common::{InputSenderType, SessionCommonInfo};
use crate::proxy::socks5::Socks5Context;
use crate::proxy::{common, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use async_trait::async_trait;
use base64::prelude::*;
use log::{error, trace};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, RwLock};
use tokio::task::yield_now;

#[derive(Clone)]
pub enum InletProxyType {
    TCP,
    UDP,
    SOCKS5,
}

impl InletProxyType {
    pub fn from_u32(value: u32) -> Option<InletProxyType> {
        match value {
            0 => Some(InletProxyType::TCP),
            1 => Some(InletProxyType::UDP),
            2 => Some(InletProxyType::SOCKS5),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            InletProxyType::TCP => 0,
            InletProxyType::UDP => 1,
            InletProxyType::SOCKS5 => 2,
        }
    }

    pub fn is_socks5(&self) -> bool {
        match self {
            InletProxyType::SOCKS5 => true,
            _ => false,
        }
    }

    pub fn is_tcp(&self) -> bool {
        match self {
            InletProxyType::TCP => true,
            _ => false,
        }
    }
}

struct SessionInfo {
    proxy_message_tx: Option<mpsc::UnboundedSender<ProxyMessage>>,
    write_msg_tx: InputSenderType,
    common_info: SessionCommonInfo,
}

type SessionInfoMap = Arc<RwLock<HashMap<u32, SessionInfo>>>;

pub struct Inlet {
    is_running: Arc<AtomicBool>,
    input: Option<UnboundedSender<ProxyMessage>>,
    session_info_map: SessionInfoMap,
    description: String,
    on_output_callback: OutputFuncType,
}

pub struct InletDataEx {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl InletDataEx {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

impl Inlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            session_info_map: Arc::new(RwLock::new(HashMap::new())),
            input: None,
            description,
            on_output_callback,
        }
    }

    pub async fn start(
        &mut self,
        inlet_proxy_type: InletProxyType,
        listen_addr: String,
        output_addr: String,
        is_compressed: bool,
        encryption_method: String,
        data_ex: InletDataEx,
    ) -> anyhow::Result<()> {
        // 重复调用启动函数
        if self.running() {
            return Err(anyhow!("Repeated start"));
        }

        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (output_tx, output_rx) = mpsc::channel::<ProxyMessage>(1000);

        self.input = Some(input_tx);

        let session_info_map = self.session_info_map.clone();
        let output_tx_cloned = output_tx.clone();
        let inlet_proxy_type_cloned = inlet_proxy_type.clone();
        let data_ex = Arc::new(data_ex);

        let create_session_delegate_func = Box::new(move || -> Box<dyn SessionDelegate> {
            Box::new(InletSession::new(
                inlet_proxy_type.clone(),
                output_addr.clone(),
                session_info_map.clone(),
                is_compressed,
                encryption_method.clone(),
                output_tx.clone(),
                data_ex.clone(),
            ))
        });

        let on_output_callback = self.on_output_callback.clone();
        let session_info_map = self.session_info_map.clone();
        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::Relaxed);

        match inlet_proxy_type_cloned {
            InletProxyType::TCP | InletProxyType::SOCKS5 => {
                let listener = tcp_server::bind(&listen_addr).await?;

                tokio::spawn(async move {
                    let server_task = tcp_server::run_server(
                        listener,
                        create_session_delegate_func,
                        |stream: TcpStream| async move { Ok(stream) },
                        Self::async_receive_input(input_rx, output_tx_cloned, session_info_map),
                    );

                    select! {
                        _= server_task => {},
                        _= common::async_receive_output(output_rx, on_output_callback) => {}
                    }

                    is_running.store(false, Ordering::Relaxed);
                });
            }
            InletProxyType::UDP => {
                let socket = UdpSocket::bind(&listen_addr).await?;

                tokio::spawn(async move {
                    let server_task = udp_server::run_server(
                        socket,
                        create_session_delegate_func,
                        Self::async_receive_input(input_rx, output_tx_cloned, session_info_map),
                    );

                    select! {
                        _= server_task => {},
                        _= common::async_receive_output(output_rx, on_output_callback) => {}
                    }

                    is_running.store(false, Ordering::Relaxed);
                });
            }
        };

        Ok(())
    }

    pub async fn input(&self, proxy_message: ProxyMessage) {
        if let Some(sender) = &self.input {
            let _ = sender.send(proxy_message);
        }
    }

    pub fn running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub async fn stop(&mut self) {
        self.input.take();
        while self.running() {
            yield_now().await;
        }
        self.session_info_map.write().await.clear();
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    async fn async_receive_input(
        mut input: UnboundedReceiver<ProxyMessage>,
        output: Sender<ProxyMessage>,
        session_info_map: SessionInfoMap,
    ) {
        while let Some(message) = input.recv().await {
            if let Err(err) = Self::input_internal(message, &output, &session_info_map).await {
                error!("inlet async_receive_input error: {}", err.to_string());
            }
        }
    }

    async fn input_internal(
        message: ProxyMessage,
        output: &Sender<ProxyMessage>,
        session_info_map: &SessionInfoMap,
    ) -> anyhow::Result<()> {
        match message {
            ProxyMessage::O2iConnect(session_id, success, error_msg) => {
                trace!(
                    "O2iConnect: session_id:{session_id}, success:{success}, error_msg:{error_msg}"
                );

                if let Some(session) = session_info_map.read().await.get(&session_id) {
                    if let Some(ref proxy_message_tx) = session.proxy_message_tx {
                        proxy_message_tx
                            .send(ProxyMessage::O2iConnect(session_id, success, error_msg))?;
                    } else {
                        if !success {
                            error!("connect error: {error_msg}");
                            session.write_msg_tx.send(WriterMessage::Close)?;
                        }
                    }
                }
            }
            ProxyMessage::O2iDisconnect(session_id) => {
                trace!("O2iDisconnect: session_id:{session_id}");
                if let Some(session) = session_info_map.read().await.get(&session_id) {
                    session.write_msg_tx.send(WriterMessage::Close)?;
                }
            }
            ProxyMessage::O2iSendDataResult(session_id, data_len) => {
                // trace!("O2iSendDataResult: session_id:{session_id}, data_len:{data_len}");
                if let Some(session) = session_info_map.read().await.get(&session_id) {
                    let mut read_buf_len = session.common_info.read_buf_len.write().await;
                    if *read_buf_len <= data_len {
                        *read_buf_len = 0;
                    } else {
                        *read_buf_len = *read_buf_len - data_len;
                    }
                    // trace!("O2iSendDataResult: session_id:{session_id}, data_len:{data_len}, read_buf_len:{}", *read_buf_len);
                    drop(read_buf_len);
                }
            }
            ProxyMessage::O2iRecvDataFrom(session_id, data, remote_addr) => {
                if let Some(session) = session_info_map.read().await.get(&session_id) {
                    if let Some(ref proxy_message_tx) = session.proxy_message_tx {
                        proxy_message_tx.send(ProxyMessage::O2iRecvDataFrom(
                            session_id,
                            data,
                            remote_addr,
                        ))?;
                    } else {
                        panic!("Faulty logic");
                    }
                }
            }
            ProxyMessage::O2iRecvData(session_id, mut data) => {
                // trace!("O2iRecvData: session_id:{session_id}");
                if let Some(session) = session_info_map.read().await.get(&session_id) {
                    if let Some(ref proxy_message_tx) = session.proxy_message_tx {
                        proxy_message_tx.send(ProxyMessage::O2iRecvData(
                            session_id,
                            data,
                        ))?;
                    } else {
                        let data_len = data.len();
                        data = session.common_info.decode_data(data)?;

                        // 写入完毕回调
                        let output = output.clone();
                        let callback: SendMessageFuncType = Box::new(move || {
                            let output = output.clone();
                            Box::pin(async move {
                                let _ = output
                                    .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
                                    .await;
                            })
                        });

                        session
                            .write_msg_tx
                            .send(WriterMessage::SendAndThen(data, callback))?;
                    }
                } else {
                    trace!("O2iRecvData: unknown session:{session_id}");
                }
            }
            _ => {
                return Err(anyhow!("Unknown message"));
            }
        }

        Ok(())
    }
}

struct InletSession {
    inlet_proxy_type: InletProxyType,
    output_addr: String,
    session_info_map: SessionInfoMap,
    session_id: u32,
    output: Sender<ProxyMessage>,
    common_data: SessionCommonInfo,
    socks5context: Option<Arc<RwLock<Socks5Context>>>,
    data_ex: Arc<InletDataEx>,
}

impl InletSession {
    pub fn new(
        inlet_proxy_type: InletProxyType,
        output_addr: String,
        session_info_map: SessionInfoMap,
        is_compressed: bool,
        encryption_method: String,
        output: Sender<ProxyMessage>,
        data_ex: Arc<InletDataEx>,
    ) -> Self {
        Self {
            inlet_proxy_type,
            output_addr,
            session_info_map,
            session_id: 0,
            output,
            common_data: SessionCommonInfo::from_method_name(is_compressed, encryption_method),
            socks5context: None,
            data_ex,
        }
    }
}

#[async_trait]
impl SessionDelegate for InletSession {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        addr: &SocketAddr,
        write_msg_tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        trace!("inlet on session({session_id}) start {addr}");

        self.session_id = session_id;

        if self.inlet_proxy_type.is_socks5() {
            let (socks5context, proxy_message_tx) = Socks5Context::new(
                write_msg_tx.clone(),
                self.output.clone(),
                self.session_id,
                addr.clone(),
                self.data_ex.clone(),
                self.common_data.clone(),
            )
            .await;

            self.socks5context = Some(socks5context);

            self.session_info_map.write().await.insert(
                session_id,
                SessionInfo {
                    proxy_message_tx: Some(proxy_message_tx),
                    write_msg_tx,
                    common_info: self.common_data.clone(),
                },
            );
        } else {
            self.session_info_map.write().await.insert(
                session_id,
                SessionInfo {
                    proxy_message_tx: None,
                    write_msg_tx,
                    common_info: self.common_data.clone(),
                },
            );

            self.output
                .send(ProxyMessage::I2oConnect(
                    session_id,
                    self.inlet_proxy_type.to_u8(),
                    self.inlet_proxy_type.is_tcp(),
                    self.common_data.is_compressed,
                    self.output_addr.clone(),
                    self.common_data.encryption_method.to_string(),
                    BASE64_STANDARD.encode(&self.common_data.encryption_key),
                    addr.to_string(),
                ))
                .await?;
        }

        Ok(())
    }

    async fn on_session_close(&mut self) -> anyhow::Result<()> {
        trace!("inlet on session({}) close", self.session_id);
        self.session_info_map.write().await.remove(&self.session_id);
        self.output
            .send(ProxyMessage::I2oDisconnect(self.session_id))
            .await?;
        if let Some(context) = self.socks5context.take() {
            context.write().await.on_destroy().await;
        }
        Ok(())
    }

    async fn on_recv_frame(&mut self, mut frame: Vec<u8>) -> anyhow::Result<()> {
        if let Some(ref context) = self.socks5context {
            context.write().await.recv_frame(frame).await?;
            return Ok(());
        }

        frame = self.common_data.encode_data_and_limiting(frame).await?;
        self.output
            .send(ProxyMessage::I2oSendData(self.session_id, frame))
            .await?;
        Ok(())
    }
}
