use crate::session::Session;
use np_base::generic;
use np_base::message_map::MessageType;
use std::io;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

pub type PlayerId = u32;

#[warn(dead_code)]
pub struct Player {
    session: Weak<RwLock<Session>>,
    player_id: PlayerId,
}

impl Player {
    pub fn new(player_id: PlayerId) -> Arc<RwLock<Player>> {
        Arc::new(RwLock::new(Player {
            session: Weak::default(),
            player_id,
        }))
    }

    pub fn get_player_id(&self) -> PlayerId {
        self.player_id
    }

    // 玩家上线
    pub async fn on_connect_session(&mut self) {}

    // 玩家离线
    pub async fn on_disconnect_session(&mut self) {}

    // 玩家被顶号，需要对旧的会话发送一些消息
    pub async fn on_terminate_old_session(&mut self) {}

    // 玩家收到消息
    pub async fn on_recv_message(&mut self, message: &MessageType) -> io::Result<MessageType> {
        // 客户端请求的消息，服务器未实现
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::InterfaceAbsent.into(),
            message: "interface absent".into(),
        }))
    }
}
