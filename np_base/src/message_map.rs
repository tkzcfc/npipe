use prost::{DecodeError, Message};

pub enum MessageType {
    None,
    ClientServerLoginReq(super::client_server::LoginReq),
    ServerClientLoginAck(super::server_client::LoginAck),
    GenericSuccess(super::generic::Success),
    GenericFail(super::generic::Fail),
    GenericError(super::generic::Error),
}

pub fn get_message_id(message: &MessageType) -> u32 {
    match message {
        MessageType::ClientServerLoginReq(_) => 1001u32,
        MessageType::ServerClientLoginAck(_) => 1002u32,
        MessageType::GenericSuccess(_) => 150001u32,
        MessageType::GenericFail(_) => 150002u32,
        MessageType::GenericError(_) => 150003u32,
        _ => panic!("error message"),
    }
}

pub fn decode_message(message_id: u32, bytes: &[u8]) -> Result<MessageType, DecodeError> {
    match message_id {
        1001u32 => match super::client_server::LoginReq::decode(bytes) {
            Ok(message) => Ok(MessageType::ClientServerLoginReq(message)),
            Err(err) => Err(err),
        },
        1002u32 => match super::server_client::LoginAck::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientLoginAck(message)),
            Err(err) => Err(err),
        },
        150001u32 => match super::generic::Success::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericSuccess(message)),
            Err(err) => Err(err),
        },
        150002u32 => match super::generic::Fail::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericFail(message)),
            Err(err) => Err(err),
        },
        150003u32 => match super::generic::Error::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericError(message)),
            Err(err) => Err(err),
        },
        _ => Err(DecodeError::new("unknown message id")),
    }
}

pub fn encode_message(message: &MessageType) -> Option<(u32, Vec<u8>)> {
    match message {
        MessageType::ClientServerLoginReq(msg) => Some((1001u32, msg.encode_to_vec())),
        MessageType::ServerClientLoginAck(msg) => Some((1002u32, msg.encode_to_vec())),
        MessageType::GenericSuccess(msg) => Some((150001u32, msg.encode_to_vec())),
        MessageType::GenericFail(msg) => Some((150002u32, msg.encode_to_vec())),
        MessageType::GenericError(msg) => Some((150003u32, msg.encode_to_vec())),
        _ => None,
    }
}

