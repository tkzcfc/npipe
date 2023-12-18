use crate::session::Session;
use np_base::{client_server, generic};
use np_base::message_map::MessageType;
use std::io;

impl Session {
    pub async fn on_recv_message(&mut self, message: &MessageType) -> io::Result<MessageType> {
        match message {
            MessageType::ClientServerLoginReq(msg) => {
                return self.on_login_requst(msg).await
            }
            _ => {
                if let Some(ref player) = self.player {
                    return player.write().await.on_recv_message(message).await;
                }
            }
        }

        // 玩家未登录
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::PlayerNotLogin.into(),
            message: "player not logged in".into(),
        }))
    }

    async fn on_login_requst(
        &mut self,
        message: &client_server::LoginReq,
    ) -> io::Result<MessageType> {
        if self.player.is_some() {
            // 重复发送登录请求
            return Ok(MessageType::GenericSuccess(generic::Success {}));
        }

        // 根据用户名查找用户id
        let player_id = 100u32;

        Ok(MessageType::None)
    }
}
