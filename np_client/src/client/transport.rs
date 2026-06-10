//! 客户端统一传输层。
//!
//! 所有协议（TCP、KCP、WS、QUIC）共享同一套连接池逻辑。
//! 当 `max_forward_paths == 0` 时不创建转发连接，所有代理消息回退到控制连接——
//! 等价于传统单连接模式。

use bytes::Bytes;
use dashmap::DashMap;
use log::{debug, info, warn};
use np_proto::client_server::BindTransportReq;
use np_proto::message_map::{get_message_size, MessageType};
use np_proto::utils::message_bridge;
use np_proto::utils::transport::TRANSPORT_CONNECTION_TYPE_FORWARD;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, WriteHalf};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

use super::io::{package_and_send_message, read_transport_events};
use super::now_secs;

// ─── 事件类型 ──────────────────────────────────────────────────────────────────

/// 从任意传输路径收到的一帧协议消息。
///
/// `path_id = None` 表示来自控制连接；`Some(id)` 表示来自转发路径。
pub struct IncomingFrame {
    pub path_id: Option<u64>,
    pub frame: Bytes,
}

/// 传输层读取任务发送给客户端会话的事件。
pub enum TransportEvent {
    /// 收到一帧完整的协议消息。
    Frame(IncomingFrame),
    /// 某条传输路径关闭或读取出错。
    Closed {
        path_id: Option<u64>,
        reason: String,
    },
}

// ─── 类型别名 ──────────────────────────────────────────────────────────────────

/// 创建新转发连接/流的工厂闭包。
///
/// 这是依赖注入点：TCP/KCP/WS 提供建立新连接的闭包；
/// QUIC 提供在同一连接上打开新双向流的闭包。
pub type ConnectFuture<S> = Pin<Box<dyn Future<Output = anyhow::Result<S>> + Send>>;
pub type ForwardConnector<S> = Arc<dyn Fn() -> ConnectFuture<S> + Send + Sync>;

// ─── PooledForwardPath ─────────────────────────────────────────────────────────

/// 一条转发路径（物理连接或 QUIC 流）。
pub struct PooledForwardPath<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 客户端分配的唯一连接 ID，上报给服务端用于绑定。
    pub connection_id: u64,
    /// 该路径的写半边，用于发送代理数据。
    pub writer: Arc<Mutex<WriteHalf<S>>>,
    /// 绑定到该路径的活跃代理会话数。
    pub active_sessions: AtomicUsize,
    /// 正在发送但未完成的字节数，用于负载均衡。
    pub inflight_bytes: AtomicUsize,
    /// 最后一次使用时间（Unix 秒），用于空闲超时判断。
    pub last_used_secs: AtomicU64,
}

impl<S> PooledForwardPath<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 计算负载评分，用于最小负载选路。
    /// 主维度：活跃会话数；次维度：在途字节数。
    fn load_score(&self) -> usize {
        self.active_sessions.load(Ordering::Relaxed) * 1_000_000
            + self.inflight_bytes.load(Ordering::Relaxed)
    }

    fn bind_session(&self) {
        self.active_sessions.fetch_add(1, Ordering::Relaxed);
        self.last_used_secs.store(now_secs(), Ordering::Relaxed);
    }

    fn unbind_session(&self) {
        let _ = self
            .active_sessions
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            });
        self.last_used_secs.store(now_secs(), Ordering::Relaxed);
    }
}

// ─── PooledTransportState ──────────────────────────────────────────────────────

