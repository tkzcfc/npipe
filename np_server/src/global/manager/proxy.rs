use crate::global::manager::GLOBAL_MANAGER;
use crate::peer::package_and_send_message;
use crate::player::{PlayerId, ProxySendPrep};
use dashmap::DashMap;
use log::{debug, error};
use np_base::proxy::inlet::{Inlet, InletDataEx, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::message_map::{get_message_size, MessageType};
use np_proto::utils::message_bridge;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 入口（监听端）运行时状态，仅保存在内存中
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InletRuntimeStatus {
    /// 是否正在监听
    pub ok: bool,
    /// 失败原因
    pub error: String,
    /// 监听地址
    pub listen_addr: String,
    /// 上报者：0 = 服务端自身，其它为客户端玩家 id
    pub reporter: PlayerId,
    /// 上报时间（Unix 秒，UTC）
    pub updated_at: i64,
}

pub struct ProxyManager {
    outlets: Arc<DashMap<u32, Arc<Outlet>>>,
    inlets: Arc<DashMap<u32, Inlet>>,
    /// tunnel_id -> 入口运行时状态
    inlet_status: DashMap<u32, InletRuntimeStatus>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(DashMap::new()),
            inlets: Arc::new(DashMap::new()),
            inlet_status: DashMap::new(),
        }
    }

    /// 服务端自身是否有该隧道正在运行的入口
    pub fn has_running_inlet(&self, tunnel_id: u32) -> bool {
        self.inlets
            .get(&tunnel_id)
            .map(|inlet| inlet.running())
            .unwrap_or(false)
    }

    pub fn set_inlet_status(
        &self,
        tunnel_id: u32,
        reporter: PlayerId,
        ok: bool,
        error: String,
        listen_addr: String,
    ) {
        self.inlet_status.insert(
            tunnel_id,
            InletRuntimeStatus {
                ok,
                error,
                listen_addr,
                reporter,
                updated_at: chrono::Utc::now().timestamp(),
            },
        );
    }

    pub fn get_inlet_status(&self, tunnel_id: u32) -> Option<InletRuntimeStatus> {
        self.inlet_status.get(&tunnel_id).map(|x| x.clone())
    }

    pub fn clear_inlet_status(&self, tunnel_id: u32) {
        self.inlet_status.remove(&tunnel_id);
    }

    /// 玩家上线/离线时清理其上报的入口状态
    pub fn clear_player_inlet_status(&self, player_id: PlayerId) {
        if player_id != 0 {
            self.inlet_status.retain(|_, s| s.reporter != player_id);
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
                .filter(|t| {
                    t.enabled == 1
                        && t.sender == 0
                        // 描述变化被删除的出口也要重建（removes 是在删除前计算的）
                        && (!self.outlets.contains_key(&t.id) || outlet_removes.contains(&t.id))
                })
                .map(|t| (t.id, t.receiver, t.sender, t.outlet_description()))
                .collect();

            // 需要添加的入口（克隆所需字段）
            let inlets_to_add: Vec<_> = tunnels
                .iter()
                .filter(|t| {
                    t.enabled == 1
                        && t.receiver == 0
                        // 描述变化被删除的入口也要重建（removes 是在删除前计算的）
                        && (!self.inlets.contains_key(&t.id) || inlet_removes.contains(&t.id))
                })
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
                // 只清理服务端自身上报的状态，避免误删客户端刚上报的状态（receiver 改为客户端的情况）
                self.inlet_status.remove_if(&key, |_, s| s.reporter == 0);
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
                self.set_inlet_status(
                    tunnel_id,
                    0,
                    false,
                    format!("unknown tunnel type: {tunnel_type}"),
                    source.clone(),
                );
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
                    self.set_inlet_status(tunnel_id, 0, false, err.to_string(), source.clone());
                } else {
                    debug!("start inlet({})", inlet.description());
                    self.set_inlet_status(tunnel_id, 0, true, String::new(), source.clone());
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

        // 先组包，再只为取发送端加锁。
        let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
        if message.is_none() {
            return;
        }

        if deliver_pb_to_player(to_player_id, &message).await {
            return;
        }

        // 玩家离线或找不到
        if let Some(proxy_message) = offline_reply(&message, to_player_id) {
            if from_player_id == 0 {
                if message_bridge::is_i2o_message(&proxy_message) {
                    send_input_to_outlet(&tunnel_id, proxy_message).await;
                } else {
                    send_input_to_inlet(&tunnel_id, proxy_message).await;
                }
            } else {
                let reply = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
                if !reply.is_none() {
                    let _ = deliver_pb_to_player(from_player_id, &reply).await;
                }
            }
        }
    }
}

async fn deliver_pb_to_player(player_id: PlayerId, message: &MessageType) -> bool {
    let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id) else {
        return false;
    };
    let tx_bytes = get_message_size(message) as u64 + 13;

    let ready = {
        let p = player.read().await;
        if !p.is_online() {
            return false;
        }
        match p.prepare_proxy_send(message) {
            ProxySendPrep::Ready(tx) => {
                p.traffic_tx.fetch_add(tx_bytes, Ordering::Relaxed);
                Some(tx)
            }
            ProxySendPrep::NeedsWrite => None,
        }
    };

    let tx = if let Some(tx) = ready {
        tx
    } else {
        let mut p = player.write().await;
        if !p.is_online() {
            return false;
        }
        p.traffic_tx.fetch_add(tx_bytes, Ordering::Relaxed);
        match p.resolve_proxy_sender(message) {
            Some(tx) => tx,
            None => {
                drop(p);
                let _ = package_and_send_message(&None, 0, message, true);
                return true;
            }
        }
    };

    let _ = package_and_send_message(&Some(tx), 0, message, true);
    true
}

fn offline_reply(message: &MessageType, to_player_id: PlayerId) -> Option<ProxyMessage> {
    let session_id = message_bridge::pb_proxy_session_id(message)?;
    match message {
        MessageType::GenericI2oConnect(_) => Some(ProxyMessage::O2iConnect(
            session_id,
            false,
            format!("no player {to_player_id} or the player is offline"),
        )),
        MessageType::GenericI2oSendData(_) | MessageType::GenericI2oRecvDataResult(_) => {
            Some(ProxyMessage::O2iDisconnect(session_id))
        }
        MessageType::GenericO2iConnect(_)
        | MessageType::GenericO2iRecvData(_)
        | MessageType::GenericO2iSendDataResult(_) => Some(ProxyMessage::I2oDisconnect(session_id)),
        _ => None,
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
