use crate::peer::package_and_send_message;
use chrono::Utc;
use log::{debug, info, trace};
use np_base::net::WriterMessage;
use np_proto::generic;
use np_proto::message_map::MessageType;
use np_proto::utils::message_bridge;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

pub type PlayerId = u32;

/// 在线用户名下的一条已认证转发路径。
///
/// 这条路径可以是 TCP/KCP/WS 的物理连接，也可以是 QUIC 的逻辑流。
/// 控制连接单独存储在 `Player` 里，本结构只用于代理流量转发路径。
#[allow(dead_code)]
pub struct PlayerConnection {
    /// 该转发路径对应的服务端 `net_session` 会话 ID。
    pub session_id: u32,
    /// 该转发路径的写消息通道，用于向客户端连接或 QUIC 流发送数据。
    pub tx: UnboundedSender<WriterMessage>,
    /// 该转发路径的远端地址。
    pub addr: String,
    /// 建立时间（Unix 秒）。
    pub connected_at: i64,
    /// 最近一次使用时间（Unix 秒）。
    pub last_active_at: i64,
    /// 当前绑定到该路径上的代理会话数量。
    pub active_sessions: usize,
    /// 当前正在该路径上写出但尚未完成的字节数。
    pub inflight_bytes: u64,
}

/// 用户在服务端的运行时状态。
///
/// `Player` 表示账号级在线状态。它拥有一条控制连接，以及新传输架构使用的可选转发连接池。
pub struct Player {
    /// 控制连接的写通道，单连接模式也继续使用它承载全部消息。
    tx: Option<UnboundedSender<WriterMessage>>,
    /// 当前在线用户的玩家 ID。
    player_id: PlayerId,
    /// 控制连接对应的服务端会话 ID，0 表示当前不在线。
    session_id: u32,
    /// 控制连接的远端地址。
    addr: String,
    /// 控制连接上线时间（Unix 时间戳，秒）。
    online_time: i64,
    /// 当前控制连接使用的接入协议，例如 tcp/kcp/ws/quic。
    connection_protocol: String,
    /// 入站流量计数器，从客户端接收的数据量，`Peer` 可通过共享引用无锁累加。
    pub traffic_rx: Arc<AtomicU64>,
    /// 出站流量计数器，发送给客户端的数据量，`Peer` 可通过共享引用无锁累加。
    pub traffic_tx: Arc<AtomicU64>,
    /// 通过临时令牌绑定后的转发连接池，键为客户端生成的连接 ID。
    forward_connections: HashMap<u64, PlayerConnection>,
    /// 代理会话到转发连接的路由表，键为代理 `session_id`，值为连接 ID。
    forward_session_routes: HashMap<u32, u64>,
    /// 控制连接登录成功后生成的临时令牌，用于后续转发连接快速绑定；控制连接断开时清理。
    transport_token: String,
    /// 服务端与客户端协商后的最大转发连接数量，0 表示关闭多连接传输模式并保持单连接行为。
    transport_max_connections: u32,
    /// 转发连接空闲关闭时间（秒），超过该时间未使用的转发路径可以被关闭。
    transport_idle_timeout_secs: u32,
}

impl Player {
    pub fn new(player_id: PlayerId) -> Arc<RwLock<Player>> {
        Arc::new(RwLock::new(Player {
            tx: None,
            player_id,
            session_id: 0,
            addr: String::new(),
            online_time: 0,
            connection_protocol: String::new(),
            traffic_rx: Arc::new(AtomicU64::new(0)),
            traffic_tx: Arc::new(AtomicU64::new(0)),
            forward_connections: HashMap::new(),
            forward_session_routes: HashMap::new(),
            transport_token: String::new(),
            transport_max_connections: 0,
            transport_idle_timeout_secs: 0,
        }))
    }

    /// 获取并重置流量计数，返回 (rx, tx)
    pub fn take_traffic(&self) -> (u64, u64) {
        (
            self.traffic_rx.swap(0, Ordering::Relaxed),
            self.traffic_tx.swap(0, Ordering::Relaxed),
        )
    }

