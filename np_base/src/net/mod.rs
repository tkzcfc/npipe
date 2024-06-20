use std::time::Duration;

pub mod client;
pub mod session_delegate;
pub mod tcp_server;
pub mod tcp_session;
pub mod udp_server;
pub mod udp_session;

pub enum WriterMessage {
    Close,
    Flush,
    CloseDelayed(Duration),
    Send(Vec<u8>, bool),
}
