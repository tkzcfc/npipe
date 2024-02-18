use crate::global::GLOBAL_DB_POOL;
use crate::utils::str::{is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address};
use anyhow::anyhow;

#[derive(Debug)]
pub struct Tunnel {
    /// 入口
    pub source: String,
    /// 出口
    pub endpoint: String,
    /// 通道id
    pub id: u32,
    /// 是否启用
    pub enabled: u8,
    /// 发送方id
    pub sender: u32,
    /// 接收方id
    pub receiver: u32,
    /// 描述文本
    pub description: String,
}

pub struct TunnelManager {
    tunnels: Vec<Tunnel>,
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

        let tunnel_id = sqlx::query!(
                "INSERT INTO tunnel (source, endpoint, enabled, sender, receiver, description) VALUES (?, ?, ?, ?, ?, ?)",
                tunnel.source,
                tunnel.endpoint,
                tunnel.enabled,
                tunnel.sender,
                tunnel.receiver,
                tunnel.description
            ).execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .last_insert_id();

        tunnel.id = tunnel_id as u32;
        self.tunnels.push(tunnel);
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
            }
            return Ok(());
        }
        Err(anyhow!(format!(
            "Unable to find tunnel_id: {}",
            tunnel_id
        )))
    }

    /// 更新通道
    pub async fn update_tunnel(&mut self, tunnel: Tunnel) -> anyhow::Result<()> {
        if !is_valid_tunnel_source_address(&tunnel.source)
            || !is_valid_tunnel_endpoint_address(&tunnel.endpoint)
        {
            return Err(anyhow!("Address format error"));
        }
        if let Some(index) = self.tunnels.iter().position(|it| it.id == tunnel.id) {
            if sqlx::query!(
                "UPDATE tunnel SET source = ?, endpoint = ?, enabled = ?, sender = ?, receiver = ?, description = ? WHERE id = ?",
                tunnel.source,
                tunnel.endpoint,
                tunnel.enabled,
                tunnel.sender,
                tunnel.receiver,
                tunnel.description,
                tunnel.id
            ).execute(GLOBAL_DB_POOL.get().unwrap())
                .await?
                .rows_affected() == 1 {
                self.tunnels[index] = tunnel;
                return Ok(());
            }
            return Err(anyhow!(format!(
                "Data update failed, tunnel_id: {}",
                tunnel.id
            )));
        }

        Err(anyhow!(format!(
            "Unable to find tunnel_id: {}",
            tunnel.id
        )))
    }


}
