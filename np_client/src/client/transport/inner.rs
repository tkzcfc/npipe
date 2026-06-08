use bytes::Bytes;
use dashmap::DashMap;
use np_base::proxy::inlet::Inlet;
use np_base::proxy::outlet::Outlet;
use np_proto::class_def::Tunnel;
use np_proto::message_map::MessageType;
#[cfg(feature = "quic")]
use s2n_quic::{
    connection::Handle as QuicConnectionHandle,
    stream::BidirectionalStream as QuicBidirectionalStream,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, WriteHalf};
use tokio::sync::{mpsc, Mutex};

use crate::client::io::package_and_send_message;

// --- 事件类型 ---

/// 从任意传输路径收到的一帧业务消息。
///
/// 单连接或控制路径使用 `path_id = None`；连接池路径使用 `Some(connection_id)`。
/// 客户端核心可据此把代理会话绑定回消息来源路径。
pub struct IncomingFrame {
    /// 消息来源路径 ID，`None` 表示控制连接。
    pub path_id: Option<u64>,
    /// 去掉外层长度头后的协议帧（零拷贝）。
    pub frame: Bytes,
}

/// 传输读取任务发给客户端核心的事件。
///
/// 读取任务和客户端核心解耦后，多条连接可以并发读取，互不阻塞。
pub enum TransportEvent {
    /// 收到一帧完整协议消息。
    Frame(IncomingFrame),
    /// 某条传输路径关闭或读失败。
    Closed {
        /// 关闭的传输路径 ID；`None` 表示控制路径。
        path_id: Option<u64>,
        /// 关闭原因或读取错误描述。
        reason: String,
    },
}

// --- 类型别名 ---

pub type ConnectFuture<S> = Pin<Box<dyn Future<Output = anyhow::Result<S>> + Send>>;
pub type ForwardConnector<S> = Arc<dyn Fn() -> ConnectFuture<S> + Send + Sync>;

// --- 结构体定义 ---

/// 客户端核心状态，负责登录、隧道同步和代理消息路由。
///
/// 不直接持有原始写入端，而是通过 `ClientTransport` 发送消息，使得单连接、
/// 连接池、QUIC 多流可以复用同一套隧道逻辑。
pub struct Client<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 当前客户端使用的传输层封装。
    pub transport: ClientTransport<S>,
    /// 登录用户名。
    pub username: String,
    /// 登录密码。
    pub password: String,
    /// 客户端期望的最大转发通道数量，0 表示单连接模式。
    pub transport_max_connections: u32,
    /// 转发通道空闲关闭时间（秒）。
    pub transport_idle_timeout_secs: u32,
    /// 登录成功后服务端分配的用户 ID，未登录时为 0。
    pub player_id: u32,
    /// 当前客户端作为出口端时启动的出口集合，key 为隧道 ID。
    pub outlets: Arc<DashMap<u32, Arc<Outlet>>>,
    /// 当前客户端作为入口端时启动的入口集合，key 为隧道 ID。
    pub inlets: Arc<DashMap<u32, Inlet>>,
    /// 服务端下发的隧道快照，用于转发消息寻址，key 为隧道 ID。
    pub tunnels: HashMap<u32, Tunnel>,
}

/// 客户端传输层门面，负责发送控制消息和代理消息。
///
/// 屏蔽底层单连接、连接池、QUIC 多流的差异，使客户端核心无需感知具体传输实现。
pub struct ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 当前启用的具体传输实现。
    pub kind: ClientTransportKind<S>,
}

/// 当前连接对应的具体传输实现。
pub enum ClientTransportKind<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 单连接模式：所有消息共用同一个写半边。
    Single {
        /// 唯一的写半边。
        writer: Arc<Mutex<WriteHalf<S>>>,
        /// 向服务端上报的期望最大转发路径数；单连接模式下不主动创建额外连接。
        max_forward_paths: u32,
        /// 向服务端上报的转发路径空闲超时（秒）；单连接模式下不主动关闭连接。
        idle_timeout_secs: u32,
    },
    /// TCP/KCP/WS 多物理连接池模式。
    Pool(Arc<PooledTransportState<S>>),
    /// QUIC 单连接多流模式。
    #[cfg(feature = "quic")]
    Quic(Arc<QuicTransportState>),
}

