use prost::{DecodeError, EncodeError};
use crate::message_map::MessageType;

pub mod message_map;
pub mod client_server;
pub mod server_client;


pub fn encode_message(message: MessageType) -> Result<(u32), EncodeError> {
    match message {
        MessageType::ClientServerLoginReq(_) => Ok((1001u32)),
        MessageType::ServerClientLoginAck(_) => Ok((1002u32)),
        _=> panic!("error message")
    }
}
