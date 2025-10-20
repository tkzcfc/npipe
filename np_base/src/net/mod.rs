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

pub enum WriterMessage {
    Close,
    Flush,
    CloseDelayed(Duration),
    Send(Vec<u8>, bool),
    SendTo(Vec<u8>, SocketAddr),
    SendAndThen(Vec<u8>, SendMessageFuncType),
}