/// 连接池传输层的共享状态。
///
/// 控制连接独立管理（不进池、不回收）。
/// 转发路径按需创建、空闲回收。
pub struct PooledTransportState<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 控制连接的写半边（登录、心跳、隧道管理、代理回退）。
    pub control_writer: Arc<Mutex<WriteHalf<S>>>,
    /// 创建新转发连接/流的工厂闭包。
    pub connector: ForwardConnector<S>,
    /// 所有读取任务共享的事件通道发送端。
    pub event_tx: mpsc::UnboundedSender<TransportEvent>,
    /// 最近一次读写活动时间（Unix 秒），供心跳超时判断。
    pub last_active_secs: Arc<AtomicU64>,
    /// 最近一次收到数据时间（Unix 秒），供硬超时判断。
    pub last_read_secs: Arc<AtomicU64>,
    /// 登录后服务端下发的临时令牌，转发连接凭此绑定。
    pub token: Mutex<String>,
    /// 服务端协商后的最大转发路径数，0 = 禁用连接池。
    pub max_forward_paths: AtomicU32,
    /// 最小保持连接数（客户端策略，不上报服务端）。
    pub min_forward_paths: AtomicU32,
    /// 转发路径空闲超时时间（秒）。
    pub idle_timeout_secs: AtomicU32,
    /// 创建新转发路径时的互斥锁，防止并发超出上限。
    pub forward_path_create_lock: Mutex<()>,
    /// 单调递增的连接 ID 计数器。
    pub next_connection_id: AtomicU64,
    /// 代理 session_id → 转发路径的绑定表。
    pub session_paths: DashMap<u32, Arc<PooledForwardPath<S>>>,
    /// 连接 ID → 转发路径的索引表。
    pub forward_paths: DashMap<u64, Arc<PooledForwardPath<S>>>,
}

