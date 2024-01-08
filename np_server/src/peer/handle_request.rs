use super::Peer;
use crate::global::GLOBAL_DB_POOL;
use crate::player::manager::PLAYER_MANAGER;
use crate::utils::str::{is_valid_password, is_valid_username};
use np_proto::generic::ErrorCode;
use np_proto::message_map::MessageType;
use np_proto::{client_server, generic};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

impl Peer {
    // 收到玩家向服务器请求的消息
    pub(crate) async fn handle_request(&self, message: MessageType) -> anyhow::Result<MessageType> {
        match message {
            MessageType::GenericPing(msg) => return self.on_ping(msg).await,
            MessageType::ClientServerLoginReq(msg) => return self.on_login_requst(msg).await,
            MessageType::ClientServerRegisterReq(msg) => {
                return self.on_register_request(msg).await
            }
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

    async fn on_register_request(
        &self,
        message: client_server::RegisterReq,
    ) -> anyhow::Result<MessageType> {
        // 参数长度越界检查
        if !is_valid_username(&message.username) || !is_valid_password(&message.password) {
            return Ok(MessageType::GenericError(generic::Error {
                number: -1,
                message: "Bad parameter".into(),
            }));
        }

        // 执行查询以检查用户名是否存在
        let record = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ?) as 'exists'",
            message.username
        )
        .fetch_one(GLOBAL_DB_POOL.get().unwrap())
        .await?;

        // 用户已存在
        if record.exists != 0 {
            return Ok(MessageType::GenericError(generic::Error {
                number: -2,
                message: "User already exists".into(),
            }));
        }

        let mut rng = StdRng::from_entropy();
        let mut count = 0;
        loop {
            count += 1;
            // 循环次数过多
            if count > 10000 {
                return Ok(MessageType::GenericError(generic::Error {
                    number: ErrorCode::InternalError.into(),
                    message: "Too many cycles".into(),
                }));
            }

            // 随机新的玩家id
            let id: u32 = rng.gen_range(10000000..99999999);
            if PLAYER_MANAGER.read().await.contain(id) {
                continue;
            }

            if sqlx::query!(
                "INSERT INTO users (id, username, password, type) VALUES (?, ?, ?, ?)",
                id,
                message.username,
                message.password,
                0
            )
            .execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected()
                == 1
            {
                PLAYER_MANAGER.write().await.create_player(id, 0).await;
                return Ok(MessageType::GenericSuccess(generic::Success {}));
            } else {
                return Ok(MessageType::GenericError(generic::Error {
                    number: -3,
                    message: "sqlx error".into(),
                }));
            }
        }
    }
}
