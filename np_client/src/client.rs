use crate::CommonArgs;
use anyhow::anyhow;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use bytes::BytesMut;
use log::{debug, error, info};
use np_base::proxy::inlet::{Inlet, InletDataEx, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::class_def::{Tunnel, TunnelPoint};
use np_proto::client_server::LoginReq;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use np_proto::server_client::ModifyTunnelNtf;
use np_proto::utils::message_bridge;
use np_proto::{generic, message_map};
use socket2::{SockRef, TcpKeepalive};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Instant};

type WriterType = Arc<Mutex<WriteHalf<TcpStream>>>;

struct Client {
    writer: WriterType,
    username: String,
    password: String,
    player_id: u32,
    outlets: Arc<RwLock<HashMap<u32, Arc<Outlet>>>>,
    inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
    tunnels: HashMap<u32, Tunnel>,
}

pub async fn run(common_args: &CommonArgs) -> anyhow::Result<()> {
    info!("Start connecting to server {}", common_args.server);
    let stream = TcpStream::connect(&common_args.server).await?;
    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;
    info!("Successful connection with server {}", common_args.server);

    let (reader, writer) = tokio::io::split(stream);

    let writer = Arc::new(Mutex::new(writer));

    let mut client = Client {
        writer: writer.clone(),
        username: common_args.username.clone(),
        password: common_args.password.clone(),
        player_id: 0u32,
        outlets: Arc::new(RwLock::new(HashMap::new())),
        inlets: Arc::new(RwLock::new(HashMap::new())),
        tunnels: HashMap::new(),
    };

    client.send_login().await?;

    let last_active_time = Arc::new(RwLock::new(Instant::now()));

    let result;
    select! {
        r1= client.run(reader, last_active_time.clone()) => { result = r1 },
        r2= ping_forever(writer, last_active_time.clone()) => { result = r2 },
    }
    client.sync_tunnels(&Vec::new()).await;
    result
}

async fn ping_forever(
    writer: WriterType,
    last_active_time: Arc<RwLock<Instant>>,
) -> anyhow::Result<()> {
    const PING_INTERVAL: Duration = Duration::from_secs(5);
    loop {
        sleep(Duration::from_secs(1)).await;

        if last_active_time.read().await.elapsed() < PING_INTERVAL {
            continue;
        }

        // 获取当前时间
        let now = SystemTime::now();

        // 计算自UNIX_EPOCH以来的持续时间
        let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

        // 将时间转换为毫秒
        let nanos = since_epoch.as_millis();

        package_and_send_message(
            writer.clone(),
            -2,
            &MessageType::GenericPing(generic::Ping {
                ticks: nanos as i64,
            }),
        )
        .await?;
    }
}

