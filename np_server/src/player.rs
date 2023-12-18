use std::io;
use crate::session::Session;
use std::sync::Weak;
use tokio::sync::RwLock;
use np_base::generic;
use np_base::message_map::MessageType;

pub struct Player {
    #[warn(dead_code)]
    session: Weak<RwLock<Session>>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            session: Weak::default(),
        }
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
