use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::error;
use np_base::net::client::Client;
use np_proto::message_map::{
    decode_message, encode_raw_message, get_message_id, get_message_size, MessageType,
};
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::{self, Instant};

enum ResponseStatus {
    // 请求回复的消息
    Message(MessageType),
    // 等待回复中...
    Waiting,
    // 请求被取消
    Cancel,
    // 请求出错，如服务器返回的消息客户端解码失败
    Error,
}

pub type RecvMessageCallback = fn(i32, &MessageType);

pub(crate) struct RpcClient {
    inner: Client,
    serial: i32,
    on_recv_message_callback: Option<RecvMessageCallback>,
    response_map: HashMap<i32, ResponseStatus>,
}

impl RpcClient {
    pub(crate) fn new(addr: SocketAddr) -> RpcClient {
        RpcClient {
            inner: Client::new(addr, try_extract_frame),
            serial: 0,
            on_recv_message_callback: None,
            response_map: HashMap::new(),
        }
    }

    #[inline]
    pub(crate) async fn connect(&mut self) -> Result<(), io::Error> {
        self.inner.connect().await
    }

    #[inline]
    pub(crate) fn is_connect(&self) -> bool {
        self.inner.is_connect()
    }

    #[inline]
    pub(crate) fn disconnect(&mut self) {
        self.inner.disconnect()
    }

    pub(crate) fn set_recv_message_callback(&mut self, callback: RecvMessageCallback) {
        self.on_recv_message_callback = Some(callback);
    }

    pub(crate) async fn send_request(&mut self, message: MessageType) -> io::Result<MessageType> {
        // 防止请求序号越界
        if self.serial >= i32::MAX {
            self.serial = 0;
        }
        self.serial += 1;
        let serial = -self.serial;

        if let Err(error) = self.package_and_send_message(serial, &message).await {
            return Err(error);
        }
        self.response_map.insert(serial, ResponseStatus::Waiting);

        let start = Instant::now();
        // 检测间隔时间 20毫秒检测一次
        let mut interval = time::interval(Duration::from_millis(10));
        // 10超时等待时间
        while Instant::now().duration_since(start) < Duration::from_secs(10) {
            self.update();
            interval.tick().await;
            if let Some(response) = self.response_map.get(&serial) {
                match response {
                    ResponseStatus::Message(_message) => {
                        if let Some(message) = self.response_map.remove(&serial) {
                            if let ResponseStatus::Message(msg) = message {
                                return Ok(msg);
                            }
                        }
                        // 不可能出现的错误
                        self.response_map.remove(&serial);
                        return Err(io::Error::new(ErrorKind::Other, "impossible errors"));
                    }
                    ResponseStatus::Waiting => {}
                    ResponseStatus::Cancel => {
                        // 请求被取消
                        self.response_map.remove(&serial);
                        return Err(io::Error::new(ErrorKind::TimedOut, "request cancelled"));
                    }
                    ResponseStatus::Error => {
                        self.response_map.remove(&serial);
                        return Err(io::Error::new(ErrorKind::Other, "protocol decoding failed"));
                    }
                }
            } else {
                // 连接已重置
                return Err(io::Error::new(
                    ErrorKind::ConnectionReset,
                    "connection reset",
                ));
            }
        }

        // 请求等待回复超时
        self.response_map.remove(&serial);
        return Err(io::Error::new(ErrorKind::TimedOut, "request timeout"));
    }

    #[inline]
    pub(crate) async fn package_and_send_message(
        &mut self,
        serial: i32,
        message: &MessageType,
    ) -> io::Result<()> {
        if self.is_connect() {
            if let Some(message_id) = get_message_id(message) {
                let message_size = get_message_size(message);
                let mut buf = Vec::with_capacity(message_size + 12);

                byteorder::WriteBytesExt::write_u32::<BigEndian>(
                    &mut buf,
                    (8 + message_size) as u32,
                )?;
                byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
                byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
                encode_raw_message(message, &mut buf);

                return self.inner.send(&buf, true).await;
            }
        }
        Err(io::Error::new(ErrorKind::NotConnected, "not connected"))
    }

    pub(crate) fn update(&mut self) {
        if !self.inner.is_connect() {
            return;
        }
        loop {
            if let Some(frame) = self.inner.try_recv() {
                // 消息不合法，长度不够
                if frame.len() < 8 {
                    self.disconnect();
                    break;
                }
                // 消息序号
                let mut serial: i32 = BigEndian::read_i32(&frame[0..4]);
                // 消息类型id
                let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

                match decode_message(msg_id, &frame[8..]) {
                    Ok(message) => {
                        if let Some(ref on_recv_message_callback) = self.on_recv_message_callback {
                            on_recv_message_callback(serial, &message);
                        }

                        serial = -serial;
                        if self.response_map.contains_key(&serial) {
                            self.response_map
                                .insert(serial, ResponseStatus::Message(message));
                        }
                    }
                    Err(err) => {
                        error!("Protobuf parse error: {}", err);
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }
}

// 数据粘包处理
#[inline]
fn try_extract_frame(buffer: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
    // 数据小于4字节
    if buffer.len() < 4 {
        return Ok(None);
    }

    let bin = buffer.get(0..4).unwrap();
    let len = BigEndian::read_u32(bin) as usize;

    // 超出最大限制
    if len <= 0 || len >= 1024 * 1024 * 5 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            String::from("bad length"),
        ));
    }

    // 数据不够
    if buffer.len() < 4 + len {
        return Ok(None);
    }

    let frame = buffer.split_to(4 + len).split_off(4).to_vec();

    Ok(Some(frame))
}
