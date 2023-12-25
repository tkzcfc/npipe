use super::Peer;
use np_proto::message_map::MessageType;
use std::io;

impl Peer {
    // 收到玩家向服务器推送消息
    pub(crate) async fn handle_push(&self, message: MessageType) -> io::Result<()> {
        Ok(())
    }
}