/// TCP/KCP/WS 多物理连接池模式的共享状态。
///
/// 控制连接负责登录、心跳和隧道变更；转发连接按需建立，
/// 并通过服务端下发的临时令牌快速绑定到具体代理会话。
pub struct PooledTransportState<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 控制连接的写半边，Legacy 回退时也用它发送代理消息。
    pub control_writer: Arc<Mutex<WriteHalf<S>>>,
    /// 建立新转发连接的工厂闭包，由具体协议的运行入口注入。
    pub connector: ForwardConnector<S>,
    /// 所有连接的读取任务汇聚到客户端核心的事件通道（发送端）。
    pub event_tx: mpsc::UnboundedSender<TransportEvent>,
    /// 最近一次读写活动时间（Unix 秒），供心跳超时判断。
    pub last_active_secs: Arc<AtomicU64>,
    /// 最近一次收到服务端数据的时间（Unix 秒），供 UDP 类传输硬超时判断。
    pub last_read_secs: Arc<AtomicU64>,
    /// 登录后服务端下发的临时绑定令牌，转发连接凭此令牌向服务端注册。
    pub token: Mutex<String>,
    /// 服务端协商后的最大转发连接数，0 表示禁用多连接模式。
    pub max_forward_paths: AtomicU32,
    /// 转发连接的空闲关闭时间（秒）。
    pub idle_timeout_secs: AtomicU32,
    /// 创建新转发连接时的互斥锁，防止并发超出上限。
    pub forward_path_create_lock: Mutex<()>,
    /// 单调递增的转发连接 ID 计数器，用于区分不同转发连接。
    pub next_connection_id: AtomicU64,
    /// 代理 session_id 到转发连接的绑定表，用于复用同一条连接。
    pub session_paths: DashMap<u32, Arc<PooledForwardPath<S>>>,
    /// 连接 ID 到转发连接的索引表，用于路径管理和空闲清理。
    pub forward_paths: DashMap<u64, Arc<PooledForwardPath<S>>>,
}

/// 一条 TCP/KCP/WS 物理转发连接。
pub struct PooledForwardPath<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 客户端生成并上报给服务端的连接 ID。
    pub connection_id: u64,
    /// 当前连接的写半边。
    pub writer: Arc<Mutex<WriteHalf<S>>>,
    /// 当前绑定到这条连接的活跃代理会话数量。
    pub active_sessions: AtomicUsize,
    /// 当前正在写出但尚未完成的字节数，用于负载均衡评分。
    pub inflight_bytes: AtomicUsize,
    /// 最近一次使用时间（Unix 秒），用于空闲超时判断。
    pub last_used_secs: AtomicU64,
}

/// QUIC 单连接多流模式的共享状态。
///
/// 控制流负责登录、心跳和隧道变更；转发流按需打开并按 session_id 绑定，
/// 确保同一代理会话的双向数据固定走同一条流，避免队头阻塞。
#[cfg(feature = "quic")]
pub struct QuicTransportState {
    /// QUIC 连接的可克隆句柄，用于按需打开新的双向流。
    pub handle: Mutex<QuicConnectionHandle>,
    /// 控制流的写半边，用于发送登录、心跳、隧道控制等消息。
    pub control_writer: Arc<Mutex<WriteHalf<QuicBidirectionalStream>>>,
    /// 所有流的读取任务汇聚到客户端核心的事件通道（发送端）。
    pub event_tx: mpsc::UnboundedSender<TransportEvent>,
    /// 最近一次读写活动时间（Unix 秒），供心跳超时判断。
    pub last_active_secs: Arc<AtomicU64>,
    /// 最近一次收到服务端数据的时间（Unix 秒），供硬超时判断。
    pub last_read_secs: Arc<AtomicU64>,
    /// 登录后服务端下发的临时绑定令牌，转发流凭此令牌向服务端注册。
    pub token: Mutex<String>,
    /// 服务端协商后的最大转发流数量，0 表示禁用多流模式。
    pub max_forward_paths: AtomicU32,
    /// 转发流的空闲关闭时间（秒）。
    pub idle_timeout_secs: AtomicU32,
    /// 创建新转发流时的互斥锁，防止并发超出上限。
    pub forward_path_create_lock: Mutex<()>,
    /// 单调递增的转发流 ID 计数器。
    pub next_connection_id: AtomicU64,
    /// 代理 session_id 到 QUIC 转发流的绑定表。
    pub session_paths: DashMap<u32, Arc<QuicForwardPath>>,
    /// 连接 ID 到 QUIC 转发流的索引表。
    pub forward_paths: DashMap<u64, Arc<QuicForwardPath>>,
}

/// 一条 QUIC 转发流。
///
/// 流数量达到上限时，新代理会话复用负载最小的流。
/// 负载由 `active_sessions` 为主、`inflight_bytes` 为辅的评分决定。
#[cfg(feature = "quic")]
pub struct QuicForwardPath {
    /// 客户端生成并上报给服务端的流 ID。
    pub connection_id: u64,
    /// 当前流的写半边。
    pub writer: Arc<Mutex<WriteHalf<QuicBidirectionalStream>>>,
    /// 当前绑定到这条流的活跃代理会话数量。
    pub active_sessions: AtomicUsize,
    /// 当前正在写出但尚未完成的字节数，用于负载均衡评分。
    pub inflight_bytes: AtomicUsize,
    /// 最近一次使用时间（Unix 秒），用于空闲超时判断。
    pub last_used_secs: AtomicU64,
}

