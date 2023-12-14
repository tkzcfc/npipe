use std::sync::Weak;
use tokio::sync::RwLock;
use crate::session::Session;

pub struct Player {
    session : Weak<RwLock<Session>>,
}

impl Player {
    // 玩家上线
    pub async fn on_connect_session(&mut self) {

    }

    // 玩家离线
    pub async fn on_disconnect_session(&mut self) {

    }

    // 玩家被顶号，需要对旧的会话发送一些消息
    pub async fn on_terminate_old_session(&mut self) {

    }
}