    /// 将已取出的流量加回计数器，用于刷库失败后的补偿
    pub fn add_traffic(&self, rx: u64, tx: u64) {
        self.traffic_rx.fetch_add(rx, Ordering::Relaxed);
        self.traffic_tx.fetch_add(tx, Ordering::Relaxed);
    }

    /// 获取当前流量（不重置）
    pub fn get_traffic(&self) -> (u64, u64) {
        (
            self.traffic_rx.load(Ordering::Relaxed),
            self.traffic_tx.load(Ordering::Relaxed),
        )
    }

    /// 克隆流量计数器引用，供 Peer 无锁访问
    pub fn clone_traffic_counters(&self) -> (Arc<AtomicU64>, Arc<AtomicU64>) {
        (self.traffic_rx.clone(), self.traffic_tx.clone())
    }

    // 获取玩家Id
    #[inline]
    pub fn get_player_id(&self) -> PlayerId {
        self.player_id
    }

    // 获取会话id
    #[inline]
    pub fn get_session_id(&self) -> u32 {
        self.session_id
    }

    // 获取IP地址
    #[inline]
    pub fn get_addr(&self) -> &str {
        &self.addr
    }

    // 获取上线时间
    #[inline]
    pub fn get_online_time(&self) -> i64 {
        self.online_time
    }

    // 获取当前连接协议
    #[inline]
    pub fn get_connection_protocol(&self) -> &str {
        &self.connection_protocol
    }

    // 是否在线
    #[inline]
    pub fn is_online(&self) -> bool {
        self.session_id > 0
    }

    pub fn is_valid_transport_token(&self, token: &str) -> bool {
        !token.is_empty()
            && self.is_online()
            && self.transport_max_connections > 0
            && self.transport_token == token
    }

    pub fn configure_transport(&mut self, max_connections: u32, idle_timeout_secs: u32) -> String {
        self.transport_max_connections = max_connections;
        self.transport_idle_timeout_secs = idle_timeout_secs;
        self.forward_connections.clear();
        self.forward_session_routes.clear();

        if max_connections == 0 {
            self.transport_token.clear();
            info!(
                "transport pool disabled, player_id:{}, idle_timeout_secs:{}",
                self.player_id, idle_timeout_secs
            );
            return String::new();
        }

        self.transport_token = generate_transport_token();
        info!(
            "transport pool configured, player_id:{}, max_connections:{}, idle_timeout_secs:{}",
            self.player_id, max_connections, idle_timeout_secs
        );
        self.transport_token.clone()
    }

    pub fn add_forward_connection(
        &mut self,
        connection_id: u64,
        session_id: u32,
        tx: UnboundedSender<WriterMessage>,
        addr: &SocketAddr,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_online(), "control connection is offline");
        anyhow::ensure!(
            self.transport_max_connections > 0,
            "transport pool is disabled"
        );
        anyhow::ensure!(
            self.forward_connections.len() < self.transport_max_connections as usize,
            "too many forward connections"
        );
        anyhow::ensure!(
            !self.forward_connections.contains_key(&connection_id),
            "duplicate forward connection id"
        );

