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
    ServerClientModifyTunnelNtf(super::server_client::ModifyTunnelNtf),
    GenericSuccess(super::generic::Success),
    GenericFail(super::generic::Fail),
    GenericError(super::generic::Error),
    GenericPing(super::generic::Ping),
    GenericPong(super::generic::Pong),
    GenericI2oConnect(super::generic::I2oConnect),
    GenericO2iConnect(super::generic::O2iConnect),
    GenericI2oSendData(super::generic::I2oSendData),
    GenericO2iRecvData(super::generic::O2iRecvData),
    GenericI2oDisconnect(super::generic::I2oDisconnect),
    GenericO2iDisconnect(super::generic::O2iDisconnect),
}

pub fn get_message_id(message: &MessageType) -> Option<u32> {
    match message {
        MessageType::ClientServerLoginReq(_) => Some(1001u32),
        MessageType::ClientServerRegisterReq(_) => Some(1003u32),
        MessageType::ClientServerManagementLoginReq(_) => Some(1005u32),
        MessageType::ServerClientLoginAck(_) => Some(1002u32),
        MessageType::ServerClientManagementLoginAck(_) => Some(1006u32),
        MessageType::ServerClientModifyTunnelNtf(_) => Some(1008u32),
        MessageType::GenericSuccess(_) => Some(150001u32),
        MessageType::GenericFail(_) => Some(150002u32),
        MessageType::GenericError(_) => Some(150003u32),
        MessageType::GenericPing(_) => Some(150004u32),
        MessageType::GenericPong(_) => Some(150005u32),
        MessageType::GenericI2oConnect(_) => Some(150006u32),
        MessageType::GenericO2iConnect(_) => Some(150007u32),
        MessageType::GenericI2oSendData(_) => Some(150008u32),
        MessageType::GenericO2iRecvData(_) => Some(150009u32),
        MessageType::GenericI2oDisconnect(_) => Some(150010u32),
        MessageType::GenericO2iDisconnect(_) => Some(150011u32),
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
        1008u32 => match super::server_client::ModifyTunnelNtf::decode(bytes) {
            Ok(message) => Ok(MessageType::ServerClientModifyTunnelNtf(message)),
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
        150006u32 => match super::generic::I2oConnect::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericI2oConnect(message)),
            Err(err) => Err(err),
        },
        150007u32 => match super::generic::O2iConnect::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericO2iConnect(message)),
            Err(err) => Err(err),
        },
        150008u32 => match super::generic::I2oSendData::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericI2oSendData(message)),
            Err(err) => Err(err),
        },
        150009u32 => match super::generic::O2iRecvData::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericO2iRecvData(message)),
            Err(err) => Err(err),
        },
        150010u32 => match super::generic::I2oDisconnect::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericI2oDisconnect(message)),
            Err(err) => Err(err),
        },
        150011u32 => match super::generic::O2iDisconnect::decode(bytes) {
            Ok(message) => Ok(MessageType::GenericO2iDisconnect(message)),
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
        MessageType::ServerClientModifyTunnelNtf(msg) => Some((1008u32, msg.encode_to_vec())),
        MessageType::GenericSuccess(msg) => Some((150001u32, msg.encode_to_vec())),
        MessageType::GenericFail(msg) => Some((150002u32, msg.encode_to_vec())),
        MessageType::GenericError(msg) => Some((150003u32, msg.encode_to_vec())),
        MessageType::GenericPing(msg) => Some((150004u32, msg.encode_to_vec())),
        MessageType::GenericPong(msg) => Some((150005u32, msg.encode_to_vec())),
        MessageType::GenericI2oConnect(msg) => Some((150006u32, msg.encode_to_vec())),
        MessageType::GenericO2iConnect(msg) => Some((150007u32, msg.encode_to_vec())),
        MessageType::GenericI2oSendData(msg) => Some((150008u32, msg.encode_to_vec())),
        MessageType::GenericO2iRecvData(msg) => Some((150009u32, msg.encode_to_vec())),
        MessageType::GenericI2oDisconnect(msg) => Some((150010u32, msg.encode_to_vec())),
        MessageType::GenericO2iDisconnect(msg) => Some((150011u32, msg.encode_to_vec())),
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
        MessageType::ServerClientModifyTunnelNtf(msg) => msg.encoded_len(),
        MessageType::GenericSuccess(msg) => msg.encoded_len(),
        MessageType::GenericFail(msg) => msg.encoded_len(),
        MessageType::GenericError(msg) => msg.encoded_len(),
        MessageType::GenericPing(msg) => msg.encoded_len(),
        MessageType::GenericPong(msg) => msg.encoded_len(),
        MessageType::GenericI2oConnect(msg) => msg.encoded_len(),
        MessageType::GenericO2iConnect(msg) => msg.encoded_len(),
        MessageType::GenericI2oSendData(msg) => msg.encoded_len(),
        MessageType::GenericO2iRecvData(msg) => msg.encoded_len(),
        MessageType::GenericI2oDisconnect(msg) => msg.encoded_len(),
        MessageType::GenericO2iDisconnect(msg) => msg.encoded_len(),
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
        MessageType::ServerClientModifyTunnelNtf(msg) => msg.encode_raw(buf),
        MessageType::GenericSuccess(msg) => msg.encode_raw(buf),
        MessageType::GenericFail(msg) => msg.encode_raw(buf),
        MessageType::GenericError(msg) => msg.encode_raw(buf),
        MessageType::GenericPing(msg) => msg.encode_raw(buf),
        MessageType::GenericPong(msg) => msg.encode_raw(buf),
        MessageType::GenericI2oConnect(msg) => msg.encode_raw(buf),
        MessageType::GenericO2iConnect(msg) => msg.encode_raw(buf),
        MessageType::GenericI2oSendData(msg) => msg.encode_raw(buf),
        MessageType::GenericO2iRecvData(msg) => msg.encode_raw(buf),
        MessageType::GenericI2oDisconnect(msg) => msg.encode_raw(buf),
        MessageType::GenericO2iDisconnect(msg) => msg.encode_raw(buf),
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
        MessageType::ServerClientModifyTunnelNtf(msg) => serde_json::to_string(&msg),
        MessageType::GenericSuccess(msg) => serde_json::to_string(&msg),
        MessageType::GenericFail(msg) => serde_json::to_string(&msg),
        MessageType::GenericError(msg) => serde_json::to_string(&msg),
        MessageType::GenericPing(msg) => serde_json::to_string(&msg),
        MessageType::GenericPong(msg) => serde_json::to_string(&msg),
        MessageType::GenericI2oConnect(msg) => serde_json::to_string(&msg),
        MessageType::GenericO2iConnect(msg) => serde_json::to_string(&msg),
        MessageType::GenericI2oSendData(msg) => serde_json::to_string(&msg),
        MessageType::GenericO2iRecvData(msg) => serde_json::to_string(&msg),
        MessageType::GenericI2oDisconnect(msg) => serde_json::to_string(&msg),
        MessageType::GenericO2iDisconnect(msg) => serde_json::to_string(&msg),
        _ => Ok("null".into()),
    }
}
