use super::Peer;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use np_proto::message_map::MessageType;
use np_proto::{class_def, client_server, generic, server_client};

impl Peer {
    // 收到玩家向服务器请求的消息
    pub(crate) async fn handle_request(
        &mut self,
        message: MessageType,
    ) -> anyhow::Result<MessageType> {
        match message {
            MessageType::GenericPing(msg) => return self.on_ping_request(msg).await,
            MessageType::ClientServerLoginReq(msg) => return self.on_login_request(msg).await,
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

    async fn on_ping_request(&self, message: generic::Ping) -> anyhow::Result<MessageType> {
        Ok(MessageType::GenericPong(generic::Pong {
            ticks: message.ticks,
        }))
    }

    async fn on_login_request(
        &mut self,
        message: client_server::LoginReq,
    ) -> anyhow::Result<MessageType> {
        if self.player.is_some() {
            // 重复发送登录请求
            return Ok(MessageType::GenericError(generic::Error {
                number: -1,
                message: "repeat login".into(),
            }));
        }

        struct Account {
            id: u32,
            password: String,
        }

        let account: Option<Account> = sqlx::query_as!(
            Account,
            "SELECT id, password FROM user WHERE username = ?",
            message.username
        )
        .fetch_optional(GLOBAL_DB_POOL.get().unwrap())
        .await?;

        if let Some(account) = account {
            // 密码错误
            if account.password != message.password {
                return Ok(MessageType::GenericError(generic::Error {
                    number: -2,
                    message: "Incorrect username or password".into(),
                }));
            }

            // 用户登录成功，将会话绑定到Player上
            if let Some(player) = GLOBAL_MANAGER
                .player_manager
                .read()
                .await
                .get_player(account.id)
            {
                self.player = Some(player.clone());
                let mut player = player.write().await;
                if player.is_online() {
                    player.on_terminate_old_session().await;
                }
                player
                    .on_connect_session(self.session_id, self.tx.clone().unwrap())
                    .await;

                let tunnel_list = GLOBAL_MANAGER
                    .tunnel_manager
                    .read()
                    .await
                    .tunnels
                    .iter()
                    .map(|x| class_def::Tunnel {
                        source: Some(class_def::TunnelPoint {
                            addr: x.source.clone(),
                        }),
                        endpoint: Some(class_def::TunnelPoint {
                            addr: x.endpoint.clone(),
                        }),
                        id: x.id,
                        enabled: x.enabled == 1,
                        sender: x.sender,
                        receiver: x.receiver,
                        tunnel_type: x.tunnel_type as i32,
                        password: x.password.clone(),
                        username: x.username.clone(),
                    })
                    .collect();

                return Ok(MessageType::ServerClientLoginAck(server_client::LoginAck {
                    player_id: account.id,
                    tunnel_list,
                }));
            }

            return Ok(MessageType::GenericError(generic::Error {
                number: -3,
                message: "unable to find player".into(),
            }));
        }

        // 找不到玩家，用户名无效
        Ok(MessageType::GenericError(generic::Error {
            number: -4,
            message: "Incorrect username or password".into(),
        }))
    }

    async fn on_register_request(
        &self,
        message: client_server::RegisterReq,
    ) -> anyhow::Result<MessageType> {
        let (code, msg) = GLOBAL_MANAGER
            .player_manager
            .write()
            .await
            .add_player(&message.username, &message.password)
            .await?;
        if code == 0 {
            Ok(MessageType::GenericSuccess(generic::Success {}))
        } else {
            Ok(MessageType::GenericError(generic::Error {
                number: code,
                message: msg,
            }))
        }
    }
}
