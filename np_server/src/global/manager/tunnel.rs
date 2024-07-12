use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::utils::str::{
    get_tunnel_address_port, is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address,
};
use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct Tunnel {
    /// 入口
    pub source: String,
    /// 出口
    pub endpoint: String,
    /// 通道id
    pub id: u32,
    /// 是否启用
    pub enabled: u8,
    /// 发送方id(出口)
    pub sender: u32,
    /// 接收方id(入口)
    pub receiver: u32,
    /// 描述文本
    pub description: String,
    /// 通道类型
    pub tunnel_type: u32,
    /// 密码
    pub password: String,
    /// 用户名
    pub username: String,
}

pub struct TunnelManager {
    pub tunnels: Vec<Tunnel>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Vec::new(),
        }
    }

    pub async fn load_all_tunnel(&mut self) -> anyhow::Result<()> {
        self.tunnels = sqlx::query_as!(Tunnel, "SELECT * FROM tunnel")
            .fetch_all(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        Ok(())
    }

    /// 增加通道
    pub async fn add_tunnel(&mut self, mut tunnel: Tunnel) -> anyhow::Result<()> {
        if !is_valid_tunnel_source_address(&tunnel.source)
            || !is_valid_tunnel_endpoint_address(&tunnel.endpoint)
        {
            return Err(anyhow!("Address format error"));
        }

        // 端口冲突检测
        if self.port_conflict_detection(
            tunnel.sender,
            get_tunnel_address_port(&tunnel.source),
            None,
        ) {
            return Err(anyhow!("Port Conflict"));
        }

        let tunnel_id = sqlx::query!(
            "INSERT INTO tunnel (source, endpoint, enabled, sender, receiver, description, tunnel_type, password, username) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            tunnel.source,
            tunnel.endpoint,
            tunnel.enabled,
            tunnel.sender,
            tunnel.receiver,
            tunnel.description,
            tunnel.tunnel_type,
            tunnel.password,
            tunnel.username,
        )
        .execute(GLOBAL_DB_POOL.get().unwrap())
        .await?
        .last_insert_id();

        tunnel.id = tunnel_id as u32;
        self.tunnels.push(tunnel);

        GLOBAL_MANAGER
            .proxy_manager
            .write()
            .await
            .sync_tunnels(&self.tunnels)
            .await;

        Ok(())
    }

    /// 删除通道
    pub async fn delete_tunnel(&mut self, tunnel_id: u32) -> anyhow::Result<()> {
        if sqlx::query!("DELETE FROM tunnel WHERE id = ?", tunnel_id)
            .execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected()
            == 1
        {
            if let Some(index) = self.tunnels.iter().position(|it| it.id == tunnel_id) {
                self.tunnels.remove(index);
                GLOBAL_MANAGER
                    .proxy_manager
                    .write()
                    .await
                    .sync_tunnels(&self.tunnels)
                    .await;
            }
            return Ok(());
        }
        Err(anyhow!(format!("Unable to find tunnel_id: {}", tunnel_id)))
    }

    /// 更新通道
    pub async fn update_tunnel(&mut self, tunnel: Tunnel) -> anyhow::Result<()> {
        // 地址合法性检测
        if !is_valid_tunnel_source_address(&tunnel.source)
            || !is_valid_tunnel_endpoint_address(&tunnel.endpoint)
        {
            return Err(anyhow!("Address format error"));
        }
        // 端口冲突检测
        if self.port_conflict_detection(
            tunnel.sender,
            get_tunnel_address_port(&tunnel.source),
            Some(tunnel.id),
        ) {
            return Err(anyhow!("Port Conflict"));
        }

        if let Some(index) = self.tunnels.iter().position(|it| it.id == tunnel.id) {
            if sqlx::query!(
                "UPDATE tunnel SET source = ?, endpoint = ?, enabled = ?, sender = ?, receiver = ?, description = ?, tunnel_type = ?, password = ?, username = ? WHERE id = ?",
                tunnel.source,
                tunnel.endpoint,
                tunnel.enabled,
                tunnel.sender,
                tunnel.receiver,
                tunnel.description,
                tunnel.tunnel_type,
                tunnel.password,
                tunnel.username,
                tunnel.id
            ).execute(GLOBAL_DB_POOL.get().unwrap())
                .await?
                .rows_affected() == 1 {
                self.tunnels[index] = tunnel;
                GLOBAL_MANAGER.proxy_manager.write().await.sync_tunnels(&self.tunnels).await;
                return Ok(());
            }
            return Err(anyhow!(format!(
                "Data update failed, tunnel_id: {}",
                tunnel.id
            )));
        }

        Err(anyhow!(format!("Unable to find tunnel_id: {}", tunnel.id)))
    }

    /// 检测端口是否冲突
    fn port_conflict_detection(
        &self,
        sender: u32,
        port: Option<u16>,
        tunnel_id: Option<u32>,
    ) -> bool {
        self.tunnels
            .iter()
            .position(|x| {
                x.sender == sender
                    && tunnel_id != Some(x.id)
                    && get_tunnel_address_port(&x.source) == port
            })
            .is_some()
    }

    /// 查询通道
    pub async fn query(
        &self,
        page_number: usize,
        page_size: usize,
    ) -> anyhow::Result<Vec<Tunnel>, sqlx::Error> {
        let page_number = page_number;
        let page_size = if page_size <= 0 || page_size > 100 {
            10
        } else {
            page_size
        };
        let offset = page_number * page_size;

        // 分页查询数据
        let data_list: Vec<Tunnel> = sqlx::query_as!(
            Tunnel,
            "SELECT * FROM tunnel LIMIT ? OFFSET ?",
            page_size as u32,
            offset as u32
        )
        .fetch_all(GLOBAL_DB_POOL.get().unwrap())
        .await?;
        return Ok(data_list);
    }

    pub fn get_tunnel(&self, id: u32) -> Option<&Tunnel> {
        if let Some(index) = self.tunnels.iter().position(|x| x.id == id) {
            Some(&self.tunnels[index])
        } else {
            None
        }
    }
}

impl Tunnel {
    pub fn outlet_description(&self) -> String {
        format!(
            "id:{}-sender:{}-enabled:{}",
            self.id, self.sender, self.enabled
        )
    }

    pub fn inlet_description(&self) -> String {
        format!(
            "id:{}-source:{}-endpoint:{}-sender:{}-receiver:{}-tunnel_type:{}-username:{}-password:{}-enabled:{}",
            self.id,
            self.source,
            self.endpoint,
            self.sender,
            self.receiver,
            self.tunnel_type,
            self.username,
            self.password,
            self.enabled
        )
    }
}
