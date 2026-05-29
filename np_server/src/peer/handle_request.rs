use super::Peer;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::prelude::User;
use crate::orm_entity::user;
use chrono::Utc;
use log::trace;
use np_proto::message_map::MessageType;
use np_proto::{client_server, generic, server_client};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

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

        let user_result = User::find()
            .filter(user::Column::Username.eq(message.username))
            .filter(user::Column::Password.eq(message.password))
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;

        if user_result.is_none() {
            return Ok(MessageType::GenericError(generic::Error {
                number: -2,
                message: "Incorrect username or password".into(),
            }));
        }

        let user = user_result.unwrap();
        if user.enabled != 1 {
            return Ok(MessageType::GenericError(generic::Error {
                number: -4,
                message: "User has been disabled".into(),
            }));
        }

        // 用户登录成功，先记录登录历史，再将会话绑定到 Player 上
        if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(user.id) {
            // 记录登录历史
            let db = GLOBAL_DB_POOL.get().unwrap();
            let login_record = login_history::ActiveModel {
                id: NotSet,
                user_id: Set(user.id),
                ip_addr: Set(self.addr.to_string()),
                login_time: Set(Utc::now().naive_utc()),
                logout_time: Set(None),
                duration_secs: Set(None),
            };
            let login_record = login_record.insert(db).await?;
            let record_id = login_record.id;

            self.player = Some(player.clone());
            {
                let mut player = player.write().await;
                if player.is_online() {
                    player.on_terminate_old_session();
                }
                // 克隆流量计数器到 Peer，后续无锁计数
                let (rx, tx) = player.clone_traffic_counters();
                self.traffic_rx = Some(rx);
                self.traffic_tx = Some(tx);
                player.on_connect_session(self.session_id, self.tx.clone().unwrap(), &self.addr);
            }

            self.login_record_id = record_id;
            trace!(
                "login recorded, user_id:{}, record_id:{}",
                user.id,
                record_id
            );

            let tunnel_list = GLOBAL_MANAGER
                .tunnel_manager
                .tunnels
                .read()
                .await
                .iter()
                .filter(|x| x.receiver == user.id || x.sender == user.id)
                .map(|x| x.into())
                .collect();
            trace!("login success, player_id:{}", user.id);
            return Ok(MessageType::ServerClientLoginAck(server_client::LoginAck {
                player_id: user.id,
                tunnel_list,
            }));
        }

        Ok(MessageType::GenericError(generic::Error {
            number: -3,
            message: "unable to find player".into(),
        }))
    }

    async fn on_register_request(
        &self,
        message: client_server::RegisterReq,
    ) -> anyhow::Result<MessageType> {
        let (code, msg) = GLOBAL_MANAGER
            .player_manager
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
