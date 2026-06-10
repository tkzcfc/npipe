//! 客户端会话：登录、事件循环、隧道同步与代理消息路由。

use anyhow::anyhow;
use byteorder::{BigEndian, ByteOrder};
use dashmap::DashMap;
use log::{debug, error, info, warn};
use np_base::proxy::inlet::{Inlet, InletDataEx, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::class_def::{Tunnel, TunnelPoint};
use np_proto::client_server::LoginReq;
use np_proto::message_map::{self, MessageType};
use np_proto::server_client::ModifyTunnelNtf;
use np_proto::utils::message_bridge;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf};
use tokio::select;
use tokio::sync::mpsc;

use super::io::ping_forever;
use super::transport::{ClientTransport, IncomingFrame, TransportEvent};

/// 客户端会话状态：登录、隧道同步与代理消息路由。
///
/// 传输层通过 `ClientTransport<S>` 注入，使会话逻辑可测试，
/// 且不依赖具体协议（TCP/KCP/WS/QUIC）。
pub struct ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub transport: ClientTransport<S>,
    /// 登录用户名。
    pub username: String,
    /// 登录密码。
    pub password: String,
    /// 客户端期望的最大转发连接数（上报至 LoginReq）。
    pub transport_max_connections: u32,
    /// 客户端期望的转发连接空闲超时秒数（上报至 LoginReq）。
    pub transport_idle_timeout_secs: u32,
    /// 登录成功后服务端分配的用户 ID，0 表示未登录。
    pub player_id: u32,
    /// 当前启动的代理出口集合，key 为隧道 ID。
    pub outlets: Arc<DashMap<u32, Arc<Outlet>>>,
    /// 当前启动的代理入口集合，key 为隧道 ID。
    pub inlets: Arc<DashMap<u32, Inlet>>,
    /// 服务端下发的隧道快照，key 为隧道 ID。
    pub tunnels: HashMap<u32, Tunnel>,
}

/// 登录超时时间（秒）。
const LOGIN_TIMEOUT_SECS: u64 = 30;

