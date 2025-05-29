use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::Tunnel;
use crate::orm_entity::tunnel;
use crate::player::PlayerId;
use crate::utils::str::{
    get_tunnel_address_port, is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address,
};
use anyhow::anyhow;
use np_base::proxy::inlet::InletProxyType;
use np_proto::message_map::MessageType;
use np_proto::{class_def, server_client};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct TunnelManager {
    pub tunnels: RwLock<Vec<tunnel::Model>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: RwLock::new(Vec::new()),
        }
    }

    pub async fn load_all_tunnel(&self) -> anyhow::Result<()> {
        *self.tunnels.write().await = Tunnel::find().all(GLOBAL_DB_POOL.get().unwrap()).await?;
        Ok(())
    }

    /// 增加通道
    pub async fn add_tunnel(&self, mut tunnel: tunnel::Model) -> anyhow::Result<()> {
        self.tunnel_detection(&tunnel).await?;

        let new_tunnel = tunnel::ActiveModel {
            id: Default::default(),
            source: Set(tunnel.source.to_owned()),
            endpoint: Set(tunnel.endpoint.to_owned()),
            enabled: Set(tunnel.enabled),
            sender: Set(tunnel.sender),
            receiver: Set(tunnel.receiver),
            description: Set(tunnel.description.to_owned()),
            tunnel_type: Set(tunnel.tunnel_type),
            password: Set(tunnel.password.to_owned()),
            username: Set(tunnel.username.to_owned()),
            is_compressed: Set(tunnel.is_compressed),
            custom_mapping: Set(tunnel.custom_mapping.to_owned()),
            encryption_method: Set(tunnel.encryption_method.to_owned()),
        };

        let new_tunnel = new_tunnel.insert(GLOBAL_DB_POOL.get().unwrap()).await?;
        tunnel.id = new_tunnel.id;

        Self::broadcast_tunnel_info(tunnel.sender, &tunnel, false).await;
        if tunnel.sender != tunnel.receiver {
            Self::broadcast_tunnel_info(tunnel.receiver, &tunnel, false).await;
        }
        self.tunnels.write().await.push(tunnel);

        GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;

        Ok(())
    }

    /// 删除通道
    pub async fn delete_tunnel(&self, tunnel_id: u32) -> anyhow::Result<()> {
        let rows_affected = Tunnel::delete_by_id(tunnel_id)
            .exec(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected;

        anyhow::ensure!(
            rows_affected == 1,
            "delete_tunnel: rows_affected = {}",
            rows_affected
        );

        let position = {
            self.tunnels
                .read()
                .await
                .iter()
                .position(|it| it.id == tunnel_id)
        };
        if let Some(index) = position {
            let tunnel = self.tunnels.write().await.remove(index);
            Self::broadcast_tunnel_info(tunnel.sender, &tunnel, true).await;
            if tunnel.sender != tunnel.receiver {
                Self::broadcast_tunnel_info(tunnel.receiver, &tunnel, true).await;
            }

            GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;
        }
        Ok(())
    }

    /// 更新通道
    pub async fn update_tunnel(&self, tunnel: tunnel::Model) -> anyhow::Result<()> {
        self.tunnel_detection(&tunnel).await?;

        let position = {
            self.tunnels
                .read()
                .await
                .iter()
                .position(|it| it.id == tunnel.id)
        };

        if let Some(index) = position {
            let db_tunnel = Tunnel::find_by_id(tunnel.id)
                .one(GLOBAL_DB_POOL.get().unwrap())
                .await?;
            anyhow::ensure!(db_tunnel.is_some(), "Can't find tunnel: {}", tunnel.id);

            let mut db_tunnel: tunnel::ActiveModel = db_tunnel.unwrap().into();
            db_tunnel.source = Set(tunnel.source.to_owned());
            db_tunnel.endpoint = Set(tunnel.endpoint.to_owned());
            db_tunnel.enabled = Set(tunnel.enabled);
            db_tunnel.sender = Set(tunnel.sender);
            db_tunnel.receiver = Set(tunnel.receiver);
            db_tunnel.description = Set(tunnel.description.to_owned());
            db_tunnel.tunnel_type = Set(tunnel.tunnel_type);
            db_tunnel.password = Set(tunnel.password.to_owned());
            db_tunnel.username = Set(tunnel.username.to_owned());
            db_tunnel.is_compressed = Set(tunnel.is_compressed.to_owned());
            db_tunnel.custom_mapping = Set(tunnel.custom_mapping.to_owned());
            db_tunnel.encryption_method = Set(tunnel.encryption_method.to_owned());
            db_tunnel.update(GLOBAL_DB_POOL.get().unwrap()).await?;

            let old_sender = self.tunnels.read().await[index].sender;
            let old_receiver = self.tunnels.read().await[index].receiver;

            if old_sender != tunnel.sender {
                Self::broadcast_tunnel_info(old_sender, &tunnel, true).await;
            }
            if old_receiver != tunnel.sender {
                Self::broadcast_tunnel_info(old_receiver, &tunnel, true).await;
            }
            Self::broadcast_tunnel_info(tunnel.sender, &tunnel, false).await;
            if tunnel.sender != tunnel.receiver {
                Self::broadcast_tunnel_info(tunnel.receiver, &tunnel, false).await;
            }

            self.tunnels.write().await[index] = tunnel;
            GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;
            return Ok(());
        }
        Err(anyhow!(format!("Unable to find tunnel_id: {}", tunnel.id)))
    }

    /// 广播通道修改通知
    async fn broadcast_tunnel_info(player_id: PlayerId, tunnel: &tunnel::Model, is_delete: bool) {
        if player_id != 0 {
            if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id).await {
                let _ = player
                    .read()
                    .await
                    .send_push(&MessageType::ServerClientModifyTunnelNtf(
                        server_client::ModifyTunnelNtf {
                            is_delete,
                            tunnel: Some(tunnel.into()),
                        },
                    ))
                    .await;
            }
        }
    }

    async fn tunnel_detection(&self, tunnel: &tunnel::Model) -> anyhow::Result<()> {
        // 地址合法性检测
        if !is_valid_tunnel_source_address(&tunnel.source) {
            return Err(anyhow!("source address format error"));
        }

        // SOCKS5 HTTP类型不检测
        let proxy_type = InletProxyType::from_u32(tunnel.tunnel_type);
        match proxy_type {
            InletProxyType::TCP | InletProxyType::UDP => {
                if !is_valid_tunnel_endpoint_address(&tunnel.endpoint) {
                    return Err(anyhow!("endpoint address format error"));
                }
            }
            _ => {}
        }

        // 玩家id检测
        self.player_id_detection(tunnel.sender).await?;
        self.player_id_detection(tunnel.receiver).await?;

        // 端口冲突检测
        if self
            .port_conflict_detection(
                tunnel.receiver,
                get_tunnel_address_port(&tunnel.source),
                Some(tunnel.id),
                matches!(proxy_type, InletProxyType::UDP),
            )
            .await
        {
            return Err(anyhow!("port already in use"));
        }
        Ok(())
    }

    /// 检测玩家id是否合法
    async fn player_id_detection(&self, player_id: PlayerId) -> anyhow::Result<()> {
        if player_id != 0 && !GLOBAL_MANAGER.player_manager.contain(player_id).await {
            Err(anyhow!("player id {} does not exist", player_id))
        } else {
            Ok(())
        }
    }

    /// 检测端口是否冲突
    async fn port_conflict_detection(
        &self,
        receiver: u32,
        port: Option<u16>,
        tunnel_id: Option<u32>,
        is_udp: bool,
    ) -> bool {
        self.tunnels.read().await.iter().any(|x| {
            x.receiver == receiver
                && tunnel_id != Some(x.id)
                && is_udp == matches!(InletProxyType::from_u32(x.tunnel_type), InletProxyType::UDP)
                && get_tunnel_address_port(&x.source) == port
        })
    }

    /// 查询通道
    pub async fn query(&self, page_number: usize, page_size: usize) -> Vec<tunnel::Model> {
        let page_size = if page_size == 0 || page_size > 100 {
            10
        } else {
            page_size
        };
        let start = page_number * page_size;
        let mut end = start + page_size;
        let tunnel_num = self.tunnels.read().await.len();
        if end > tunnel_num {
            end = tunnel_num;
        }

        if start <= end && end <= tunnel_num {
            self.tunnels.read().await[start..end].to_vec()
        } else {
            vec![]
        }
    }
}

