use crate::global::GLOBAL_DB_POOL;
use anyhow::anyhow;

#[derive(Debug)]
pub struct Channel {
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

pub struct ChannelManager {
    channels: Vec<Channel>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }

    pub async fn load_all_channel(&mut self) -> anyhow::Result<()> {
        self.channels = sqlx::query_as!(Channel, "SELECT * FROM channel")
            .fetch_all(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        Ok(())
    }

    /// 增加通道
    pub async fn add_channel(&mut self, mut channel: Channel) -> anyhow::Result<()> {
        let channel_id = sqlx::query!(
                "INSERT INTO channel (source, endpoint, enabled, sender, receiver, description) VALUES (?, ?, ?, ?, ?, ?)",
                channel.source,
                channel.endpoint,
                channel.enabled,
                channel.sender,
                channel.receiver,
                channel.description
            ).execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .last_insert_id();

        channel.id = channel_id as u32;
        self.channels.push(channel);
        Ok(())
    }

    /// 删除通道
    pub async fn delete_channel(&mut self, channel_id: u32) -> anyhow::Result<()> {
        if sqlx::query!("DELETE FROM channel WHERE id = ?", channel_id)
            .execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected()
            == 1
        {
            if let Some(index) = self.channels.iter().position(|it| it.id == channel_id) {
                self.channels.remove(index);
            }
            return Ok(());
        }
        Err(anyhow!(format!(
            "Unable to find channel_id: {}",
            channel_id
        )))
    }

    /// 更新通道
    pub async fn update_channel(&mut self, channel: Channel) -> anyhow::Result<()> {
        if let Some(index) = self.channels.iter().position(|it| it.id == channel.id) {
            if sqlx::query!(
                "UPDATE channel SET source = ?, endpoint = ?, enabled = ?, sender = ?, receiver = ?, description = ? WHERE id = ?",
                channel.source,
                channel.endpoint,
                channel.enabled,
                channel.sender,
                channel.receiver,
                channel.description,
                channel.id
            ).execute(GLOBAL_DB_POOL.get().unwrap())
                .await?
                .rows_affected() == 1 {
                self.channels[index] = channel;
                return Ok(());
            }
            return Err(anyhow!(format!(
                "Data update failed, channel_id: {}",
                channel.id
            )));
        }

        Err(anyhow!(format!(
            "Unable to find channel_id: {}",
            channel.id
        )))
    }
}
