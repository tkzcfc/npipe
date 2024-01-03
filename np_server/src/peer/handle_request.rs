use super::Peer;
use crate::player::manager::PLAYER_MANAGER;
use np_proto::message_map::MessageType;
use np_proto::{client_server, generic};

impl Peer {
    // 收到玩家向服务器请求的消息
    pub(crate) async fn handle_request(&self, message: MessageType) -> anyhow::Result<MessageType> {
        match message {
            MessageType::GenericPing(msg) => return self.on_ping(msg).await,
            MessageType::ClientServerLoginReq(msg) => return self.on_login_requst(msg).await,
            _ => {
                if let Some(ref player) = self.player {
                    return player.write().await.handle_request(message).await;
                }
            }
        }

        // 玩家未登录
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::PlayerNotLogin.into(),
            message: "player not logged in".into(),
        }))
    }

    async fn on_ping(&self, message: generic::Ping) -> anyhow::Result<MessageType> {
        Ok(MessageType::GenericPong(generic::Pong {
            ticks: message.ticks,
        }))
    }

    async fn on_login_requst(
        &self,
        message: client_server::LoginReq,
    ) -> anyhow::Result<MessageType> {
        if self.player.is_some() {
            // 重复发送登录请求
            return Ok(MessageType::GenericError(generic::Error {
                number: -1,
                message: "repeat login".into(),
            }));
        }

        // 根据用户名查找用户id
        let player_id = 100u32;

        // 用户登录成功，将会话绑定到Player上
        if let Some(player) = PLAYER_MANAGER.read().await.get_player(player_id) {
            let mut player = player.write().await;
            if player.is_online() {
                player.on_terminate_old_session().await;
            }
            player
                .on_connect_session(self.session_id, self.tx.clone().unwrap())
                .await;
            return Ok(MessageType::GenericSuccess(generic::Success {}));
        }

        Ok(MessageType::GenericError(generic::Error {
            number: -2,
            message: message.password,
        }))

        // 找不到玩家
        // Ok(MessageType::GenericError(generic::Error {
        //     number: -2,
        //     message: "unable to find player".into(),
        // }))
    }
}
