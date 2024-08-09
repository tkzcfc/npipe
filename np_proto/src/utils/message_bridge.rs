use crate::generic;
use crate::message_map::MessageType;
use np_base::proxy::ProxyMessage;

pub fn proxy_message_2_pb(proxy_message: ProxyMessage, tunnel_id: u32) -> MessageType {
    match proxy_message {
        ProxyMessage::I2oConnect(session_id, tunnel_type, is_tcp, is_compressed, addr, encryption_method, encryption_key, client_addr) => {
            MessageType::GenericI2oConnect(generic::I2oConnect {
                tunnel_id,
                session_id,
                tunnel_type: tunnel_type as u32,
                addr,
                is_tcp,
                is_compressed,
                encryption_method,
                encryption_key,
                client_addr,
            })
        }
        ProxyMessage::O2iConnect(session_id, success, error_info) => MessageType::GenericO2iConnect(generic::O2iConnect {
            tunnel_id,
            session_id,
            success,
            error_info,
        }),
        ProxyMessage::I2oSendData(session_id, data) => MessageType::GenericI2oSendData(generic::I2oSendData { tunnel_id, session_id, data }),
        ProxyMessage::I2oSendToData(session_id, data, target_addr) => MessageType::GenericI2oSendToData(generic::I2oSendToData {
            tunnel_id,
            session_id,
            data,
            target_addr,
        }),
        ProxyMessage::O2iSendDataResult(session_id, data_len) => MessageType::GenericO2iSendDataResult(generic::O2iSendDataResult {
            tunnel_id,
            session_id,
            data_len: data_len as u32,
        }),
        ProxyMessage::O2iRecvData(session_id, data) => MessageType::GenericO2iRecvData(generic::O2iRecvData { tunnel_id, session_id, data }),
        ProxyMessage::O2iRecvDataFrom(session_id, data, remote_addr) => MessageType::GenericO2iRecvDataFrom(generic::O2iRecvDataFrom {
            tunnel_id,
            session_id,
            data,
            remote_addr,
        }),
        ProxyMessage::I2oRecvDataResult(session_id, data_len) => MessageType::GenericI2oRecvDataResult(generic::I2oRecvDataResult {
            tunnel_id,
            session_id,
            data_len: data_len as u32,
        }),
        ProxyMessage::I2oDisconnect(session_id) => MessageType::GenericI2oDisconnect(generic::I2oDisconnect { tunnel_id, session_id }),
        ProxyMessage::O2iDisconnect(session_id) => MessageType::GenericO2iDisconnect(generic::O2iDisconnect { tunnel_id, session_id }),
    }
}

pub fn pb_2_proxy_message(message: MessageType) -> Option<(ProxyMessage, u32)> {
    match message {
        MessageType::GenericI2oConnect(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericI2oSendData(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericI2oSendToData(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericI2oRecvDataResult(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericI2oDisconnect(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericO2iConnect(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericO2iRecvData(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }

        MessageType::GenericO2iRecvDataFrom(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericO2iDisconnect(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        MessageType::GenericO2iSendDataResult(msg) => {
            let tunnel_id = msg.tunnel_id;
            Some((msg.into(), tunnel_id))
        }
        _ => None,
    }
}

pub fn is_i2o_message(proxy_message: &ProxyMessage) -> bool {
    match proxy_message {
        ProxyMessage::I2oConnect(_, ..)
        | ProxyMessage::I2oSendData(_, ..)
        | ProxyMessage::I2oSendToData(_, ..)
        | ProxyMessage::I2oDisconnect(_)
        | ProxyMessage::I2oRecvDataResult(_, ..) => true,

        ProxyMessage::O2iConnect(_, ..)
        | ProxyMessage::O2iSendDataResult(_, ..)
        | ProxyMessage::O2iRecvData(_, ..)
        | ProxyMessage::O2iRecvDataFrom(_, ..)
        | ProxyMessage::O2iDisconnect(_, ..) => false,
    }
}

impl From<generic::I2oConnect> for ProxyMessage {
    fn from(msg: generic::I2oConnect) -> Self {
        ProxyMessage::I2oConnect(
            msg.session_id,
            msg.tunnel_type as u8,
            msg.is_tcp,
            msg.is_compressed,
            msg.addr,
            msg.encryption_method,
            msg.encryption_key,
            msg.client_addr,
        )
    }
}

impl From<generic::O2iConnect> for ProxyMessage {
    fn from(msg: generic::O2iConnect) -> Self {
        ProxyMessage::O2iConnect(msg.session_id, msg.success, msg.error_info)
    }
}

impl From<generic::I2oSendData> for ProxyMessage {
    fn from(msg: generic::I2oSendData) -> Self {
        ProxyMessage::I2oSendData(msg.session_id, msg.data)
    }
}

impl From<generic::I2oSendToData> for ProxyMessage {
    fn from(msg: generic::I2oSendToData) -> Self {
        ProxyMessage::I2oSendToData(msg.session_id, msg.data, msg.target_addr)
    }
}

impl From<generic::O2iSendDataResult> for ProxyMessage {
    fn from(msg: generic::O2iSendDataResult) -> Self {
        ProxyMessage::O2iSendDataResult(msg.session_id, msg.data_len as usize)
    }
}

impl From<generic::O2iRecvData> for ProxyMessage {
    fn from(msg: generic::O2iRecvData) -> Self {
        ProxyMessage::O2iRecvData(msg.session_id, msg.data)
    }
}

impl From<generic::O2iRecvDataFrom> for ProxyMessage {
    fn from(msg: generic::O2iRecvDataFrom) -> Self {
        ProxyMessage::O2iRecvDataFrom(msg.session_id, msg.data, msg.remote_addr)
    }
}

impl From<generic::I2oRecvDataResult> for ProxyMessage {
    fn from(msg: generic::I2oRecvDataResult) -> Self {
        ProxyMessage::I2oRecvDataResult(msg.session_id, msg.data_len as usize)
    }
}

impl From<generic::I2oDisconnect> for ProxyMessage {
    fn from(msg: generic::I2oDisconnect) -> Self {
        ProxyMessage::I2oDisconnect(msg.session_id)
    }
}

impl From<generic::O2iDisconnect> for ProxyMessage {
    fn from(msg: generic::O2iDisconnect) -> Self {
        ProxyMessage::O2iDisconnect(msg.session_id)
    }
}
