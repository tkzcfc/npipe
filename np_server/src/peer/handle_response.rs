use super::Peer;
use np_proto::message_map::MessageType;

impl Peer {
    // 收到玩家向服务器回复消息
    pub(crate) async fn handle_response(&self, _message: MessageType) -> anyhow::Result<()> {
        Ok(())
    }
}
