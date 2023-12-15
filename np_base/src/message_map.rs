use prost::{DecodeError, Message};

pub enum MessageType {
    None,
    ClientServerLoginReq(super::client_server::LoginReq),
    ServerClientLoginAck(super::server_client::LoginAck),
}

pub fn get_message_id(message: MessageType) ->u32 {
    match message {
        MessageType::ClientServerLoginReq(_) => 1001u32,
        MessageType::ServerClientLoginAck(_) => 1002u32,
        _=> panic!("error message")
    }
}

pub fn parse_message(message_id: u32, bytes: &[u8]) -> Result<MessageType, DecodeError> {
    match message_id {
        1001u32 => {
            match super::client_server::LoginReq::decode(bytes) {
                Ok(message) => Ok(MessageType::ClientServerLoginReq(message)),
                Err(err)=> Err(err)
            }
        }
        1002u32 => {
            match super::server_client::LoginAck::decode(bytes) {
                Ok(message) => Ok(MessageType::ServerClientLoginAck(message)),
                Err(err)=> Err(err)
            }
        }
        _ => Err(DecodeError::new("unknown message id"))
    }
}