impl<S> PooledTransportState<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 根据服务端登录响应配置传输参数。
    pub async fn configure_from_login(
        &self,
        token: String,
        max_forward_paths: u32,
        idle_timeout_secs: u32,
    ) {
        self.max_forward_paths
            .store(max_forward_paths, Ordering::Relaxed);
        self.idle_timeout_secs
            .store(idle_timeout_secs, Ordering::Relaxed);
        // 将 min 限制在协商后的 max 范围内
        let current_min = self.min_forward_paths.load(Ordering::Relaxed);
        if current_min > max_forward_paths {
            self.min_forward_paths
                .store(max_forward_paths, Ordering::Relaxed);
        }
        let mut guard = self.token.lock().await;
        *guard = token;
        info!(
            "transport pool configured: max_forward_paths={}, min_forward_paths={}, idle_timeout_secs={}",
            max_forward_paths,
            self.min_forward_paths.load(Ordering::Relaxed),
            idle_timeout_secs,
        );
    }

    /// 通过控制连接发送控制消息（登录、心跳、隧道管理）。
    pub async fn send_control_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        package_and_send_message(self.control_writer.clone(), serial, message).await
    }

    /// 发送代理消息，优先路由到转发路径。
    ///
    /// 以下情况回退到控制连接：
    /// - 连接池禁用（max == 0）
    /// - 令牌为空（尚未登录）
    /// - 消息无 session_id（非代理消息）
    pub async fn send_proxy_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        let Some(session_id) = message_bridge::pb_proxy_session_id(message) else {
            return self.send_control_message(serial, message).await;
        };

        let Some(path) = self.get_or_create_forward_path(session_id).await? else {
            return self.send_control_message(serial, message).await;
        };

        let message_size = get_message_size(message) + 13;
        path.inflight_bytes
            .fetch_add(message_size, Ordering::Relaxed);
        let result = package_and_send_message(path.writer.clone(), serial, message).await;
        path.inflight_bytes
            .fetch_sub(message_size, Ordering::Relaxed);
        path.last_used_secs.store(now_secs(), Ordering::Relaxed);

        if message_bridge::pb_proxy_is_disconnect(message) {
            self.unbind_session_path(session_id);
        }

        result
    }

    /// 将收到的消息绑定到其来源路径，用于回复路由。
    pub fn bind_incoming_message_path(&self, message: &MessageType, path_id: Option<u64>) {
        let (Some(path_id), Some(session_id)) =
            (path_id, message_bridge::pb_proxy_session_id(message))
        else {
            return;
        };

        if let Some(path) = self.forward_paths.get(&path_id).map(|p| p.clone()) {
            if self
                .session_paths
                .insert(session_id, path.clone())
                .is_none()
            {
                path.bind_session();
            }
            if message_bridge::pb_proxy_is_disconnect(message) {
                self.unbind_session_path(session_id);
            }
        }
    }

    /// 预创建转发路径到 `min_forward_paths` 数量，减少首次请求延迟。
    pub async fn warm_up(&self) {
        let min = self.min_forward_paths.load(Ordering::Relaxed) as usize;
        if min == 0 {
            return;
        }

        let token = self.token.lock().await.clone();
        if token.is_empty() {
            debug!("warm-up skipped: token is empty");
            return;
        }

        let _guard = self.forward_path_create_lock.lock().await;
        while self.forward_paths.len() < min {
            match self.open_forward_path(token.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("warm-up failed: {}", e);
                    break;
                }
            }
        }
        info!(
            "warm-up complete: {} forward paths ready",
            self.forward_paths.len()
        );
    }

    /// 移除指定转发路径（路径关闭或出错时调用）。
    pub async fn remove_forward_path(&self, connection_id: u64) {
        self.session_paths
            .retain(|_, path| path.connection_id != connection_id);
        if let Some((_, path)) = self.forward_paths.remove(&connection_id) {
            info!(
                "forward path removed: connection_id={}, remaining={}",
                connection_id,
                self.forward_paths.len()
            );
            let _ = path.writer.lock().await.shutdown().await;
        }
    }

    /// 启动后台任务，定期清理空闲转发路径。
    pub fn start_idle_cleanup(state: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                if Arc::strong_count(&state) <= 1 {
                    break;
                }
                state.close_idle_forward_paths().await;
            }
        });
    }

    // ─── 私有方法 ──────────────────────────────────────────────────────────────

    async fn get_or_create_forward_path(
        &self,
        session_id: u32,
    ) -> anyhow::Result<Option<Arc<PooledForwardPath<S>>>> {
        let max_paths = self.max_forward_paths.load(Ordering::Relaxed);
        if max_paths == 0 {
            debug!(
                "pool disabled, session_id={} using control connection",
                session_id
            );
            return Ok(None);
        }

        let token = self.token.lock().await.clone();
        if token.is_empty() {
            debug!(
                "token is empty, session_id={} using control connection",
                session_id
            );
            return Ok(None);
        }

        // 快速路径：会话已绑定
        if let Some(path) = self.session_paths.get(&session_id).map(|p| p.clone()) {
            return Ok(Some(path));
        }

        // 加锁序列化创建，防止超出上限
        let _guard = self.forward_path_create_lock.lock().await;

        // 获取锁后再次检查
        if let Some(path) = self.session_paths.get(&session_id).map(|p| p.clone()) {
            return Ok(Some(path));
        }

        let path = if self.forward_paths.len() < max_paths as usize {
            self.open_forward_path(token).await?
        } else {
            debug!(
                "pool full: session_id={}, current={}, max={}",
                session_id,
                self.forward_paths.len(),
                max_paths
            );
            self.select_least_loaded_path()
                .ok_or_else(|| anyhow::anyhow!("no available forward path"))?
        };

        if self
            .session_paths
            .insert(session_id, path.clone())
            .is_none()
        {
            path.bind_session();
        }
        Ok(Some(path))
    }

    async fn open_forward_path(&self, token: String) -> anyhow::Result<Arc<PooledForwardPath<S>>> {
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        info!("opening forward path: connection_id={}", connection_id);

        let stream = (self.connector)().await?;
        let (reader, writer) = tokio::io::split(stream);
        let writer = Arc::new(Mutex::new(writer));

        // 立即发送绑定请求——无需等待 ACK，因为同一连接上帧有序。
        package_and_send_message(
            writer.clone(),
            -3,
            &MessageType::ClientServerBindTransportReq(BindTransportReq {
                transport_token: token,
                connection_id,
                connection_type: TRANSPORT_CONNECTION_TYPE_FORWARD,
            }),
        )
        .await?;

        let path = Arc::new(PooledForwardPath {
            connection_id,
            writer,
            active_sessions: AtomicUsize::new(0),
            inflight_bytes: AtomicUsize::new(0),
            last_used_secs: AtomicU64::new(now_secs()),
        });

        self.forward_paths.insert(connection_id, path.clone());
        debug!(
            "forward path opened: connection_id={}, total={}",
            connection_id,
            self.forward_paths.len()
        );

        // 为该路径启动读取任务
        tokio::spawn(read_transport_events(
            reader,
            Some(connection_id),
            self.event_tx.clone(),
            self.last_active_secs.clone(),
            self.last_read_secs.clone(),
        ));

        Ok(path)
    }

    fn select_least_loaded_path(&self) -> Option<Arc<PooledForwardPath<S>>> {
        self.forward_paths
            .iter()
            .map(|entry| entry.value().clone())
            .min_by_key(|path| (path.load_score(), path.connection_id))
    }

    fn unbind_session_path(&self, session_id: u32) {
        if let Some((_, path)) = self.session_paths.remove(&session_id) {
            path.unbind_session();
        }
    }

    async fn close_idle_forward_paths(&self) {
        let idle_timeout_secs = self.idle_timeout_secs.load(Ordering::Relaxed);
        if idle_timeout_secs == 0 {
            return;
        }

        let min_paths = self.min_forward_paths.load(Ordering::Relaxed) as usize;
        if self.forward_paths.len() <= min_paths {
            return;
        }

        let now = now_secs();
        let removable_count = self.forward_paths.len().saturating_sub(min_paths);

        let mut idle_ids: Vec<u64> = self
            .forward_paths
            .iter()
            .filter(|entry| {
                entry.value().active_sessions.load(Ordering::Relaxed) == 0
                    && now.saturating_sub(entry.value().last_used_secs.load(Ordering::Relaxed))
                        >= u64::from(idle_timeout_secs)
            })
            .map(|entry| *entry.key())
            .collect();

        // 最多清理 removable_count 条，保证不低于 min_forward_paths
        idle_ids.truncate(removable_count);

        for connection_id in idle_ids {
            if let Some((_, path)) = self.forward_paths.remove(&connection_id) {
                self.session_paths
                    .retain(|_, p| p.connection_id != connection_id);
                info!(
                    "idle forward path closed: connection_id={}, remaining={}",
                    connection_id,
                    self.forward_paths.len()
                );
                let _ = path.writer.lock().await.shutdown().await;
            }
        }
    }
}