/// QUIC 客户端握手结果，包含后续开流所需的连接句柄和第一条控制流。
#[cfg(feature = "quic")]
pub struct QuicClientConnection {
    /// QUIC 连接句柄，用于在同一连接上继续打开转发流。
    pub handle: QuicConnectionHandle,
    /// 登录、心跳和隧道控制消息使用的双向控制流。
    pub control_stream: QuicBidirectionalStream,
}

// --- ClientTransport impl ---

impl<S> Clone for ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn clone(&self) -> Self {
        let kind = match &self.kind {
            ClientTransportKind::Single {
                writer,
                max_forward_paths,
                idle_timeout_secs,
            } => ClientTransportKind::Single {
                writer: writer.clone(),
                max_forward_paths: *max_forward_paths,
                idle_timeout_secs: *idle_timeout_secs,
            },
            ClientTransportKind::Pool(state) => ClientTransportKind::Pool(state.clone()),
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => ClientTransportKind::Quic(state.clone()),
        };
        Self { kind }
    }
}

impl<S> ClientTransport<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub fn legacy(
        writer: Arc<Mutex<WriteHalf<S>>>,
        max_forward_paths: u32,
        idle_timeout_secs: u32,
    ) -> Self {
        Self {
            kind: ClientTransportKind::Single {
                writer,
                max_forward_paths,
                idle_timeout_secs,
            },
        }
    }

    pub fn pool(
        writer: Arc<Mutex<WriteHalf<S>>>,
        connector: ForwardConnector<S>,
        event_tx: mpsc::UnboundedSender<TransportEvent>,
        last_active_secs: Arc<AtomicU64>,
        last_read_secs: Arc<AtomicU64>,
    ) -> Self {
        let state = Arc::new(PooledTransportState {
            control_writer: writer,
            connector,
            event_tx,
            last_active_secs,
            last_read_secs,
            token: Mutex::new(String::new()),
            max_forward_paths: AtomicU32::new(0),
            idle_timeout_secs: AtomicU32::new(0),
            forward_path_create_lock: Mutex::new(()),
            next_connection_id: AtomicU64::new(1),
            session_paths: DashMap::new(),
            forward_paths: DashMap::new(),
        });
        PooledTransportState::start_idle_cleanup(state.clone());
        Self {
            kind: ClientTransportKind::Pool(state),
        }
    }

    pub async fn send_control_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        match &self.kind {
            ClientTransportKind::Single { writer, .. } => {
                package_and_send_message(writer.clone(), serial, message).await
            }
            ClientTransportKind::Pool(state) => state.send_control_message(serial, message).await,
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => state.send_control_message(serial, message).await,
        }
    }

    pub async fn send_proxy_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        match &self.kind {
            ClientTransportKind::Single { writer, .. } => {
                package_and_send_message(writer.clone(), serial, message).await
            }
            ClientTransportKind::Pool(state) => state.send_proxy_message(serial, message).await,
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => state.send_proxy_message(serial, message).await,
        }
    }

    pub fn bind_incoming_message_path(&self, message: &MessageType, path_id: Option<u64>) {
        match &self.kind {
            ClientTransportKind::Single { .. } => {}
            ClientTransportKind::Pool(state) => state.bind_incoming_message_path(message, path_id),
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => state.bind_incoming_message_path(message, path_id),
        }
    }

    pub async fn configure_from_login(
        &self,
        token: String,
        max_forward_paths: u32,
        idle_timeout_secs: u32,
    ) {
        match &self.kind {
            ClientTransportKind::Single { .. } => {}
            ClientTransportKind::Pool(state) => {
                state
                    .configure_from_login(token, max_forward_paths, idle_timeout_secs)
                    .await;
            }
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => {
                state
                    .configure_from_login(token, max_forward_paths, idle_timeout_secs)
                    .await;
            }
        }
    }

    pub async fn remove_path(&self, path_id: Option<u64>) {
        let Some(path_id) = path_id else {
            return;
        };
        match &self.kind {
            ClientTransportKind::Single { .. } => {}
            ClientTransportKind::Pool(state) => state.remove_forward_path(path_id).await,
            #[cfg(feature = "quic")]
            ClientTransportKind::Quic(state) => state.remove_forward_path(path_id).await,
        }
    }
}
