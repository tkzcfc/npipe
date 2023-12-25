use bytes::BufMut;
use prost::{DecodeError, Message};

#[derive(Clone)]
pub enum MessageType {
    None,
    ClientServerLoginReq(super::client_server::LoginReq),
    ServerClientLoginAck(super::server_client::LoginAck),
    GenericSuccess(super::generic::Success),
    GenericFail(super::generic::Fail),
    GenericError(super::generic::Error),
    GenericPing(super::generic::Ping),
    GenericPong(super::generic::Pong),
}

pub fn get_message_id(message: &MessageType) -> Option<u32> {
    match message {
        MessageType::ClientServerLoginReq(_) => Some(1001u32),
        MessageType::ServerClientLoginAck(_) => Some(1002u32),
        MessageType::GenericSuccess(_) => Some(150001u32),
        MessageType::GenericFail(_) => Some(150002u32),
        MessageType::GenericError(_) => Some(150003u32),
        MessageType::GenericPing(_) => Some(150004u32),
        MessageType::GenericPong(_) => Some(150005u32),
        _ => None,
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
        150004u32 => match super::generic::Ping::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericPing(message)),
            Err(err) => Err(err),
        },
        150005u32 => match super::generic::Pong::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericPong(message)),
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
        MessageType::GenericPing(msg) => Some((150004u32, msg.encode_to_vec())),
        MessageType::GenericPong(msg) => Some((150005u32, msg.encode_to_vec())),
        _ => None,
    }
}

pub fn get_message_size(message: &MessageType) -> usize {
    match message {
        MessageType::ClientServerLoginReq(msg) => msg.encoded_len(),
        MessageType::ServerClientLoginAck(msg) => msg.encoded_len(),
        MessageType::GenericSuccess(msg) => msg.encoded_len(),
        MessageType::GenericFail(msg) => msg.encoded_len(),
        MessageType::GenericError(msg) => msg.encoded_len(),
        MessageType::GenericPing(msg) => msg.encoded_len(),
        MessageType::GenericPong(msg) => msg.encoded_len(),
        _ => 0,
    }
}

pub fn encode_raw_message(message: &MessageType, buf: &mut impl BufMut) {
    match message {
        MessageType::ClientServerLoginReq(msg) => msg.encode_raw(buf),
        MessageType::ServerClientLoginAck(msg) => msg.encode_raw(buf),
        MessageType::GenericSuccess(msg) => msg.encode_raw(buf),
        MessageType::GenericFail(msg) => msg.encode_raw(buf),
        MessageType::GenericError(msg) => msg.encode_raw(buf),
        MessageType::GenericPing(msg) => msg.encode_raw(buf),
        MessageType::GenericPong(msg) => msg.encode_raw(buf),
        _ => {}
    }
}