impl Client {
    async fn run(
        &mut self,
        mut reader: ReadHalf<TcpStream>,
        last_active_time: Arc<RwLock<Instant>>,
    ) -> anyhow::Result<()> {
        const WRITE_TIMEOUT: Duration = Duration::from_secs(1);
        let mut buffer = BytesMut::with_capacity(65536);
        loop {
            let len = reader.read_buf(&mut buffer).await?;

            if last_active_time.read().await.elapsed() >= WRITE_TIMEOUT {
                let mut instant_write = last_active_time.write().await;
                *instant_write = Instant::now();
            }

            // len为0表示对端已经关闭连接。
            if len == 0 {
                info!("Disconnect from the server");
                break;
            } else {
                // 循环解包
                loop {
                    if buffer.is_empty() {
                        break;
                    }

                    let result = try_extract_frame(&mut buffer)?;
                    if let Some(frame) = result {
                        // 收到完整消息
                        self.on_recv_frame(frame).await?;
                    } else {
                        // 消息包接收还未完成
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_login(&self) -> anyhow::Result<()> {
        info!("Start Login");
        package_and_send_message(
            self.writer.clone(),
            -1,
            &MessageType::ClientServerLoginReq(LoginReq {
                version: "0.0.0".to_string(),
                username: self.username.clone(),
                password: self.password.clone(),
            }),
        )
        .await
    }

    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
        if frame.len() < 8 {
            return Err(anyhow!("message length is too small"));
        }
        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

        let message = message_map::decode_message(msg_id, &frame[8..])?;
        self.handle_message(serial, message).await
    }

    async fn handle_message(&mut self, serial: i32, message: MessageType) -> anyhow::Result<()> {
        if self.player_id == 0 {
            if serial > 0 {
                return match message {
                    MessageType::ServerClientLoginAck(msg) => {
                        info!("Login successful");
                        self.player_id = msg.player_id;
                        self.sync_tunnels(&msg.tunnel_list).await;
                        self.tunnels = msg
                            .tunnel_list
                            .into_iter()
                            .map(|x| (x.id, x))
                            .collect::<HashMap<u32, Tunnel>>();
                        Ok(())
                    }
                    MessageType::GenericError(err) => Err(anyhow!(
                        "Login failed: {}, code: {}",
                        err.message,
                        err.number
                    )),
                    _ => Err(anyhow!("Login failed, received unknown message")),
                };
            }
            return Err(anyhow!("Login failed"));
        } else {
            if serial == 0 {
                self.handle_push(message).await?;
            }
        }
        Ok(())
    }

    async fn sync_tunnels(&mut self, tunnels: &Vec<Tunnel>) {
        // 收集无效的出口
        let mut keys_to_remove: Vec<_> = self
            .outlets
            .read()
            .await
            .iter()
            .filter(|(id, outlet)| {
                let retain = tunnels.iter().any(|tunnel| {
                    **id == tunnel.id
                        && tunnel.enabled
                        && tunnel.sender == self.player_id
                        && &outlet_description(&tunnel) == outlet.description()
                });
                !retain
            })
            .map(|(key, _)| key.clone())
            .collect();

        // 删除无效的出口
        for key in keys_to_remove {
            info!("start deleting the outlet({key})");
            if let Some(outlet) = self.outlets.write().await.remove(&key) {
                outlet.stop().await;
            }
            info!("delete outlet({key}) end");
        }

        // 收集无效的入口
        keys_to_remove = self
            .inlets
            .read()
            .await
            .iter()
            .filter(|(id, inlet)| {
                let retain = tunnels.iter().any(|tunnel| {
                    **id == tunnel.id
                        && tunnel.enabled
                        && tunnel.receiver == self.player_id
                        && &inlet_description(&tunnel) == inlet.description()
                });
                return !retain;
            })
            .map(|(key, _)| key.clone())
            .collect();

        // 删除无效入口
        for key in keys_to_remove {
            info!("start deleting the inlet({key})");
            if let Some(mut inlet) = self.inlets.write().await.remove(&key) {
                inlet.stop().await;
            }
            info!("delete inlet({key}) end");
        }

        // 添加代理出口
        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled && tunnel.sender == self.player_id)
        {
            if !self.outlets.read().await.contains_key(&tunnel.id) {
                let this_machine = tunnel.receiver == tunnel.sender;
                let inlets = self.inlets.clone();
                let outlets = self.outlets.clone();
                let writer = self.writer.clone();
                let self_player_id = self.player_id;
                let tunnel_id = tunnel.id;
                let player_id = tunnel.receiver;

                let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    let outlets = outlets.clone();
                    let writer = writer.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(inlet) = inlets.read().await.get(&tunnel_id) {
                                inlet.input(message).await;
                            } else {
                                debug!("unknown inlet({tunnel_id})");
                            }
                        } else {
                            Self::send_proxy_message(
                                outlets,
                                inlets,
                                writer,
                                self_player_id,
                                player_id,
                                tunnel_id,
                                message,
                            )
                            .await;
                        }
                    })
                });
                self.outlets.write().await.insert(
                    tunnel_id,
                    Outlet::new(outlet_output, outlet_description(&tunnel)),
                );
            }
        }

        // 添加代理入口
        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled && tunnel.receiver == self.player_id)
        {
            if !self.inlets.read().await.contains_key(&tunnel.id) {
                let this_machine = tunnel.receiver == tunnel.sender;
                let tunnel_id = tunnel.id;
                let inlets = self.inlets.clone();
                let outlets = self.outlets.clone();
                let writer = self.writer.clone();
                let self_player_id = self.player_id;
                let player_id = tunnel.sender;

                let source = match tunnel.source {
                    Some(ref x) => x.addr.clone(),
                    None => "".to_string(),
                };
                let endpoint = match tunnel.endpoint {
                    Some(ref x) => x.addr.clone(),
                    None => "".to_string(),
                };

                let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    let outlets = outlets.clone();
                    let writer = writer.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(outlet) = outlets.read().await.get(&tunnel_id) {
                                outlet.input(message).await;
                            } else {
                                debug!("unknown outlet({tunnel_id})");
                            }
                        } else {
                            Self::send_proxy_message(
                                outlets,
                                inlets,
                                writer,
                                self_player_id,
                                player_id,
                                tunnel_id,
                                message,
                            )
                            .await;
                        }
                    })
                });

                if let Some(inlet_proxy_type) = InletProxyType::from_u32(tunnel.tunnel_type as u32)
                {
                    let mut inlet = Inlet::new(inlet_output, inlet_description(&tunnel));
                    if let Err(err) = inlet
                        .start(
                            inlet_proxy_type,
                            source.clone(),
                            endpoint.clone(),
                            tunnel.is_compressed,
                            tunnel.encryption_method.clone(),
                            InletDataEx::new(tunnel.username.clone(), tunnel.password.clone()),
                        )
                        .await
                    {
                        error!("inlet({}) start error: {}", source, err);
                    } else {
                        self.inlets.write().await.insert(tunnel.id, inlet);
                    }
                } else {
                    error!(
                        "inlet({}) unknown tunnel type: {}",
                        source, tunnel.tunnel_type
                    );
                }
            }
        }
    }

    async fn send_proxy_message(
        outlets: Arc<RwLock<HashMap<u32, Arc<Outlet>>>>,
        inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
        writer: WriterType,
        self_player_id: u32,
        player_id: u32,
        tunnel_id: u32,
        proxy_message: ProxyMessage,
    ) {
        if self_player_id == player_id {
            if message_bridge::is_i2o_message(&proxy_message) {
                if let Some(outlet) = outlets.read().await.get(&tunnel_id) {
                    outlet.input(proxy_message).await;
                }
            } else {
                if let Some(inlet) = inlets.read().await.get(&tunnel_id) {
                    inlet.input(proxy_message).await;
                }
            }
        } else {
            let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
            if !message.is_none() {
                let _ = package_and_send_message(writer, 0, &message).await;
            }
        }
    }

    // 收到玩家向服务器推送消息
    pub(crate) async fn handle_push(&mut self, message: MessageType) -> anyhow::Result<()> {
        match message {
            MessageType::ServerClientModifyTunnelNtf(msg) => {
                self.on_server_client_modify_tunnel_ntf(msg).await
            }
            _ => {
                if let Some((msg, tunnel_id)) = message_bridge::pb_2_proxy_message(message) {
                    if let Some(tunnel) = self.tunnels.get(&tunnel_id) {
                        let player_id = if message_bridge::is_i2o_message(&msg) {
                            tunnel.sender
                        } else {
                            tunnel.receiver
                        };

                        Self::send_proxy_message(
                            self.outlets.clone(),
                            self.inlets.clone(),
                            self.writer.clone(),
                            self.player_id,
                            player_id,
                            tunnel.id,
                            msg,
                        )
                        .await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_server_client_modify_tunnel_ntf(&mut self, msg: ModifyTunnelNtf) {
        if let Some(tunnel) = msg.tunnel {
            self.tunnels.remove(&tunnel.id);
            if msg.is_delete {
                return;
            }
            self.tunnels.insert(tunnel.id, tunnel);
            let tunnel_list: Vec<Tunnel> =
                self.tunnels.clone().into_iter().map(|(_, x)| x).collect();
            self.sync_tunnels(&tunnel_list).await;
        }
    }
}

#[inline]
pub(crate) async fn package_and_send_message(
    writer: WriterType,
    serial: i32,
    message: &MessageType,
) -> anyhow::Result<()> {
    if let Some(message_id) = get_message_id(message) {
        let message_size = get_message_size(message);
        let mut buf = Vec::with_capacity(message_size + 14);

        byteorder::WriteBytesExt::write_u8(&mut buf, 33u8)?;
        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
        byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
        encode_raw_message(message, &mut buf);

        writer.lock().await.write_all(&buf).await?;
        Ok(())
    } else {
        Err(anyhow!("Message id not found"))
    }
}

/// 数据粘包处理
///
/// 注意：这个函数只能使用消耗 buffer 数据的函数，否则框架会一直循环调用本函数来驱动处理消息
///
fn try_extract_frame(buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
    if buffer.len() > 0 {
        if buffer[0] != 33u8 {
            return Err(anyhow!("Bad flag"));
        }
    }
    // 数据小于5字节,继续读取数据
    if buffer.len() < 5 {
        return Ok(None);
    }

    // 读取包长度
    let buf = buffer.get(1..5).unwrap();
    let len = BigEndian::read_u32(buf) as usize;

    // 超出最大限制
    if len <= 0 || len >= 1024 * 1024 * 5 {
        return Err(anyhow!("Message too long"));
    }

    // 数据不够,继续读取数据
    if buffer.len() < 5 + len {
        return Ok(None);
    }

    // 拆出这个包的数据
    let frame = buffer.split_to(5 + len).split_off(5).to_vec();

    Ok(Some(frame))
}

fn fmt_point(point: &Option<TunnelPoint>) -> String {
    match point {
        Some(point) => {
            format!("{}", point.addr)
        }
        _ => "none".to_string(),
    }
}

fn outlet_description(tunnel: &Tunnel) -> String {
    format!(
        "id:{}-sender:{}-enabled:{}",
        tunnel.id, tunnel.sender, tunnel.enabled
    )
}

fn inlet_description(tunnel: &Tunnel) -> String {
    let custom_mapping: String = tunnel
        .custom_mapping
        .iter()
        .map(|(key, value)| format!("{}:{}\n", key, value))
        .collect();
    format!(
        "id:{}-source:{}-endpoint:{}-sender:{}-receiver:{}-tunnel_type:{}-username:{}-password:{}-enabled:{}-is_compressed:{}-encryption_method:{}-custom_mapping:[{}]",
        tunnel.id,
        fmt_point(&tunnel.source),
        fmt_point(&tunnel.endpoint),
        tunnel.sender,
        tunnel.receiver,
        tunnel.tunnel_type,
        tunnel.username,
        tunnel.password,
        tunnel.enabled,
        tunnel.is_compressed,
        tunnel.encryption_method,
        custom_mapping,
    )
}
