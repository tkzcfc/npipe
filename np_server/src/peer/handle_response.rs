use super::Peer;
use np_proto::message_map::MessageType;
use std::io;

impl Peer {
    // 收到玩家向服务器回复消息
    pub(crate) async fn handle_response(&self, _message: MessageType) -> io::Result<()> {
        Ok(())
    }
}
