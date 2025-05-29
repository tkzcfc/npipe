use crate::global::manager::GLOBAL_MANAGER;
use crate::player::PlayerId;
use log::{debug, error};
use np_base::proxy::inlet::{Inlet, InletDataEx, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::message_map::MessageType;
use np_proto::utils::message_bridge;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ProxyManager {
    outlets: Arc<RwLock<HashMap<u32, Arc<Outlet>>>>,
    inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(RwLock::new(HashMap::new())),
            inlets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn sync_tunnels(&self) {
        let tunnels = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await;

        // 收集无效的出口
        let mut keys_to_remove: Vec<_> = self
            .outlets
            .read()
            .await
            .iter()
            .filter(|(id, outlet)| {
                let retain = tunnels.iter().any(|tunnel| {
                    **id == tunnel.id
                        && tunnel.enabled == 1
                        && tunnel.sender == 0
                        && &tunnel.outlet_description() == outlet.description()
                });
                !retain
            })
            .map(|(key, _)| *key)
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
                        && tunnel.enabled == 1
                        && tunnel.receiver == 0
                        && &tunnel.inlet_description() == inlet.description()
                });
                !retain
            })
            .map(|(key, _)| *key)
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
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.sender == 0)
        {
            if !self.outlets.read().await.contains_key(&tunnel.id) {
                let this_machine = tunnel.receiver == tunnel.sender;
                let inlets = self.inlets.clone();
                let tunnel_id = tunnel.id;
                let player_id = tunnel.receiver;

                let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(inlet) = inlets.read().await.get(&tunnel_id) {
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
                debug!("start outlet({})", tunnel.outlet_description());
                self.outlets.write().await.insert(
                    tunnel_id,
                    Outlet::new(outlet_output, tunnel.outlet_description()),
                );
            }
        }

        // 添加代理入口
        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.receiver == 0)
        {
            if !self.inlets.read().await.contains_key(&tunnel.id) {
                let tunnel_id = tunnel.id;
                let this_machine = tunnel.receiver == tunnel.sender;
                let outlets = self.outlets.clone();
                let player_id = tunnel.sender;

                let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let outlets = outlets.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(outlet) = outlets.read().await.get(&tunnel_id) {
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

                let inlet_proxy_type = InletProxyType::from_u32(tunnel.tunnel_type);
                if matches!(inlet_proxy_type, InletProxyType::UNKNOWN) {
                    error!(
                        "inlet({}) unknown tunnel type: {}",
                        tunnel.source, tunnel.tunnel_type
                    );
                } else {
                    let mut inlet = Inlet::new(inlet_output, tunnel.inlet_description());
                    if let Err(err) = inlet
                        .start(
                            inlet_proxy_type,
                            tunnel.source.clone(),
                            tunnel.endpoint.clone(),
                            tunnel.is_compressed == 1,
                            tunnel.encryption_method.clone(),
                            InletDataEx::new(tunnel.username.clone(), tunnel.password.clone()),
                        )
                        .await
                    {
                        error!("inlet({}) start error: {}", tunnel.source, err);
                    } else {
                        debug!("start inlet({})", inlet.description());
                        self.inlets.write().await.insert(tunnel.id, inlet);
                    }
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

        if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(to_player_id).await {
            let is_online = { player.read().await.is_online() };
            if is_online {
                let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
                if !message.is_none() {
                    let _ = player.read().await.send_push(&message).await;
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
    if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id).await {
        let _ = player.read().await.send_push(message).await;
    }
}

async fn send_input_to_outlet(tunnel_id: &u32, proxy_message: ProxyMessage) {
    if let Some(outlet) = GLOBAL_MANAGER
        .proxy_manager
        .outlets
        .read()
        .await
        .get(tunnel_id)
    {
        outlet.input(proxy_message).await;
    }
}

async fn send_input_to_inlet(tunnel_id: &u32, proxy_message: ProxyMessage) {
    if let Some(inlet) = GLOBAL_MANAGER
        .proxy_manager
        .inlets
        .read()
        .await
        .get(tunnel_id)
    {
        inlet.input(proxy_message).await;
    }
}
