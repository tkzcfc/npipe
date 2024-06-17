use bytes::BufMut;
use prost::{DecodeError, Message};

#[derive(Clone)]
pub enum MessageType {
    None,
    ClientServerLoginReq(super::client_server::LoginReq),
    ClientServerRegisterReq(super::client_server::RegisterReq),
    ClientServerManagementLoginReq(super::client_server::ManagementLoginReq),
    ServerClientLoginAck(super::server_client::LoginAck),
    ServerClientManagementLoginAck(super::server_client::ManagementLoginAck),
    ServerClientModifyChannelNtf(super::server_client::ModifyChannelNtf),
    ServerClientSendMessageToChannel(super::server_client::SendMessageToChannel),
    ServerClientRecvMessageFromChannel(super::server_client::RecvMessageFromChannel),
    GenericSuccess(super::generic::Success),
    GenericFail(super::generic::Fail),
    GenericError(super::generic::Error),
    GenericPing(super::generic::Ping),
    GenericPong(super::generic::Pong),
}

pub fn get_message_id(message: &MessageType) -> Option<u32> {
    match message {
        MessageType::ClientServerLoginReq(_) => Some(1001u32),
        MessageType::ClientServerRegisterReq(_) => Some(1003u32),
        MessageType::ClientServerManagementLoginReq(_) => Some(1005u32),
        MessageType::ServerClientLoginAck(_) => Some(1002u32),
        MessageType::ServerClientManagementLoginAck(_) => Some(1006u32),
        MessageType::ServerClientModifyChannelNtf(_) => Some(1008u32),
        MessageType::ServerClientSendMessageToChannel(_) => Some(1010u32),
        MessageType::ServerClientRecvMessageFromChannel(_) => Some(1012u32),
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
        1003u32 => match super::client_server::RegisterReq::decode(bytes) {
            Ok(message) => Ok(MessageType::ClientServerRegisterReq(message)),
            Err(err) => Err(err),
        },
        1005u32 => match super::client_server::ManagementLoginReq::decode(bytes) {
            Ok(message) => Ok(MessageType::ClientServerManagementLoginReq(message)),
            Err(err) => Err(err),
        },
        1002u32 => match super::server_client::LoginAck::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientLoginAck(message)),
            Err(err) => Err(err),
        },
        1006u32 => match super::server_client::ManagementLoginAck::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientManagementLoginAck(message)),
            Err(err) => Err(err),
        },
        1008u32 => match super::server_client::ModifyChannelNtf::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientModifyChannelNtf(message)),
            Err(err) => Err(err),
        },
        1010u32 => match super::server_client::SendMessageToChannel::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientSendMessageToChannel(message)),
            Err(err) => Err(err),
        },
        1012u32 => match super::server_client::RecvMessageFromChannel::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientRecvMessageFromChannel(message)),
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
        MessageType::ClientServerRegisterReq(msg) => Some((1003u32, msg.encode_to_vec())),
        MessageType::ClientServerManagementLoginReq(msg) => Some((1005u32, msg.encode_to_vec())),
        MessageType::ServerClientLoginAck(msg) => Some((1002u32, msg.encode_to_vec())),
        MessageType::ServerClientManagementLoginAck(msg) => Some((1006u32, msg.encode_to_vec())),
        MessageType::ServerClientModifyChannelNtf(msg) => Some((1008u32, msg.encode_to_vec())),
        MessageType::ServerClientSendMessageToChannel(msg) => Some((1010u32, msg.encode_to_vec())),
        MessageType::ServerClientRecvMessageFromChannel(msg) => {
            Some((1012u32, msg.encode_to_vec()))
        }
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
        MessageType::ClientServerRegisterReq(msg) => msg.encoded_len(),
        MessageType::ClientServerManagementLoginReq(msg) => msg.encoded_len(),
        MessageType::ServerClientLoginAck(msg) => msg.encoded_len(),
        MessageType::ServerClientManagementLoginAck(msg) => msg.encoded_len(),
        MessageType::ServerClientModifyChannelNtf(msg) => msg.encoded_len(),
        MessageType::ServerClientSendMessageToChannel(msg) => msg.encoded_len(),
        MessageType::ServerClientRecvMessageFromChannel(msg) => msg.encoded_len(),
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
        MessageType::ClientServerRegisterReq(msg) => msg.encode_raw(buf),
        MessageType::ClientServerManagementLoginReq(msg) => msg.encode_raw(buf),
        MessageType::ServerClientLoginAck(msg) => msg.encode_raw(buf),
        MessageType::ServerClientManagementLoginAck(msg) => msg.encode_raw(buf),
        MessageType::ServerClientModifyChannelNtf(msg) => msg.encode_raw(buf),
        MessageType::ServerClientSendMessageToChannel(msg) => msg.encode_raw(buf),
        MessageType::ServerClientRecvMessageFromChannel(msg) => msg.encode_raw(buf),
        MessageType::GenericSuccess(msg) => msg.encode_raw(buf),
        MessageType::GenericFail(msg) => msg.encode_raw(buf),
        MessageType::GenericError(msg) => msg.encode_raw(buf),
        MessageType::GenericPing(msg) => msg.encode_raw(buf),
        MessageType::GenericPong(msg) => msg.encode_raw(buf),
        _ => {}
    }
}

#[cfg(feature = "serde-serialize")]
pub fn serialize_to_json(message: &MessageType) -> serde_json::Result<String> {
    match message {
        MessageType::ClientServerLoginReq(msg) => serde_json::to_string(&msg),
        MessageType::ClientServerRegisterReq(msg) => serde_json::to_string(&msg),
        MessageType::ClientServerManagementLoginReq(msg) => serde_json::to_string(&msg),
        MessageType::ServerClientLoginAck(msg) => serde_json::to_string(&msg),
        MessageType::ServerClientManagementLoginAck(msg) => serde_json::to_string(&msg),
        MessageType::ServerClientModifyChannelNtf(msg) => serde_json::to_string(&msg),
        MessageType::ServerClientSendMessageToChannel(msg) => serde_json::to_string(&msg),
        MessageType::ServerClientRecvMessageFromChannel(msg) => serde_json::to_string(&msg),
        MessageType::GenericSuccess(msg) => serde_json::to_string(&msg),
        MessageType::GenericFail(msg) => serde_json::to_string(&msg),
        MessageType::GenericError(msg) => serde_json::to_string(&msg),
        MessageType::GenericPing(msg) => serde_json::to_string(&msg),
        MessageType::GenericPong(msg) => serde_json::to_string(&msg),
        _ => Ok("null".into()),
    }
}
