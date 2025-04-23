use crate::CommonArgs;
use anyhow::anyhow;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use bytes::BytesMut;
use log::{debug, error, info};
use np_base::net::tls;
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
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout, Instant};
use tokio_kcp::{KcpConfig, KcpNoDelayConfig, KcpStream};
use tokio_rustls::rustls::client::ServerCertVerified;
use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName};
use tokio_rustls::{rustls, TlsConnector};
use webpki_roots::TLS_SERVER_ROOTS;

const TIMEOUT_TLS: u64 = 30;

struct Client<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    writer: Arc<Mutex<WriteHalf<S>>>,
    username: String,
    password: String,
    player_id: u32,
    outlets: Arc<RwLock<HashMap<u32, Arc<Outlet>>>>,
    inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
    tunnels: HashMap<u32, Tunnel>,
}

struct NoCertificateVerifier;

impl rustls::client::ServerCertVerifier for NoCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

async fn connect_with_tcp(addr: &str) -> anyhow::Result<TcpStream> {
    let stream = TcpStream::connect(&addr).await?;
    info!("TCP successfully connected to serve {}", addr);

    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;
    Ok(stream)
}

async fn connect_with_kcp(addr: &str) -> anyhow::Result<KcpStream> {
    let addrs = tokio::net::lookup_host(addr).await?;

    let mut last_err = None;

    for addr in addrs {
        match KcpStream::connect(
            &KcpConfig {
                mtu: 1400,
                nodelay: KcpNoDelayConfig::fastest(),
                wnd_size: (1024, 1024),
                session_expire: Some(Duration::from_secs(60)),
                flush_write: false,
                flush_acks_input: false,
                stream: true,
                allow_recv_empty_packet: false,
            },
            addr,
        )
        .await
        {
            Ok(stream) => {
                info!("KCP successfully connected to serve {}", addr);
                return Ok(stream);
            }
            Err(e) => last_err = Some(e),
        }
    }

    if let Some(err) = last_err {
        Err(anyhow!(err))
    } else {
        Err(anyhow!("could not resolve to any address"))
    }
}

pub async fn run(common_args: &CommonArgs, use_tcp: bool) -> anyhow::Result<()> {
    info!("Start connecting to server {}", common_args.server);

    // 升级为TLS连接
    if common_args.enable_tls {
        let mut root_cert_store = RootCertStore::empty();

        // 加载系统默认的根证书
        let trust_anchors = TLS_SERVER_ROOTS.0.iter().map(|ta| {
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        });
        root_cert_store.add_server_trust_anchors(trust_anchors);

        // 加载自签证书
        if !common_args.ca_cert.is_empty() {
            let ca_certs = tls::load_certs(&common_args.ca_cert)?;
            anyhow::ensure!(
                !ca_certs.is_empty(),
                "invalid cert file: {}",
                common_args.ca_cert
            );
            root_cert_store.add(&ca_certs[0])?;
        }

        // 创建TLS配置
        let mut config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth(); // 不需要客户端认证

        if common_args.insecure {
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoCertificateVerifier {}));
        }

        let str_vec: Vec<&str> = common_args.server.split(":").collect();
        if str_vec.is_empty() {
            return Err(anyhow!("invalid addr: {}", common_args.server));
        }
        let connector = TlsConnector::from(Arc::new(config));

        if use_tcp {
            let stream = timeout(
                Duration::from_secs(TIMEOUT_TLS),
                connector.connect(
                    ServerName::try_from(str_vec[0])?,
                    connect_with_tcp(&common_args.server).await?,
                ),
            )
            .await??;

            run_client(common_args, stream).await
        } else {
            let kcp_stream = connect_with_kcp(&common_args.server).await?;
            let stream = timeout(
                Duration::from_secs(TIMEOUT_TLS),
                connector.connect(ServerName::try_from(str_vec[0])?, kcp_stream),
            )
            .await??;

            run_client(common_args, stream).await
        }
    } else {
        if use_tcp {
            run_client(common_args, connect_with_tcp(&common_args.server).await?).await
        } else {
            run_client(common_args, connect_with_kcp(&common_args.server).await?).await
        }
    }
}

async fn run_client<S>(common_args: &CommonArgs, stream: S) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);

    let writer = Arc::new(Mutex::new(writer));

    let mut client = Client::<S> {
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

async fn ping_forever<S>(
    writer: Arc<Mutex<WriteHalf<S>>>,
    last_active_time: Arc<RwLock<Instant>>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    const PING_INTERVAL: Duration = Duration::from_secs(5);
    const PING_TIMEOUT: Duration = Duration::from_secs(15);
    loop {
        sleep(Duration::from_secs(1)).await;

        if last_active_time.read().await.elapsed() < PING_INTERVAL {
            continue;
        }

        if last_active_time.read().await.elapsed() > PING_TIMEOUT {
            return Err(anyhow!("ping timeout"));
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

impl<S> Client<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    async fn run(
        &mut self,
        mut reader: ReadHalf<S>,
        last_active_time: Arc<RwLock<Instant>>,
    ) -> anyhow::Result<()> {
        const WRITE_TIMEOUT: Duration = Duration::from_secs(1);
        let mut buffer = BytesMut::with_capacity(65536);
        loop {
            let len = reader.read_buf(&mut buffer).await?;
            // len为0表示对端已经关闭连接。
            if len == 0 {
                info!("Disconnect from the server");
                break;
            } else {
                if last_active_time.read().await.elapsed() >= WRITE_TIMEOUT {
                    let mut instant_write = last_active_time.write().await;
                    *instant_write = Instant::now();
                }

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
            if let Some(outlet) = self.outlets.write().await.remove(&key) {
                let description = outlet.description().to_owned();
                debug!("start deleting the outlet({description})");
                outlet.stop().await;
                debug!("delete outlet({description}) end");
            }
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
            if let Some(mut inlet) = self.inlets.write().await.remove(&key) {
                let description = inlet.description().to_owned();
                debug!("start deleting the inlet({description})");
                inlet.stop().await;
                debug!("delete inlet({description}) end");
            }
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
                debug!("start outlet({})", outlet_description(&tunnel));
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

                let inlet_proxy_type = InletProxyType::from_u32(tunnel.tunnel_type as u32);
                if matches!(inlet_proxy_type, InletProxyType::UNKNOWN) {
                    error!(
                        "inlet({}) unknown tunnel type: {}",
                        source, tunnel.tunnel_type
                    );
                } else {
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
                        debug!("start inlet({})", inlet.description());
                        self.inlets.write().await.insert(tunnel.id, inlet);
                    }
                }
            }
        }
    }

    async fn send_proxy_message(
        outlets: Arc<RwLock<HashMap<u32, Arc<Outlet>>>>,
        inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
        writer: Arc<Mutex<WriteHalf<S>>>,
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
pub(crate) async fn package_and_send_message<S>(
    writer: Arc<Mutex<WriteHalf<S>>>,
    serial: i32,
    message: &MessageType,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
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