        let now = Utc::now().timestamp();
        self.forward_connections.insert(
            connection_id,
            PlayerConnection {
                session_id,
                tx,
                addr: addr.to_string(),
                connected_at: now,
                last_active_at: now,
                active_sessions: 0,
                inflight_bytes: 0,
            },
        );
        info!(
            "forward transport connected, player_id:{}, connection_id:{}, session_id:{}, addr:{}, current_connections:{}/{}",
            self.player_id,
            connection_id,
            session_id,
            addr,
            self.forward_connections.len(),
            self.transport_max_connections
        );
        Ok(())
    }

    pub fn remove_forward_connection(&mut self, connection_id: u64) {
        if self.forward_connections.remove(&connection_id).is_some() {
            info!(
                "forward transport disconnected, player_id:{}, connection_id:{}, remaining_connections:{}",
                self.player_id,
                connection_id,
                self.forward_connections.len()
            );
        }
        self.forward_session_routes
            .retain(|_, route_connection_id| *route_connection_id != connection_id);
    }

    pub fn close_idle_forward_connections(&mut self, now: i64) {
        if self.transport_idle_timeout_secs == 0 {
            return;
        }

        let idle_timeout_secs = i64::from(self.transport_idle_timeout_secs);
        let connection_ids = self
            .forward_connections
            .iter()
            .filter(|(_, connection)| {
                connection.active_sessions == 0
                    && now.saturating_sub(connection.last_active_at) >= idle_timeout_secs
            })
            .map(|(connection_id, _)| *connection_id)
            .collect::<Vec<_>>();

        for connection_id in connection_ids {
            if let Some(connection) = self.forward_connections.remove(&connection_id) {
                self.forward_session_routes
                    .retain(|_, route_connection_id| *route_connection_id != connection_id);
                info!(
                    "close idle forward transport, player_id:{}, connection_id:{}, idle_secs:{}, active_sessions:{}",
                    self.player_id,
                    connection_id,
                    now.saturating_sub(connection.last_active_at),
                    connection.active_sessions
                );
                let _ = connection.tx.send(WriterMessage::Close);
            }
        }
    }

    pub fn bind_forward_session(&mut self, session_id: u32, connection_id: u64) {
        if !self.forward_connections.contains_key(&connection_id) {
            debug!(
                "ignore forward session bind, player_id:{}, session_id:{}, missing_connection_id:{}",
                self.player_id, session_id, connection_id
            );
            return;
        }

        if self
            .forward_session_routes
            .get(&session_id)
            .copied()
            .is_some_and(|old_connection_id| old_connection_id == connection_id)
        {
            if let Some(connection) = self.forward_connections.get_mut(&connection_id) {
                connection.last_active_at = Utc::now().timestamp();
            }
            return;
        }

        if let Some(old_connection_id) = self
            .forward_session_routes
            .insert(session_id, connection_id)
        {
            if let Some(connection) = self.forward_connections.get_mut(&old_connection_id) {
                connection.active_sessions = connection.active_sessions.saturating_sub(1);
            }
        }

        if let Some(connection) = self.forward_connections.get_mut(&connection_id) {
            connection.active_sessions += 1;
            connection.last_active_at = Utc::now().timestamp();
            debug!(
                "forward session bound, player_id:{}, proxy_session_id:{}, connection_id:{}, active_sessions:{}",
                self.player_id, session_id, connection_id, connection.active_sessions
            );
        }
    }

    pub fn unbind_forward_session(&mut self, session_id: u32) {
        if let Some(connection_id) = self.forward_session_routes.remove(&session_id) {
            if let Some(connection) = self.forward_connections.get_mut(&connection_id) {
                connection.active_sessions = connection.active_sessions.saturating_sub(1);
                connection.last_active_at = Utc::now().timestamp();
                debug!(
                    "forward session unbound, player_id:{}, proxy_session_id:{}, connection_id:{}, active_sessions:{}",
                    self.player_id, session_id, connection_id, connection.active_sessions
                );
            }
        }
    }

    pub fn send_proxy_push(&mut self, message: &MessageType) -> anyhow::Result<()> {
        let Some(session_id) = message_bridge::pb_proxy_session_id(message) else {
            return self.send_push(message);
        };

        let mut connection_id = self.forward_session_routes.get(&session_id).copied();
        if connection_id
            .and_then(|id| self.forward_connections.get(&id))
            .is_none()
        {
            self.forward_session_routes.remove(&session_id);
            connection_id = self.select_least_loaded_forward_connection_id();
            if let Some(connection_id) = connection_id {
                self.bind_forward_session(session_id, connection_id);
            }
        }

        let result = if let Some(connection_id) = connection_id {
            if let Some(connection) = self.forward_connections.get_mut(&connection_id) {
                connection.last_active_at = Utc::now().timestamp();
                package_and_send_message(&Some(connection.tx.clone()), 0, message, true)
            } else {
                self.send_push(message)
            }
        } else {
            self.send_push(message)
        };

        if message_bridge::pb_proxy_is_disconnect(message) {
            self.unbind_forward_session(session_id);
        }

        result
    }

    fn select_least_loaded_forward_connection_id(&self) -> Option<u64> {
        self.forward_connections
            .iter()
            .min_by_key(|(_, connection)| {
                (
                    connection.active_sessions,
                    connection.inflight_bytes,
                    connection.last_active_at,
                )
            })
            .map(|(connection_id, _)| *connection_id)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn send_response(&self, serial: i32, message: &MessageType) -> anyhow::Result<()> {
        assert!(serial < 0);
        package_and_send_message(&self.tx, -serial, message, true)
    }

    // #[inline]
    // pub fn send_request(&self, _message: &MessageType) -> anyhow::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    #[allow(dead_code)]
    pub fn send_push(&self, message: &MessageType) -> anyhow::Result<()> {
        package_and_send_message(&self.tx, 0, message, true)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn flush(&self) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Flush);
        }
    }

    #[inline]
    pub fn close_session(&mut self) {
        trace!("close_session, player_id: {}", self.player_id);
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Close);
        }
    }

    // 重置会话信息
    #[inline]
    fn reset_session_info(&mut self) {
        trace!("reset_session_info, player_id: {}", self.player_id);
        for (_, connection) in self.forward_connections.drain() {
            debug!(
                "close forward transport during session reset, player_id:{}, session_id:{}",
                self.player_id, connection.session_id
            );
            let _ = connection.tx.send(WriterMessage::Close);
        }
        self.forward_session_routes.clear();
        self.session_id = 0;
        self.tx.take();
        self.addr.clear();
        self.online_time = 0;
        self.connection_protocol.clear();
        self.transport_token.clear();
        self.transport_max_connections = 0;
        self.transport_idle_timeout_secs = 0;
    }

    // 玩家上线
    pub fn on_connect_session(
        &mut self,
        session_id: u32,
        tx: UnboundedSender<WriterMessage>,
        addr: &SocketAddr,
        connection_protocol: &str,
    ) {
        trace!("on_connect_session, player_id: {}", self.player_id);
        assert!(!self.is_online());
        self.session_id = session_id;
        self.tx = Some(tx);
        self.addr = addr.to_string();
        self.online_time = Utc::now().timestamp();
        self.connection_protocol = connection_protocol.to_string();
    }

    // 玩家离线
    #[allow(dead_code)]
    pub fn on_disconnect_session(&mut self) {
        trace!("on_disconnect_session, player_id: {}", self.player_id);
        self.reset_session_info();
    }

    // 玩家被顶号，需要对旧的会话发送一些消息
    pub fn on_terminate_old_session(&mut self) {
        trace!("on_terminate_old_session, player_id: {}", self.player_id);
        self.close_session();

        // 重置会话信息
        self.reset_session_info();
    }

    // 管理员主动将玩家踢下线
    pub fn kick_offline(&mut self) {
        trace!("kick_offline, player_id: {}", self.player_id);
        self.close_session();
        self.reset_session_info();
    }

    // 玩家收到消息
    pub async fn handle_request(&mut self, _message: MessageType) -> anyhow::Result<MessageType> {
        // 客户端请求的消息，服务器未实现
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::InterfaceAbsent.into(),
            message: "interface absent".into(),
        }))
    }
}

