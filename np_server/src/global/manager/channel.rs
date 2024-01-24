use crate::global::GLOBAL_DB_POOL;

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

    pub async fn delete_channel(&mut self, channel_id: u32) {

    }
}