impl tunnel::Model {
    pub fn outlet_description(&self) -> String {
        format!(
            "id:{}-sender:{}-enabled:{}",
            self.id, self.sender, self.enabled
        )
    }

    pub fn inlet_description(&self) -> String {
        format!(
            "id:{}-source:{}-endpoint:{}-sender:{}-receiver:{}-tunnel_type:{}-username:{}-password:{}-enabled:{}-is_compressed:{}-encryption_method:{}-custom_mapping:{}",
            self.id,
            self.source,
            self.endpoint,
            self.sender,
            self.receiver,
            self.tunnel_type,
            self.username,
            self.password,
            self.enabled,
            self.is_compressed,
            self.encryption_method,
            self.custom_mapping,
        )
    }
}

impl From<&tunnel::Model> for class_def::Tunnel {
    fn from(tunnel: &tunnel::Model) -> Self {
        let custom_mapping: HashMap<String, String> =
            serde_json::from_str(&tunnel.custom_mapping).map_or(HashMap::new(), |x| x);

        Self {
            source: Some(class_def::TunnelPoint {
                addr: tunnel.source.clone(),
            }),
            endpoint: Some(class_def::TunnelPoint {
                addr: tunnel.endpoint.clone(),
            }),
            id: tunnel.id,
            enabled: tunnel.enabled == 1,
            sender: tunnel.sender,
            receiver: tunnel.receiver,
            tunnel_type: tunnel.tunnel_type as i32,
            username: tunnel.username.clone(),
            password: tunnel.password.clone(),
            is_compressed: tunnel.is_compressed == 1,
            encryption_method: tunnel.encryption_method.clone(),
            custom_mapping,
        }
    }
}
