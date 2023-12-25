use std::io;
use np_proto::message_map::MessageType;
use super::Peer;

impl Peer {
    // 收到玩家向服务器回复消息
    pub(crate) async fn handle_response(
        &self,
        message: MessageType,
    ) -> io::Result<()> {
        Ok(())
    }
}