use bytes::Bytes;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

pub mod net_session;

pub mod session_delegate;
pub mod tcp_server;
pub mod tls;
pub mod udp_server;
pub mod udp_session;

#[cfg(feature = "kcp")]
pub mod kcp_server;
#[cfg(feature = "quic")]
pub mod quic_server;
#[cfg(feature = "ws")]
pub mod ws_async_io;
#[cfg(feature = "ws")]
pub mod ws_server;

pub type SendMessageFuncType =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// 写入消息枚举
///
/// 使用 `Bytes` 而非 `Vec<u8>` 实现零拷贝传递:
/// - `Bytes::clone()` 是 O(1) 引用计数操作，不复制数据
/// - `Bytes::from(vec)` 是 O(1) 转移所有权操作
pub enum WriterMessage {
    Close,
    Flush,
    CloseDelayed(Duration),
    /// (data, flush_immediately)
    Send(Bytes, bool),
    SendTo(Bytes, SocketAddr),
    SendAndThen(Bytes, SendMessageFuncType),
}
