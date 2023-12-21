use crate::session::Session;
use log::info;
use np_base::message_map::MessageType;
use np_base::{client_server, generic};
use std::io;
use crate::server::Server;

impl Session {
    pub async fn on_recv_message(&mut self, message: &MessageType) -> io::Result<MessageType> {
        match message {
            MessageType::ClientServerLoginReq(msg) => return self.on_login_requst(msg).await,
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
            return Ok(MessageType::GenericError(generic::Error {
                number: -1,
                message: "repeat login".into(),
            }));
        }

        info!("request login -------------->>>");

        // 根据用户名查找用户id
        let player_id = 100u32;

        // 用户登录成功，将会话绑定到Player上
        if let Some(player) = Server::instance().player_manager.write().await.get_player(player_id) {
            let mut player = player.write().await;
            if player.is_online() {
                player.on_terminate_old_session().await;
            }
            player.on_connect_session(self.get_session_id(), self.clone_tx()).await;
            return Ok(MessageType::GenericSuccess(generic::Success {}))
        }

        // 重复发送登录请求
        Ok(MessageType::GenericError(generic::Error {
            number: -1,
            message: "unable to find player".into(),
        }))
    }
}
