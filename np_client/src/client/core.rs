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
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf};
use tokio::select;
use tokio::sync::mpsc;

use super::io::{ping_forever, read_transport_events};
use super::transport::{Client, ClientTransport, IncomingFrame, TransportEvent};

impl<S> Client<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 完整的客户端运行流程：登录 → 事件循环/心跳/读取 → 清理隧道。
    pub(super) async fn run(
        &mut self,
        reader: ReadHalf<S>,
        event_tx: mpsc::UnboundedSender<TransportEvent>,
        event_rx: mpsc::UnboundedReceiver<TransportEvent>,
        last_active_secs: Arc<AtomicU64>,
        last_read_secs: Arc<AtomicU64>,
    ) -> anyhow::Result<()> {
        self.send_login().await?;

        let transport = self.transport.clone();
        let result;
        select! {
            r1 = self.event_loop(event_rx) => { result = r1 },
            r2 = ping_forever(transport, last_active_secs.clone(), last_read_secs.clone()) => { result = r2 },
            r3 = read_transport_events(reader, None, event_tx, last_active_secs, last_read_secs) => { result = r3 },
        }
        self.sync_tunnels(&Vec::new()).await;
        result
    }

    async fn event_loop(
        &mut self,
        mut event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    ) -> anyhow::Result<()> {
        while let Some(event) = event_rx.recv().await {
            match event {
                TransportEvent::Frame(incoming) => {
                    self.on_recv_frame(incoming).await?;
                }
                TransportEvent::Closed { path_id, reason } => {
                    if path_id.is_none() {
                        info!("Disconnect from the server: {reason}");
                        break;
                    }

                    debug!("transport path closed: path_id:{path_id:?}, reason:{reason}");
                    self.transport.remove_path(path_id).await;
                }
            }
        }

        Ok(())
    }

    pub(super) async fn send_login(&self) -> anyhow::Result<()> {
        info!("Start Login");
        self.transport
            .send_control_message(
                -1,
                &MessageType::ClientServerLoginReq(LoginReq {
                    version: "0.0.0".to_string(),
                    username: self.username.clone(),
                    password: self.password.clone(),
                    transport_max_connections: self.common_transport_max_connections(),
                    transport_idle_timeout_secs: self.common_transport_idle_timeout_secs(),
                }),
            )
            .await
    }

    fn common_transport_max_connections(&self) -> u32 {
        self.transport_max_connections
    }

    fn common_transport_idle_timeout_secs(&self) -> u32 {
        self.transport_idle_timeout_secs
    }

    async fn on_recv_frame(&mut self, incoming: IncomingFrame) -> anyhow::Result<()> {
        let frame = incoming.frame;
        if frame.len() < 8 {
            return Err(anyhow!("message length is too small"));
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
        if self.player_id == 0 {
            if serial > 0 {
                return match message {
                    MessageType::ServerClientLoginAck(msg) => {
                        info!("Login successful");
                        info!(
                            "transport negotiated, max_forward_paths:{}, idle_timeout_secs:{}",
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
        } else if serial > 0 {
            match message {
                MessageType::ServerClientBindTransportAck(msg) => {
                    debug!(
                        "transport bind successful, player_id:{}, connection_id:{}",
                        msg.player_id, msg.connection_id
                    );
                }
                MessageType::GenericError(err) => {
                    warn!(
                        "transport response error: {}, code: {}",
                        err.message, err.number
                    );
                    self.transport.remove_path(path_id).await;
                }
                _ => {}
            }
        } else if serial == 0 {
            self.handle_push(message).await?;
        }
        Ok(())
    }

    pub(super) async fn sync_tunnels(&mut self, tunnels: &[Tunnel]) {
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

        // 添加代理出口
        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled && tunnel.sender == self.player_id)
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
                                debug!("unknown inlet({tunnel_id})");
                            }
                        } else {
                            Self::send_proxy_message(
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

        // 添加代理入口
        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled && tunnel.receiver == self.player_id)
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
                    .map_or("".to_string(), |x| x.addr.clone());
                let endpoint = tunnel
                    .endpoint
                    .as_ref()
                    .map_or("".to_string(), |x| x.addr.clone());

                let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    let outlets = outlets.clone();
                    let transport = transport.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(outlet) = outlets.get(&tunnel_id) {
                                outlet.input(message).await;
                            } else {
                                debug!("unknown outlet({tunnel_id})");
                            }
                        } else {
                            Self::send_proxy_message(
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
                    error!("unsupported tunnel type {} for {}", tunnel.tunnel_type, source);
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
                        error!("inlet listen failed on {}: {}", source, err);
                    } else {
                        debug!("+ inlet({})", inlet.description());
                        self.inlets.insert(tunnel.id, inlet);
                    }
                }
            }
        }
    }

    async fn send_proxy_message(
        outlets: Arc<DashMap<u32, Arc<Outlet>>>,
        inlets: Arc<DashMap<u32, Inlet>>,
        transport: ClientTransport<S>,
        self_player_id: u32,
        player_id: u32,
        tunnel_id: u32,
        proxy_message: ProxyMessage,
    ) {
        if self_player_id == player_id {
            if message_bridge::is_i2o_message(&proxy_message) {
                if let Some(outlet) = outlets.get(&tunnel_id) {
                    outlet.input(proxy_message).await;
                }
            } else if let Some(inlet) = inlets.get(&tunnel_id) {
                inlet.input(proxy_message).await;
            }
        } else {
            let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
            if !message.is_none() {
                let _ = transport.send_proxy_message(0, &message).await;
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
                            self.transport.clone(),
                            self.player_id,
                            player_id,
                            tunnel_id,
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
                // sync_tunnels 会处理删除已不存在 tunnel 对应的 inlet/outlet
                let tunnel_list: Vec<&Tunnel> = self.tunnels.values().collect();
                let tunnel_list: Vec<Tunnel> = tunnel_list.into_iter().cloned().collect();
                self.sync_tunnels(&tunnel_list).await;
                return;
            }
            self.tunnels.insert(tunnel.id, tunnel);
            // 注意: 不需要 clone 整个 map，只需要收集 values 的引用再 sync
            let tunnel_list: Vec<Tunnel> = self.tunnels.values().cloned().collect();
            self.sync_tunnels(&tunnel_list).await;
        }
    }
}

fn fmt_point(point: &Option<TunnelPoint>) -> String {
    match point {
        Some(point) => point.addr.to_string(),
        _ => "none".to_string(),
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