// ─── ClientTransport ───────────────────────────────────────────────────────────

/// `PooledTransportState` 的薄封装，对外提供客户端会话使用的公共 API。
///
/// 客户端会话仅与此类型交互——它透明地处理单连接（池禁用）和多连接（池启用）两种模式。
pub struct ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    state: Arc<PooledTransportState<S>>,
}

impl<S> Clone for ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<S> ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 创建新的传输实例。
    pub fn new(
        control_writer: Arc<Mutex<WriteHalf<S>>>,
        connector: ForwardConnector<S>,
        event_tx: mpsc::UnboundedSender<TransportEvent>,
        last_active_secs: Arc<AtomicU64>,
        last_read_secs: Arc<AtomicU64>,
        min_forward_paths: u32,
    ) -> Self {
        let state = Arc::new(PooledTransportState {
            control_writer,
            connector,
            event_tx,
            last_active_secs,
            last_read_secs,
            token: Mutex::new(String::new()),
            max_forward_paths: AtomicU32::new(0),
            min_forward_paths: AtomicU32::new(min_forward_paths),
            idle_timeout_secs: AtomicU32::new(0),
            forward_path_create_lock: Mutex::new(()),
            next_connection_id: AtomicU64::new(1),
            session_paths: DashMap::new(),
            forward_paths: DashMap::new(),
        });
        PooledTransportState::start_idle_cleanup(state.clone());
        Self { state }
    }

    /// 发送控制消息。
    pub async fn send_control_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        self.state.send_control_message(serial, message).await
    }

    /// 发送代理消息。
    pub async fn send_proxy_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        self.state.send_proxy_message(serial, message).await
    }

    /// 绑定收到消息的来源路径。
    pub fn bind_incoming_message_path(&self, message: &MessageType, path_id: Option<u64>) {
        self.state.bind_incoming_message_path(message, path_id);
    }

    /// 根据登录响应配置传输参数。
    pub async fn configure_from_login(
        &self,
        token: String,
        max_forward_paths: u32,
        idle_timeout_secs: u32,
    ) {
        self.state
            .configure_from_login(token, max_forward_paths, idle_timeout_secs)
            .await;
    }

    /// 预热转发路径到 min_forward_paths 数量。
    pub async fn warm_up(&self) {
        self.state.warm_up().await;
    }

    /// 移除转发路径。path_id 为 None 时无操作（控制连接不在此移除）。
    pub async fn remove_path(&self, path_id: Option<u64>) {
        if let Some(id) = path_id {
            self.state.remove_forward_path(id).await;
        }
    }
}
