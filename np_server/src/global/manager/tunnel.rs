use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::Tunnel;
use crate::orm_entity::tunnel;
use crate::player::PlayerId;
use crate::utils::str::{is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address};
use anyhow::anyhow;
use np_base::proxy::inlet::InletProxyType;
use np_proto::message_map::MessageType;
use np_proto::{class_def, server_client};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use std::collections::HashMap;
use std::net::SocketAddr;
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
    pub async fn add_tunnel(&self, mut tunnel: tunnel::Model) -> anyhow::Result<u32> {
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
        let tunnel_id = tunnel.id;
        self.tunnels.write().await.push(tunnel);

        GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;

        Ok(tunnel_id)
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

        // 一次读锁找 index，立即 drop，再写锁 remove
        let position = {
            self.tunnels
                .read()
                .await
                .iter()
                .position(|it| it.id == tunnel_id)
        }; // ← 读锁 drop
        if let Some(index) = position {
            let tunnel = self.tunnels.write().await.remove(index);
            GLOBAL_MANAGER.proxy_manager.clear_inlet_status(tunnel_id);
            Self::broadcast_tunnel_info(tunnel.sender, &tunnel, true).await;
            if tunnel.sender != tunnel.receiver {
                Self::broadcast_tunnel_info(tunnel.receiver, &tunnel, true).await;
            }

            GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;
        }
        Ok(())
    }

    /// 更新通道
    pub async fn update_tunnel(
        &self,
        mut tunnel: tunnel::Model,
        preserve_password: bool,
    ) -> anyhow::Result<()> {
        self.tunnel_detection(&tunnel).await?;

        // 一次读锁找 position 和旧值，立即 drop
        let found = {
            let guard = self.tunnels.read().await;
            guard.iter().enumerate().find_map(|(i, t)| {
                if t.id == tunnel.id {
                    Some((i, t.sender, t.receiver))
                } else {
                    None
                }
            })
        }; // ← 读锁在此 drop

        if let Some((index, old_sender, old_receiver)) = found {
            let db_tunnel = Tunnel::find_by_id(tunnel.id)
                .one(GLOBAL_DB_POOL.get().unwrap())
                .await?;
            anyhow::ensure!(db_tunnel.is_some(), "Can't find tunnel: {}", tunnel.id);

            let db_tunnel = db_tunnel.unwrap();
            if preserve_password && tunnel.password.is_empty() {
                tunnel.password = db_tunnel.password.clone();
            }

            let mut db_tunnel: tunnel::ActiveModel = db_tunnel.into();
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

            // 配置已变化，旧的监听状态作废，等待新的上报
            GLOBAL_MANAGER.proxy_manager.clear_inlet_status(tunnel.id);

            if old_sender != tunnel.sender {
                Self::broadcast_tunnel_info(old_sender, &tunnel, true).await;
            }
            if old_receiver != tunnel.receiver && old_receiver != old_sender {
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

    /// 更新通道启用状态
    pub async fn update_tunnel_status(&self, tunnel_id: u32, enabled: u8) -> anyhow::Result<()> {
        let found = {
            let guard = self.tunnels.read().await;
            guard.iter().enumerate().find_map(|(i, t)| {
                if t.id == tunnel_id {
                    Some((i, t.clone()))
                } else {
                    None
                }
            })
        };

        if let Some((index, mut tunnel)) = found {
            let db_tunnel = Tunnel::find_by_id(tunnel_id)
                .one(GLOBAL_DB_POOL.get().unwrap())
                .await?;
            anyhow::ensure!(db_tunnel.is_some(), "Can't find tunnel: {}", tunnel_id);

            // 从禁用改为启用时，需要和其它已启用通道做冲突检测（含服务端真实端口探测）
            if enabled == 1 && tunnel.enabled != 1 {
                let mut check = tunnel.clone();
                check.enabled = 1;
                self.listen_conflict_detection(&check).await?;
            }

            tunnel.enabled = enabled;
            GLOBAL_MANAGER.proxy_manager.clear_inlet_status(tunnel_id);

            let mut db_tunnel: tunnel::ActiveModel = db_tunnel.unwrap().into();
            db_tunnel.enabled = Set(enabled);
            db_tunnel.update(GLOBAL_DB_POOL.get().unwrap()).await?;

            Self::broadcast_tunnel_info(tunnel.sender, &tunnel, false).await;
            if tunnel.sender != tunnel.receiver {
                Self::broadcast_tunnel_info(tunnel.receiver, &tunnel, false).await;
            }

            self.tunnels.write().await[index] = tunnel;
            GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;
            return Ok(());
        }

        Err(anyhow!(format!("Unable to find tunnel_id: {}", tunnel_id)))
    }

    /// 广播通道修改通知
    async fn broadcast_tunnel_info(player_id: PlayerId, tunnel: &tunnel::Model, is_delete: bool) {
        if player_id != 0 {
            if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id) {
                let _ = player
                    .read()
                    .await
                    .send_push(&MessageType::ServerClientModifyTunnelNtf(
                        server_client::ModifyTunnelNtf {
                            is_delete,
                            tunnel: Some(tunnel.into()),
                        },
                    ));
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
            InletProxyType::TCP | InletProxyType::UDP
                if !is_valid_tunnel_endpoint_address(&tunnel.endpoint) =>
            {
                return Err(anyhow!("endpoint address format error"));
            }
            _ => {}
        }

        // 玩家id检测
        self.player_id_detection(tunnel.sender).await?;
        self.player_id_detection(tunnel.receiver).await?;

        // 监听地址冲突检测（仅对启用状态的通道）
        self.listen_conflict_detection(tunnel).await
    }

    /// 启用状态的通道：检测与其它已启用通道的监听冲突；receiver 为服务端时再做真实 bind 探测
    async fn listen_conflict_detection(&self, tunnel: &tunnel::Model) -> anyhow::Result<()> {
        if tunnel.enabled != 1 {
            return Ok(());
        }
        let is_udp = is_udp_tunnel_type(tunnel.tunnel_type);
        // add 时 id 为 0，不会与任何已有通道相同
        let self_id = if tunnel.id == 0 {
            None
        } else {
            Some(tunnel.id)
        };
        if let Some(other) = self
            .port_conflict_detection(tunnel.receiver, &tunnel.source, self_id, is_udp)
            .await
        {
            return Err(anyhow!(
                "listen address {} conflicts with enabled tunnel #{} ({})",
                tunnel.source,
                other.id,
                other.source
            ));
        }

        if tunnel.receiver == 0 {
            // 若该地址正被本通道自己正在运行的入口占用，则跳过探测
            let own_running = {
                let guard = self.tunnels.read().await;
                guard.iter().any(|t| {
                    t.id == tunnel.id
                        && t.enabled == 1
                        && t.receiver == 0
                        && t.source == tunnel.source
                        && is_udp_tunnel_type(t.tunnel_type) == is_udp
                })
            } && GLOBAL_MANAGER.proxy_manager.has_running_inlet(tunnel.id);
            if !own_running {
                probe_bind(&tunnel.source, is_udp)?;
            }
        }
        Ok(())
    }

    /// 检测玩家id是否合法
    async fn player_id_detection(&self, player_id: PlayerId) -> anyhow::Result<()> {
        if player_id != 0 && !GLOBAL_MANAGER.player_manager.contain(player_id) {
            Err(anyhow!("player id {} does not exist", player_id))
        } else {
            Ok(())
        }
    }

    /// 检测监听地址是否与其它【已启用】通道冲突，返回冲突的通道
    async fn port_conflict_detection(
        &self,
        receiver: u32,
        source: &str,
        tunnel_id: Option<u32>,
        is_udp: bool,
    ) -> Option<tunnel::Model> {
        self.tunnels
            .read()
            .await
            .iter()
            .find(|x| {
                x.enabled == 1
                    && x.receiver == receiver
                    && tunnel_id != Some(x.id)
                    && is_udp == is_udp_tunnel_type(x.tunnel_type)
                    && listen_addr_conflict(&x.source, source)
            })
            .cloned()
    }

    /// 检测监听地址是否和现有已启用通道冲突
    pub async fn has_port_conflict(
        &self,
        receiver: u32,
        source: &str,
        tunnel_id: Option<u32>,
        is_udp: bool,
    ) -> bool {
        self.port_conflict_detection(receiver, source, tunnel_id, is_udp)
            .await
            .is_some()
    }

    /// 查询通道，同时返回总条数
    /// 只加一次读锁，避免调用方再次加锁获取总数。
    pub async fn query_with_total(
        &self,
        page_number: usize,
        page_size: usize,
    ) -> (Vec<tunnel::Model>, usize) {
        let page_size = if page_size == 0 {
            20
        } else {
            page_size.min(100)
        };
        let guard = self.tunnels.read().await;
        let total_count = guard.len();
        let start = page_number * page_size;
        let end = (start + page_size).min(total_count);
        let page = if start <= end {
            guard[start..end].to_vec()
        } else {
            vec![]
        };
        (page, total_count)
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

/// 1 = UDP；TCP / SOCKS5 / HTTP 都监听 TCP
pub fn is_udp_tunnel_type(tunnel_type: u32) -> bool {
    matches!(InletProxyType::from_u32(tunnel_type), InletProxyType::UDP)
}

/// 两个监听地址是否冲突：端口相同，且 IP 相同或任一方为通配地址（0.0.0.0 / ::）
pub fn listen_addr_conflict(a: &str, b: &str) -> bool {
    match (a.parse::<SocketAddr>(), b.parse::<SocketAddr>()) {
        (Ok(x), Ok(y)) => {
            x.port() == y.port()
                && (x.ip() == y.ip() || x.ip().is_unspecified() || y.ip().is_unspecified())
        }
        _ => a.trim() == b.trim(),
    }
}

/// 在服务端真实尝试 bind 监听地址后立即释放，用于发现被其它进程占用的端口
pub fn probe_bind(addr: &str, is_udp: bool) -> anyhow::Result<()> {
    let sock_addr: SocketAddr = addr
        .parse()
        .map_err(|_| anyhow!("source address format error"))?;
    let result = if is_udp {
        std::net::UdpSocket::bind(sock_addr).map(drop)
    } else {
        std::net::TcpListener::bind(sock_addr).map(drop)
    };
    result.map_err(|err| {
        anyhow!(
            "listen address {} ({}) is not available on server: {}",
            addr,
            if is_udp { "udp" } else { "tcp" },
            err
        )
    })
}