fn generate_transport_token() -> String {
    format!(
        "{:032x}{:032x}",
        rand::random::<u128>(),
        rand::random::<u128>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use np_base::net::WriterMessage;
    use std::net::SocketAddr;
    use tokio::sync::mpsc::unbounded_channel;

    fn online_player(max_connections: u32, idle_timeout_secs: u32) -> Player {
        let (tx, _) = unbounded_channel();
        Player {
            tx: Some(tx),
            player_id: 1,
            session_id: 10,
            addr: "127.0.0.1:10000".to_string(),
            online_time: Utc::now().timestamp(),
            connection_protocol: "tcp".to_string(),
            traffic_rx: Arc::new(AtomicU64::new(0)),
            traffic_tx: Arc::new(AtomicU64::new(0)),
            forward_connections: HashMap::new(),
            forward_session_routes: HashMap::new(),
            transport_token: String::new(),
            transport_max_connections: max_connections,
            transport_idle_timeout_secs: idle_timeout_secs,
        }
    }

    fn add_forward_connection(player: &mut Player, connection_id: u64) {
        let (tx, _) = unbounded_channel();
        let addr = SocketAddr::from(([127, 0, 0, 1], 10000 + connection_id as u16));
        player
            .add_forward_connection(connection_id, 100 + connection_id as u32, tx, &addr)
            .unwrap();
    }

    #[test]
    fn configure_transport_keeps_legacy_mode_when_max_is_zero() {
        let mut player = online_player(0, 0);

        let token = player.configure_transport(0, 60);

        assert!(token.is_empty());
        assert!(player.transport_token.is_empty());
        assert_eq!(player.transport_max_connections, 0);
        assert!(!player.is_valid_transport_token("any-token"));
    }

    #[test]
    fn configure_transport_generates_valid_token_for_online_player() {
        let mut player = online_player(0, 0);

        let token = player.configure_transport(2, 60);

        assert!(!token.is_empty());
        assert_eq!(player.transport_token, token);
        assert_eq!(player.transport_max_connections, 2);
        assert!(player.is_valid_transport_token(&token));
    }

    #[test]
    fn add_forward_connection_enforces_pool_limit() {
        let mut player = online_player(2, 60);

        add_forward_connection(&mut player, 1);
        add_forward_connection(&mut player, 2);

        let (tx, _) = unbounded_channel();
        let addr = SocketAddr::from(([127, 0, 0, 1], 10003));
        let result = player.add_forward_connection(3, 103, tx, &addr);

        assert!(result.is_err());
        assert_eq!(player.forward_connections.len(), 2);
    }

    #[test]
    fn forward_session_route_moves_between_connections() {
        let mut player = online_player(2, 60);
        add_forward_connection(&mut player, 1);
        add_forward_connection(&mut player, 2);

        player.bind_forward_session(55, 1);
        player.bind_forward_session(55, 2);

        assert_eq!(player.forward_session_routes.get(&55), Some(&2));
        assert_eq!(
            player.forward_connections.get(&1).unwrap().active_sessions,
            0
        );
        assert_eq!(
            player.forward_connections.get(&2).unwrap().active_sessions,
            1
        );

        player.unbind_forward_session(55);

        assert!(!player.forward_session_routes.contains_key(&55));
        assert_eq!(
            player.forward_connections.get(&2).unwrap().active_sessions,
            0
        );
    }

    #[test]
    fn remove_forward_connection_clears_bound_routes() {
        let mut player = online_player(2, 60);
        add_forward_connection(&mut player, 1);

        player.bind_forward_session(55, 1);
        player.remove_forward_connection(1);

        assert!(!player.forward_connections.contains_key(&1));
        assert!(!player.forward_session_routes.contains_key(&55));
    }

    #[test]
    fn close_idle_forward_connections_only_closes_inactive_paths() {
        let mut player = online_player(2, 10);
        let (idle_tx, mut idle_rx) = unbounded_channel();
        let (active_tx, mut active_rx) = unbounded_channel();
        let now = Utc::now().timestamp();

        player.forward_connections.insert(
            1,
            PlayerConnection {
                session_id: 101,
                tx: idle_tx,
                addr: "127.0.0.1:10001".to_string(),
                connected_at: now - 30,
                last_active_at: now - 30,
                active_sessions: 0,
                inflight_bytes: 0,
            },
        );
        player.forward_connections.insert(
            2,
            PlayerConnection {
                session_id: 102,
                tx: active_tx,
                addr: "127.0.0.1:10002".to_string(),
                connected_at: now - 30,
                last_active_at: now - 30,
                active_sessions: 1,
                inflight_bytes: 0,
            },
        );

        player.close_idle_forward_connections(now);

        assert!(!player.forward_connections.contains_key(&1));
        assert!(player.forward_connections.contains_key(&2));
        assert!(matches!(idle_rx.try_recv(), Ok(WriterMessage::Close)));
        assert!(active_rx.try_recv().is_err());
    }

    #[test]
    fn least_loaded_forward_connection_prefers_fewer_sessions() {
        let mut player = online_player(2, 60);
        add_forward_connection(&mut player, 1);
        add_forward_connection(&mut player, 2);

        player.bind_forward_session(55, 1);

        assert_eq!(player.select_least_loaded_forward_connection_id(), Some(2));
    }
}
