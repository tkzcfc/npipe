use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub mod session_delegate;
pub mod tcp_client;
pub mod tcp_server;
pub mod tcp_session;
pub mod udp_server;
pub mod udp_session;

pub type SendMessageFuncType =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub enum WriterMessage {
    Close,
    Flush,
    CloseDelayed(Duration),
    Send(Vec<u8>, bool),
    SendAndThen(Vec<u8>, SendMessageFuncType),
}
