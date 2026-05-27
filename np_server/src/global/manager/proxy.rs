use crate::global::manager::GLOBAL_MANAGER;
use crate::player::PlayerId;
use dashmap::DashMap;
use log::{debug, error};
use np_base::proxy::inlet::{Inlet, InletDataEx, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::message_map::MessageType;
use np_proto::utils::message_bridge;
use std::sync::Arc;

pub struct ProxyManager {
    /// DashMap 替代 Arc<RwLock<HashMap<u32, ...>>>:
    /// - 每条代理消息路由时只需访问特定 tunnel 的 inlet/outlet
    /// - 原来的全局 RwLock 导致所有消息路由串行化
    /// - DashMap 分片锁，不同 tunnel_id 并发访问无竞争
    outlets: Arc<DashMap<u32, Arc<Outlet>>>,
    inlets: Arc<DashMap<u32, Inlet>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(DashMap::new()),
            inlets: Arc::new(DashMap::new()),
        }
    }
    pub async fn sync_tunnels(&self) {
        // ── 收集阶段：持有读锁，全程同步，尽快释放 ───────────────────────────────
        let (outlet_removes, inlet_removes, outlets_to_add, inlets_to_add) = {
            let tunnels = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await;

            // 需要删除的出口 id
            let outlet_removes: Vec<u32> = self
                .outlets
                .iter()
                .filter(|entry| {
                    let id = *entry.key();
                    let outlet = entry.value();
                    !tunnels.iter().any(|tunnel| {
                        id == tunnel.id
                            && tunnel.enabled == 1
                            && tunnel.sender == 0
                            && &tunnel.outlet_description() == outlet.description()
                    })
                })
                .map(|e| *e.key())
                .collect();

            // 需要删除的入口 id
            let inlet_removes: Vec<u32> = self
                .inlets
                .iter()
                .filter(|entry| {
                    let id = *entry.key();
                    let inlet = entry.value();
                    !tunnels.iter().any(|tunnel| {
                        id == tunnel.id
                            && tunnel.enabled == 1
                            && tunnel.receiver == 0
                            && &tunnel.inlet_description() == inlet.description()
                    })
                })
                .map(|e| *e.key())
                .collect();

            // 需要添加的出口（克隆所需字段，避免锁跨 await）
            let outlets_to_add: Vec<_> = tunnels
                .iter()
                .filter(|t| t.enabled == 1 && t.sender == 0 && !self.outlets.contains_key(&t.id))
                .map(|t| (t.id, t.receiver, t.sender, t.outlet_description()))
                .collect();

            // 需要添加的入口（克隆所需字段）
            let inlets_to_add: Vec<_> = tunnels
                .iter()
                .filter(|t| t.enabled == 1 && t.receiver == 0 && !self.inlets.contains_key(&t.id))
                .map(|t| {
                    (
                        t.id,
                        t.receiver,
                        t.sender,
                        t.tunnel_type,
                        t.source.clone(),
                        t.endpoint.clone(),
                        t.is_compressed,
                        t.encryption_method.clone(),
                        t.username.clone(),
                        t.password.clone(),
                        t.inlet_description(),
                    )
                })
                .collect();

            (outlet_removes, inlet_removes, outlets_to_add, inlets_to_add)
        }; // ← 读锁在此 drop，后续 async 操作不再持锁

        // ── 执行阶段：不持有任何 tunnels 锁 ─────────────────────────────────────

        // 删除过期出口
        for key in outlet_removes {
            if let Some((_, outlet)) = self.outlets.remove(&key) {
                let description = outlet.description().to_owned();
                debug!("start deleting the outlet({description})");
                outlet.stop().await;
                debug!("delete outlet({description}) end");
            }
        }

        // 删除过期入口
        for key in inlet_removes {
            if let Some((_, mut inlet)) = self.inlets.remove(&key) {
                let description = inlet.description().to_owned();
                debug!("start deleting the inlet({description})");
                inlet.stop().await;
                debug!("delete inlet({description}) end");
            }
        }

        // 添加新出口
        for (tunnel_id, receiver, sender, outlet_desc) in outlets_to_add {
            let this_machine = receiver == sender;
            let inlets = self.inlets.clone();
            let player_id = receiver;

            let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                let inlets = inlets.clone();
                Box::pin(async move {
                    if this_machine {
                        if let Some(inlet) = inlets.get(&tunnel_id) {
                            inlet.input(message).await;
                        } else {
                            debug!("unknown inlet({tunnel_id})");
                        }
                    } else {
                        Self::send_proxy_message(0, player_id as PlayerId, tunnel_id, message)
                            .await;
                    }
                })
            });
            debug!("start outlet({outlet_desc})");
            self.outlets
                .insert(tunnel_id, Outlet::new(outlet_output, outlet_desc));
        }

        // 添加新入口
        for (
            tunnel_id,
            receiver,
            sender,
            tunnel_type,
            source,
            endpoint,
            is_compressed,
            encryption_method,
            username,
            password,
            inlet_desc,
        ) in inlets_to_add
        {
            let this_machine = receiver == sender;
            let outlets = self.outlets.clone();
            let player_id = sender;

            let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                let outlets = outlets.clone();
                Box::pin(async move {
                    if this_machine {
                        if let Some(outlet) = outlets.get(&tunnel_id) {
                            outlet.input(message).await;
                        } else {
                            debug!("unknown outlet({tunnel_id})");
                        }
                    } else {
                        Self::send_proxy_message(0, player_id as PlayerId, tunnel_id, message)
                            .await;
                    }
                })
            });

            let inlet_proxy_type = InletProxyType::from_u32(tunnel_type);
            if matches!(inlet_proxy_type, InletProxyType::UNKNOWN) {
                error!("inlet({source}) unknown tunnel type: {tunnel_type}");
            } else {
                let mut inlet = Inlet::new(inlet_output, inlet_desc);
                if let Err(err) = inlet
                    .start(
                        inlet_proxy_type,
                        source.clone(),
                        endpoint,
                        is_compressed == 1,
                        encryption_method,
                        InletDataEx::new(username, password),
                    )
                    .await
                {
                    error!("inlet({source}) start error: {err}");
                } else {
                    debug!("start inlet({})", inlet.description());
                    self.inlets.insert(tunnel_id, inlet);
                }
            }
        }
    }

    pub(crate) async fn send_proxy_message(
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        tunnel_id: u32,
        proxy_message: ProxyMessage,
    ) {
        if to_player_id == 0 {
            if message_bridge::is_i2o_message(&proxy_message) {
                send_input_to_outlet(&tunnel_id, proxy_message).await;
            } else {
                send_input_to_inlet(&tunnel_id, proxy_message).await;
            }
            return;
        }

        if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(to_player_id) {
            // 合并为单次读锁：同时检查 is_online 与发送，避免两次 read().await
            let p = player.read().await;
            if p.is_online() {
                let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
                if !message.is_none() {
                    let _ = p.send_push(&message);
                }
                return;
            }
        }

        // 玩家离线或找不到
        let message = match proxy_message {
            ProxyMessage::I2oConnect(session_id, ..) => Some(ProxyMessage::O2iConnect(
                session_id,
                false,
                format!("no player {to_player_id} or the player is offline"),
            )),

            ProxyMessage::I2oSendData(session_id, ..)
            | ProxyMessage::I2oRecvDataResult(session_id, ..) => {
                Some(ProxyMessage::O2iDisconnect(session_id))
            }

            ProxyMessage::O2iConnect(session_id, ..)
            | ProxyMessage::O2iRecvData(session_id, ..)
            | ProxyMessage::O2iSendDataResult(session_id, ..) => {
                Some(ProxyMessage::I2oDisconnect(session_id))
            }
            _ => None,
        };

        if let Some(proxy_message) = message {
            // 入口是服务器自己
            if from_player_id == 0 {
                if message_bridge::is_i2o_message(&proxy_message) {
                    send_input_to_outlet(&tunnel_id, proxy_message).await;
                } else {
                    send_input_to_inlet(&tunnel_id, proxy_message).await;
                }
            } else {
                push_message_to_player(
                    from_player_id,
                    &message_bridge::proxy_message_2_pb(proxy_message, tunnel_id),
                )
                .await;
            }
        }
    }
}

async fn push_message_to_player(player_id: PlayerId, message: &MessageType) {
    if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id) {
        let _ = player.read().await.send_push(message);
    }
}

async fn send_input_to_outlet(tunnel_id: &u32, proxy_message: ProxyMessage) {
    if let Some(outlet) = GLOBAL_MANAGER.proxy_manager.outlets.get(tunnel_id) {
        outlet.input(proxy_message).await;
    }
}

async fn send_input_to_inlet(tunnel_id: &u32, proxy_message: ProxyMessage) {
    if let Some(inlet) = GLOBAL_MANAGER.proxy_manager.inlets.get(tunnel_id) {
        inlet.input(proxy_message).await;
    }
}
