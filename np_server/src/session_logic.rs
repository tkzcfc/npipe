use crate::server::Server;
use crate::session::Session;
use log::info;
use np_base::message_map::MessageType;
use np_base::{client_server, generic};
use std::io;

impl Session {
    // 收到玩家向服务器请求的消息
    pub(crate) async fn on_recv_request(
        &mut self,
        message: &MessageType,
    ) -> io::Result<MessageType> {
        match message {
            MessageType::GenericPing(msg) => return self.on_ping(msg).await,
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

    // 收到玩家回复服务器的消息
    pub(crate) async fn on_recv_response(&mut self, _message: &MessageType) -> io::Result<()> {
        Ok(())
    }

    // 收到玩家向服务器推送的消息
    pub(crate) async fn on_recv_push(&mut self, _message: &MessageType) -> io::Result<()> {
        Ok(())
    }

    async fn on_ping(&mut self, message: &generic::Ping) -> io::Result<MessageType> {
        Ok(MessageType::GenericPong(generic::Pong {
            ticks: message.ticks,
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
        if let Some(player) = Server::instance()
            .player_manager
            .write()
            .await
            .get_player(player_id)
        {
            let mut player = player.write().await;
            if player.is_online() {
                player.on_terminate_old_session().await;
            }
            player
                .on_connect_session(self.get_session_id(), self.clone_tx())
                .await;
            return Ok(MessageType::GenericSuccess(generic::Success {}));
        }

        // 重复发送登录请求
        Ok(MessageType::GenericError(generic::Error {
            number: -2,
            message: "unable to find player".into(),
        }))
    }
}