impl<S> ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 运行完整的客户端生命周期：登录 → 预热 → 事件循环 + 心跳。
    ///
    /// 先启动控制连接读取任务，使登录期间就能接收事件。
    /// 登录成功后预热转发路径，再进入主循环。
    pub async fn run(
        &mut self,
        reader: ReadHalf<S>,
        event_tx: mpsc::UnboundedSender<TransportEvent>,
        mut event_rx: mpsc::UnboundedReceiver<TransportEvent>,
        last_active_secs: Arc<AtomicU64>,
        last_read_secs: Arc<AtomicU64>,
    ) -> anyhow::Result<()> {
        // 1. 启动控制连接读取任务
        let read_handle = tokio::spawn(super::io::read_transport_events(
            reader,
            None,
            event_tx,
            last_active_secs.clone(),
            last_read_secs.clone(),
        ));

        // 2. 发送登录并等待回复
        self.login_with_timeout(&mut event_rx).await?;

        // 3. 预热转发路径
        self.transport.warm_up().await;

        // 4. 主事件循环 + 心跳
        let transport = self.transport.clone();
        let result;
        select! {
            r1 = self.event_loop(&mut event_rx) => { result = r1 },
            r2 = ping_forever(transport, last_active_secs, last_read_secs) => { result = r2 },
            r3 = read_handle => {
                // 读取任务退出 = 控制连接断开
                result = match r3 {
                    Ok(inner) => inner,
                    Err(e) => Err(anyhow!("control connection read task panicked: {}", e)),
                };
            },
        }

        // 5. 清理隧道
        self.sync_tunnels(&[]).await;
        result
    }

    /// 发送登录请求并在超时时间内等待服务端回复。
    async fn login_with_timeout(
        &mut self,
        event_rx: &mut mpsc::UnboundedReceiver<TransportEvent>,
    ) -> anyhow::Result<()> {
        info!("sending login request");
        self.transport
            .send_control_message(
                -1,
                &MessageType::ClientServerLoginReq(LoginReq {
                    version: "0.0.0".to_string(),
                    username: self.username.clone(),
                    password: self.password.clone(),
                    transport_max_connections: self.transport_max_connections,
                    transport_idle_timeout_secs: self.transport_idle_timeout_secs,
                }),
            )
            .await?;

        // 等待登录响应
        let timeout_duration = Duration::from_secs(LOGIN_TIMEOUT_SECS);
        loop {
            let event = tokio::time::timeout(timeout_duration, event_rx.recv())
                .await
                .map_err(|_| anyhow!("login timeout: no reply within {}s", LOGIN_TIMEOUT_SECS))?
                .ok_or_else(|| anyhow!("control connection closed during login"))?;

            match event {
                TransportEvent::Frame(incoming) => {
                    if incoming.path_id.is_some() {
                        // 登录期间收到转发路径帧——不应发生，跳过
                        debug!("ignoring forward path frame during login");
                        continue;
                    }
                    return self.handle_login_frame(incoming).await;
                }
                TransportEvent::Closed { path_id, reason } => {
                    if path_id.is_none() {
                        return Err(anyhow!(
                            "control connection closed during login: {}",
                            reason
                        ));
                    }
                    // 转发路径在登录期间关闭——忽略
                    debug!("forward path closed during login: {}", reason);
                }
            }
        }
    }

    /// 解析登录响应帧，成功则配置传输层。
    async fn handle_login_frame(&mut self, incoming: IncomingFrame) -> anyhow::Result<()> {
        let frame = incoming.frame;
        if frame.len() < 8 {
            return Err(anyhow!("login response frame too short"));
        }

        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);
        let message = message_map::decode_message(msg_id, &frame[8..])?;

        if serial <= 0 {
            return Err(anyhow!("login failed: unexpected serial={}", serial));
        }

        match message {
            MessageType::ServerClientLoginAck(msg) => {
                info!("login successful, player_id={}", msg.player_id);
                info!(
                    "transport negotiated: max_forward_paths={}, idle_timeout_secs={}",
                    msg.transport_max_connections, msg.transport_idle_timeout_secs
                );
                self.player_id = msg.player_id;
                self.transport
                    .configure_from_login(
                        msg.transport_token.clone(),
                        msg.transport_max_connections,
                        msg.transport_idle_timeout_secs,
                    )
                    .await;
                self.sync_tunnels(&msg.tunnel_list).await;
                self.tunnels = msg.tunnel_list.into_iter().map(|t| (t.id, t)).collect();
                Ok(())
            }
            MessageType::GenericError(err) => Err(anyhow!(
                "login failed: {} (code={})",
                err.message,
                err.number
            )),
            _ => Err(anyhow!("login failed: unexpected message type")),
        }
    }

    /// 主事件循环：分发接收到的帧，处理路径关闭事件。
    async fn event_loop(
        &mut self,
        event_rx: &mut mpsc::UnboundedReceiver<TransportEvent>,
    ) -> anyhow::Result<()> {
        while let Some(event) = event_rx.recv().await {
            match event {
                TransportEvent::Frame(incoming) => {
                    self.on_recv_frame(incoming).await?;
                }
                TransportEvent::Closed { path_id, reason } => {
                    if path_id.is_none() {
                        info!("disconnected from server: {}", reason);
                        break;
                    }
                    debug!(
                        "forward path closed: path_id={:?}, reason={}",
                        path_id, reason
                    );
                    self.transport.remove_path(path_id).await;
                }
            }
        }
        Ok(())
    }

    async fn on_recv_frame(&mut self, incoming: IncomingFrame) -> anyhow::Result<()> {
        let frame = incoming.frame;
        if frame.len() < 8 {
            return Err(anyhow!("message frame too short ({} bytes)", frame.len()));
        }

        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);
        let message = message_map::decode_message(msg_id, &frame[8..])?;

        self.transport
            .bind_incoming_message_path(&message, incoming.path_id);
        self.handle_message(serial, message, incoming.path_id).await
    }

    async fn handle_message(
        &mut self,
        serial: i32,
        message: MessageType,
        path_id: Option<u64>,
    ) -> anyhow::Result<()> {
        if serial > 0 {
            // 我方请求的响应
            match message {
                MessageType::ServerClientBindTransportAck(msg) => {
                    debug!(
                        "transport bound: player_id={}, connection_id={}",
                        msg.player_id, msg.connection_id
                    );
                }
                MessageType::GenericError(err) => {
                    warn!(
                        "transport response error: {} (code={})",
                        err.message, err.number
                    );
                    self.transport.remove_path(path_id).await;
                }
                _ => {}
            }
        } else if serial == 0 {
            // 服务端推送
            self.handle_push(message).await?;
        }
        Ok(())
    }

    /// 处理服务端推送消息。
    async fn handle_push(&mut self, message: MessageType) -> anyhow::Result<()> {
        match message {
            MessageType::ServerClientDisconnectNtf(msg) => {
                return Err(anyhow!("server disconnected: {}", msg.reason));
            }
            MessageType::ServerClientModifyTunnelNtf(msg) => {
                self.on_modify_tunnel_ntf(msg).await;
            }
            _ => {
                if let Some((proxy_msg, tunnel_id)) = message_bridge::pb_2_proxy_message(message) {
                    if let Some(tunnel) = self.tunnels.get(&tunnel_id) {
                        let player_id = if message_bridge::is_i2o_message(&proxy_msg) {
                            tunnel.sender
                        } else {
                            tunnel.receiver
                        };
                        Self::route_proxy_message(
                            self.outlets.clone(),
                            self.inlets.clone(),
                            self.transport.clone(),
                            self.player_id,
                            player_id,
                            tunnel_id,
                            proxy_msg,
                        )
                        .await;
                    }
                }
            }
        }
        Ok(())
    }

    /// 处理隧道变更通知。
    async fn on_modify_tunnel_ntf(&mut self, msg: ModifyTunnelNtf) {
        if let Some(tunnel) = msg.tunnel {
            self.tunnels.remove(&tunnel.id);
            if msg.is_delete {
                let tunnel_list: Vec<Tunnel> = self.tunnels.values().cloned().collect();
                self.sync_tunnels(&tunnel_list).await;
                return;
            }
            self.tunnels.insert(tunnel.id, tunnel);
            let tunnel_list: Vec<Tunnel> = self.tunnels.values().cloned().collect();
            self.sync_tunnels(&tunnel_list).await;
        }
    }

    // ─── 隧道同步 ──────────────────────────────────────────────────────────────

    /// 根据隧道列表同步入口和出口。
    pub async fn sync_tunnels(&mut self, tunnels: &[Tunnel]) {
        // 收集无效的出口
        let keys_to_remove: Vec<u32> = self
            .outlets
            .iter()
            .filter(|entry| {
                let id = *entry.key();
                let outlet = entry.value();
                !tunnels.iter().any(|tunnel| {
                    id == tunnel.id
                        && tunnel.enabled
                        && tunnel.sender == self.player_id
                        && &outlet_description(tunnel) == outlet.description()
                })
            })
            .map(|entry| *entry.key())
            .collect();

        for key in keys_to_remove {
            if let Some((_, outlet)) = self.outlets.remove(&key) {
                debug!("- outlet({}) stopping", outlet.description());
                outlet.stop().await;
                debug!("- outlet({}) stopped", outlet.description());
            }
        }

        // 收集无效的入口
        let keys_to_remove: Vec<u32> = self
            .inlets
            .iter()
            .filter(|entry| {
                let id = *entry.key();
                let inlet = entry.value();
                !tunnels.iter().any(|tunnel| {
                    id == tunnel.id
                        && tunnel.enabled
                        && tunnel.receiver == self.player_id
                        && &inlet_description(tunnel) == inlet.description()
                })
            })
            .map(|entry| *entry.key())
            .collect();

        for key in keys_to_remove {
            if let Some((_, mut inlet)) = self.inlets.remove(&key) {
                debug!("- inlet({}) stopping", inlet.description());
                inlet.stop().await;
                debug!("- inlet({}) stopped", inlet.description());
            }
        }

        // 添加新的出口
        for tunnel in tunnels
            .iter()
            .filter(|t| t.enabled && t.sender == self.player_id)
        {
            if !self.outlets.contains_key(&tunnel.id) {
                let this_machine = tunnel.receiver == tunnel.sender;
                let inlets = self.inlets.clone();
                let outlets = self.outlets.clone();
                let transport = self.transport.clone();
                let self_player_id = self.player_id;
                let tunnel_id = tunnel.id;
                let player_id = tunnel.receiver;

                let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    let outlets = outlets.clone();
                    let transport = transport.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(inlet) = inlets.get(&tunnel_id) {
                                inlet.input(message).await;
                            } else {
                                debug!("unknown inlet({})", tunnel_id);
                            }
                        } else {
                            Self::route_proxy_message(
                                outlets,
                                inlets,
                                transport,
                                self_player_id,
                                player_id,
                                tunnel_id,
                                message,
                            )
                            .await;
                        }
                    })
                });
                debug!("+ outlet({})", outlet_description(tunnel));
                self.outlets.insert(
                    tunnel_id,
                    Outlet::new(outlet_output, outlet_description(tunnel)),
                );
            }
        }

        // 添加新的入口
        for tunnel in tunnels
            .iter()
            .filter(|t| t.enabled && t.receiver == self.player_id)
        {
            if !self.inlets.contains_key(&tunnel.id) {
                let this_machine = tunnel.receiver == tunnel.sender;
                let tunnel_id = tunnel.id;
                let inlets = self.inlets.clone();
                let outlets = self.outlets.clone();
                let transport = self.transport.clone();
                let self_player_id = self.player_id;
                let player_id = tunnel.sender;

                let source = tunnel
                    .source
                    .as_ref()
                    .map_or(String::new(), |x| x.addr.clone());
                let endpoint = tunnel
                    .endpoint
                    .as_ref()
                    .map_or(String::new(), |x| x.addr.clone());

                let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    let outlets = outlets.clone();
                    let transport = transport.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(outlet) = outlets.get(&tunnel_id) {
                                outlet.input(message).await;
                            } else {
                                debug!("unknown outlet({})", tunnel_id);
                            }
                        } else {
                            Self::route_proxy_message(
                                outlets,
                                inlets,
                                transport,
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
                        "unsupported tunnel type {} ({})",
                        tunnel.tunnel_type, source
                    );
                } else {
                    let mut inlet = Inlet::new(inlet_output, inlet_description(tunnel));
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
                        error!("inlet listen failed {}: {}", source, err);
                    } else {
                        debug!("+ inlet({})", inlet.description());
                        self.inlets.insert(tunnel.id, inlet);
                    }
                }
            }
        }
    }

    // ─── 代理消息路由 ──────────────────────────────────────────────────────────

    /// 路由代理消息：本机直接投递，远端通过传输层发送。
    async fn route_proxy_message(
        outlets: Arc<DashMap<u32, Arc<Outlet>>>,
        inlets: Arc<DashMap<u32, Inlet>>,
        transport: ClientTransport<S>,
        self_player_id: u32,
        player_id: u32,
        tunnel_id: u32,
        proxy_message: ProxyMessage,
    ) {
        if self_player_id == player_id {
            // 本机路由
            if message_bridge::is_i2o_message(&proxy_message) {
                if let Some(outlet) = outlets.get(&tunnel_id) {
                    outlet.input(proxy_message).await;
                }
            } else if let Some(inlet) = inlets.get(&tunnel_id) {
                inlet.input(proxy_message).await;
            }
        } else {
            // 远端路由
            let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
            if !message.is_none() {
                if let Err(e) = transport.send_proxy_message(0, &message).await {
                    warn!("failed to send proxy message (tunnel={}): {}", tunnel_id, e);
                }
            }
        }
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

fn fmt_point(point: &Option<TunnelPoint>) -> String {
    match point {
        Some(p) => p.addr.to_string(),
        None => "none".to_string(),
    }
}

fn outlet_description(tunnel: &Tunnel) -> String {
    format!(
        "tunnel#{} sender:{} enabled:{}",
        tunnel.id, tunnel.sender, tunnel.enabled
    )
}

fn fmt_tunnel_type(t: i32) -> &'static str {
    match t {
        0 => "tcp",
        1 => "udp",
        2 => "socks5",
        3 => "http",
        _ => "unknown",
    }
}

fn inlet_description(tunnel: &Tunnel) -> String {
    let custom_mapping: String = if tunnel.custom_mapping.is_empty() {
        String::new()
    } else {
        tunnel
            .custom_mapping
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "tunnel#{}[{}] {} -> {} sender:{} receiver:{} enabled:{} compressed:{} encrypt:{} auth:{}:{} mapping:[{}]",
        tunnel.id,
        fmt_tunnel_type(tunnel.tunnel_type),
        fmt_point(&tunnel.source),
        fmt_point(&tunnel.endpoint),
        tunnel.sender,
        tunnel.receiver,
        tunnel.enabled,
        tunnel.is_compressed,
        if tunnel.encryption_method.is_empty() { "none" } else { &tunnel.encryption_method },
        tunnel.username,
        tunnel.password,
        custom_mapping,
    )
}
