use crate::global::manager::GLOBAL_MANAGER;
use crate::orm_entity::tunnel;
use crate::player::PlayerId;
use log::{debug, error, info};
use np_base::proxy::inlet::{Inlet, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use np_proto::generic;
use np_proto::message_map::MessageType;
use np_proto::utils::message_bridge;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ProxyManager {
    outlets: Arc<RwLock<HashMap<u32, Outlet>>>,
    inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(RwLock::new(HashMap::new())),
            inlets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn sync_tunnels(&self, tunnels: &Vec<tunnel::Model>) {
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
                        && tunnel.enabled == 1
                        && tunnel.receiver == 0
                        && &tunnel.inlet_description() == inlet.description()
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

                if let Some(inlet_proxy_type) = InletProxyType::from_u32(tunnel.tunnel_type) {
                    let mut inlet = Inlet::new(inlet_output, tunnel.inlet_description());
                    if let Err(err) = inlet
                        .start(
                            inlet_proxy_type,
                            tunnel.source.clone(),
                            tunnel.endpoint.clone(),
                            tunnel.is_compressed == 1,
                            tunnel.encryption_method.clone(),
                        )
                        .await
                    {
                        error!("inlet({}) start error: {}", tunnel.source, err);
                    } else {
                        self.inlets.write().await.insert(tunnel.id, inlet);
                    }
                } else {
                    error!(
                        "inlet({}) unknown tunnel type: {}",
                        tunnel.source, tunnel.tunnel_type
                    );
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
                if let Some(outlet) = GLOBAL_MANAGER
                    .proxy_manager
                    .read()
                    .await
                    .outlets
                    .read()
                    .await
                    .get(&tunnel_id)
                {
                    outlet.input(proxy_message).await;
                }
            } else {
                if let Some(inlet) = GLOBAL_MANAGER
                    .proxy_manager
                    .read()
                    .await
                    .inlets
                    .read()
                    .await
                    .get(&tunnel_id)
                {
                    inlet.input(proxy_message).await;
                }
            }
            return;
        }

        if let Some(player) = GLOBAL_MANAGER
            .player_manager
            .read()
            .await
            .get_player(to_player_id)
        {
            if player.read().await.is_online() {
                let message = message_bridge::proxy_message_2_pb(proxy_message, tunnel_id);
                match message {
                    MessageType::None => {}
                    _ => {
                        let _ = player.read().await.send_push(&message).await;
                    }
                }
                return;
            }
        }

        // 玩家离线或找不到
        if from_player_id == 0 {
            match proxy_message {
                ProxyMessage::I2oConnect(session_id, ..) => {
                    tokio::spawn(async move {
                        if let Some(inlet) = GLOBAL_MANAGER
                            .proxy_manager
                            .read()
                            .await
                            .inlets
                            .read()
                            .await
                            .get(&tunnel_id)
                        {
                            inlet
                                .input(ProxyMessage::O2iConnect(
                                    session_id,
                                    false,
                                    format!("no sender {to_player_id}"),
                                ))
                                .await;
                        }
                    });
                }
                ProxyMessage::I2oSendData(session_id, _)
                | ProxyMessage::I2oRecvDataResult(session_id, _) => {
                    tokio::spawn(async move {
                        if let Some(inlet) = GLOBAL_MANAGER
                            .proxy_manager
                            .read()
                            .await
                            .inlets
                            .read()
                            .await
                            .get(&tunnel_id)
                        {
                            inlet.input(ProxyMessage::O2iDisconnect(session_id)).await;
                        }
                    });
                }
                ProxyMessage::O2iRecvData(session_id, _)
                | ProxyMessage::O2iSendDataResult(session_id, _) => {
                    tokio::spawn(async move {
                        if let Some(outlet) = GLOBAL_MANAGER
                            .proxy_manager
                            .read()
                            .await
                            .outlets
                            .read()
                            .await
                            .get(&tunnel_id)
                        {
                            outlet.input(ProxyMessage::I2oDisconnect(session_id)).await;
                        }
                    });
                }
                _ => {}
            }
        } else {
            let message = match proxy_message {
                ProxyMessage::I2oConnect(session_id, ..) => {
                    MessageType::GenericO2iConnect(generic::O2iConnect {
                        tunnel_id,
                        session_id,
                        success: false,
                        error_info: format!("no sender {to_player_id}"),
                    })
                }
                ProxyMessage::I2oSendData(session_id, ..)
                | ProxyMessage::I2oRecvDataResult(session_id, ..) => {
                    MessageType::GenericO2iDisconnect(generic::O2iDisconnect {
                        tunnel_id,
                        session_id,
                    })
                }
                ProxyMessage::O2iRecvData(session_id, _)
                | ProxyMessage::O2iSendDataResult(session_id, _) => {
                    MessageType::GenericI2oDisconnect(generic::I2oDisconnect {
                        tunnel_id,
                        session_id,
                    })
                }
                _ => MessageType::None,
            };

            match message {
                MessageType::None => {}
                _ => {
                    if let Some(player) = GLOBAL_MANAGER
                        .player_manager
                        .read()
                        .await
                        .get_player(from_player_id)
                    {
                        let _ = player.read().await.send_push(&message).await;
                    }
                }
            }
        }
    }
}
