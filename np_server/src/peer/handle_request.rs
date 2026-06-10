use super::Peer;
use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::prelude::User;
use crate::orm_entity::user;
use chrono::Utc;
use log::{debug, info, trace, warn};
use np_proto::message_map::MessageType;
use np_proto::utils::transport::TRANSPORT_CONNECTION_TYPE_FORWARD;
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
            MessageType::ClientServerBindTransportReq(msg) => {
                return self.on_bind_transport_request(msg).await
            }
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

        let username = message.username;
        let password = message.password;
        let requested_transport_max_connections = message.transport_max_connections;
        let requested_transport_idle_timeout_secs = message.transport_idle_timeout_secs;

        let user_result = User::find()
            .filter(user::Column::Username.eq(username))
            .filter(user::Column::Password.eq(password))
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
                number: -3,
                message: "User has been disabled".into(),
            }));
        }

        // 用户登录成功，先记录登录历史，再将会话绑定到 Player 上
        if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(user.id) {
            let transport_max_connections = negotiate_transport_max_connections(
                requested_transport_max_connections,
                GLOBAL_CONFIG.transport_max_connections_per_player,
            );
            let transport_idle_timeout_secs = negotiate_transport_idle_timeout_secs(
                requested_transport_idle_timeout_secs,
                GLOBAL_CONFIG.transport_idle_timeout_secs,
            );
            info!(
                "transport negotiated, player_id:{}, client_max_connections:{}, server_max_connections:{}, negotiated_max_connections:{}, client_idle_timeout_secs:{}, server_idle_timeout_secs:{}, negotiated_idle_timeout_secs:{}",
                user.id,
                requested_transport_max_connections,
                GLOBAL_CONFIG.transport_max_connections_per_player,
                transport_max_connections,
                requested_transport_idle_timeout_secs,
                GLOBAL_CONFIG.transport_idle_timeout_secs,
                transport_idle_timeout_secs
            );

            // 记录登录历史
            let db = GLOBAL_DB_POOL.get().unwrap();
            let login_record = login_history::ActiveModel {
                id: NotSet,
                user_id: Set(user.id),
                ip_addr: Set(self.addr.to_string()),
                login_time: Set(Utc::now().naive_utc()),
                logout_time: Set(None),
                duration_secs: Set(None),
                login_source: Set("client".to_owned()),
                success: Set(1),
            };
            let login_record = login_record.insert(db).await?;
            let record_id = login_record.id;

            self.player = Some(player.clone());
            let transport_token = {
                let mut player = player.write().await;
                if player.is_online() {
                    player.on_terminate_old_session();
                }
                // 克隆流量计数器到 Peer，后续无锁计数
                let (rx, tx) = player.clone_traffic_counters();
                self.traffic_rx = Some(rx);
                self.traffic_tx = Some(tx);
                player.on_connect_session(
                    self.session_id,
                    self.tx.clone().unwrap(),
                    &self.addr,
                    self.connection_protocol(),
                );
                self.mark_control_connection();
                player.configure_transport(transport_max_connections, transport_idle_timeout_secs)
            };

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
                transport_token,
                transport_max_connections,
                transport_idle_timeout_secs,
            }));
        }

        Ok(MessageType::GenericError(generic::Error {
            number: -4,
            message: "unable to find player".into(),
        }))
    }

    async fn on_bind_transport_request(
        &mut self,
        message: client_server::BindTransportReq,
    ) -> anyhow::Result<MessageType> {
        if self.player.is_some() {
            warn!(
                "reject transport bind, session_id:{}, addr:{}, reason:already bound",
                self.session_id(),
                self.addr()
            );
            return Ok(MessageType::GenericError(generic::Error {
                number: -1,
                message: "transport connection is already bound".into(),
            }));
        }

        if message.connection_type != TRANSPORT_CONNECTION_TYPE_FORWARD {
            warn!(
                "reject transport bind, session_id:{}, addr:{}, connection_id:{}, unsupported_connection_type:{}",
                self.session_id(),
                self.addr(),
                message.connection_id,
                message.connection_type
            );
            return Ok(MessageType::GenericError(generic::Error {
                number: -3,
                message: "unsupported transport connection type".into(),
            }));
        }

        let Some(player) = GLOBAL_MANAGER
            .player_manager
            .get_player_by_transport_token(&message.transport_token)
            .await
        else {
            warn!(
                "reject transport bind, session_id:{}, addr:{}, connection_id:{}, reason:invalid token",
                self.session_id(),
                self.addr(),
                message.connection_id
            );
            return Ok(MessageType::GenericError(generic::Error {
                number: -2,
                message: "invalid transport token".into(),
            }));
        };

        let connection_id = if message.connection_id == 0 {
            u64::from(self.session_id())
        } else {
            message.connection_id
        };

        {
            let mut p = player.write().await;
            debug!(
                "binding forward transport, player_id:{}, session_id:{}, connection_id:{}, addr:{}",
                p.get_player_id(),
                self.session_id(),
                connection_id,
                self.addr()
            );
            p.add_forward_connection(
                connection_id,
                self.session_id(),
                self.tx().ok_or_else(|| anyhow::anyhow!("tx is none"))?,
                &self.addr(),
            )?;
            let (rx, tx) = p.clone_traffic_counters();
            self.traffic_rx = Some(rx);
            self.traffic_tx = Some(tx);
        }

        self.player = Some(player.clone());
        self.mark_forward_connection(connection_id);

        let player_id = player.read().await.get_player_id();
        info!(
            "transport bind successful, player_id:{}, session_id:{}, connection_id:{}, addr:{}",
            player_id,
            self.session_id(),
            connection_id,
            self.addr()
        );
        Ok(MessageType::ServerClientBindTransportAck(
            server_client::BindTransportAck {
                player_id,
                connection_id,
                transport_idle_timeout_secs: GLOBAL_CONFIG.transport_idle_timeout_secs,
            },
        ))
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

fn negotiate_transport_max_connections(client_requested: u32, server_limit: u32) -> u32 {
    if client_requested == 0 || server_limit == 0 {
        0
    } else {
        client_requested.min(server_limit)
    }
}

fn negotiate_transport_idle_timeout_secs(client_requested: u32, server_value: u32) -> u32 {
    if client_requested == 0 {
        server_value
    } else if server_value == 0 {
        client_requested
    } else {
        client_requested.min(server_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_transport_max_connections_keeps_legacy_when_either_side_is_zero() {
        assert_eq!(negotiate_transport_max_connections(0, 8), 0);
        assert_eq!(negotiate_transport_max_connections(8, 0), 0);
    }

    #[test]
    fn negotiate_transport_max_connections_uses_lower_non_zero_limit() {
        assert_eq!(negotiate_transport_max_connections(8, 4), 4);
        assert_eq!(negotiate_transport_max_connections(4, 8), 4);
    }

    #[test]
    fn negotiate_transport_idle_timeout_uses_non_zero_or_lower_value() {
        assert_eq!(negotiate_transport_idle_timeout_secs(0, 60), 60);
        assert_eq!(negotiate_transport_idle_timeout_secs(30, 0), 30);
        assert_eq!(negotiate_transport_idle_timeout_secs(30, 60), 30);
        assert_eq!(negotiate_transport_idle_timeout_secs(90, 60), 60);
    }
}
