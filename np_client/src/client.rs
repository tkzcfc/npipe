use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::{debug, error};
use np_proto::message_map::{
    decode_message, encode_message, encode_raw_message, get_message_id, get_message_size,
    MessageType,
};
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::time::{self, Instant};

enum Response {
    // 请求回复的消息
    Message(MessageType),
    // 等待回复中...
    Waiting,
    // 请求被取消
    Cancel,
    // 请求出错，如服务器返回的消息客户端解码失败
    Error,
}

enum ChannelMessage {
    Disconnect,
    RecvMessage(i32, MessageType),
}

pub struct Client {
    addr: SocketAddr,
    writer: Option<WriteHalf<TcpStream>>,
    serial: i32,
    response_map: HashMap<i32, Response>,
    closed: bool,
    rx: Option<UnboundedReceiver<ChannelMessage>>,
}

impl Client {
    pub fn new(addr: SocketAddr) -> Arc<RwLock<Client>> {
        Arc::new(RwLock::new(Client {
            addr,
            writer: None,
            serial: 0i32,
            response_map: HashMap::new(),
            closed: true,
            rx: None,
        }))
    }

    pub async fn connect(&mut self) -> Result<(), io::Error> {
        self.disconnect();

        let socket = if self.addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(self.addr).await?;
        let (mut reader, writer) = tokio::io::split(stream);

        self.writer = Some(writer);

        let (tx, rx) = unbounded_channel();

        let addr = self.addr.clone();

        // 单独开一个协程处理读逻辑
        tokio::spawn(async move {
            let mut buffer = BytesMut::with_capacity(1024);
            loop {
                if tx.is_closed() {
                    break;
                }

                match reader.read_buf(&mut buffer).await {
                    // n为0表示对端已经关闭连接。
                    Ok(n) if n == 0 => {
                        debug!("socket[{}] closed.", addr);
                        if let Err(error) = tx.send(ChannelMessage::Disconnect) {
                            error!("Send channel message error: {}", error);
                        }
                        return;
                    }
                    Ok(_n) => {
                        loop {
                            if let Ok(result) = try_extract_frame(&mut buffer) {
                                if let Some(frame) = result {
                                    if !on_recv_pkg_frame(&tx, frame) {
                                        return;
                                    }
                                } else {
                                    // 数据包长度不够,继续读
                                    break;
                                }
                            } else {
                                error!("Message too long");
                                // 消息过长, 主动断开
                                if let Err(error) = tx.send(ChannelMessage::Disconnect) {
                                    error!("Send channel message error: {}", error);
                                }
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from socket[{}]: {}", addr, e);
                        // socket读错误,主动断开
                        if let Err(error) = tx.send(ChannelMessage::Disconnect) {
                            error!("Send channel message error: {}", error);
                        }
                        return;
                    }
                }
            }
        });

        Ok(())
    }

    // 是否关闭会话
    #[inline]
    pub fn is_connect(&self) -> bool {
        !self.closed
    }

    pub fn disconnect(&mut self) {
        self.response_map.clear();

        if let Some(ref mut rx) = self.rx {
            rx.close();
            self.rx = None;
        }

        if self.is_connect() {
            self.closed = true;

            if let Some(mut writer) = self.writer.take() {
                tokio::spawn(async move {
                    if let Err(error) = writer.shutdown().await {
                        error!("Socket shutdown error: {}", error)
                    }
                });
            }
        }
    }

    pub async fn send_request(&mut self, message: &MessageType) -> io::Result<MessageType> {
        if let Some(ref mut writer) = self.writer {
            // 防止请求序号越界
            if self.serial >= i32::MAX {
                self.serial = 0;
            }
            self.serial += 1;

            let serial = -self.serial;
            self.response_map.insert(serial, Response::Waiting);
            package_and_send_message(writer, serial, message, true).await?;

            let start = Instant::now();
            // 检测间隔时间 20毫秒检测一次
            let mut interval = time::interval(Duration::from_millis(20));
            // 10超时等待时间
            while Instant::now().duration_since(start) < Duration::from_secs(10) {
                interval.tick().await;
                if let Some(response) = self.response_map.get(&serial) {
                    match response {
                        Response::Message(_message) => {
                            if let Some(message) = self.response_map.remove(&serial) {
                                if let Response::Message(msg) = message {
                                    return Ok(msg);
                                }
                            }
                            // 不可能出现的错误
                            self.response_map.remove(&serial);
                            return Err(io::Error::new(ErrorKind::Other, "impossible errors"));
                        }
                        Response::Waiting => {}
                        Response::Cancel => {
                            // 请求被取消
                            self.response_map.remove(&serial);
                            return Err(io::Error::new(ErrorKind::TimedOut, "request cancelled"));
                        }
                        Response::Error => {
                            self.response_map.remove(&serial);
                            return Err(io::Error::new(
                                ErrorKind::Other,
                                "protocol decoding failed",
                            ));
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
        Err(io::Error::new(ErrorKind::NotConnected, "not connected"))
    }

    fn update_frame(&mut self) {
        if let Some(ref mut rx) = self.rx {
            loop {
                if let Ok(channel_message) = rx.try_recv() {
                    match channel_message {
                        ChannelMessage::Disconnect => {
                            self.disconnect();
                            break;
                        }
                        ChannelMessage::RecvMessage(serial, message) => {}
                    }
                } else {
                    break;
                }
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

// 收到一个完整的消息包
fn on_recv_pkg_frame(tx: &UnboundedSender<ChannelMessage>, frame: Vec<u8>) -> bool {
    // 消息不合法，长度不够
    if frame.len() < 8 {
        return false;
    }
    // 消息序号
    let serial: i32 = BigEndian::read_i32(&frame[0..4]);
    // 消息类型id
    let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

    return match decode_message(msg_id, &frame[8..]) {
        Ok(message) => {
            if let Err(error) = tx.send(ChannelMessage::RecvMessage(serial, message)) {
                error!("Send channel message error: {}", error);
                return false;
            }
            true
        }
        Err(err) => {
            error!("Protobuf parse error: {}", err);
            false
        }
    };
}

#[inline]
pub(crate) async fn package_and_send_message(
    writer: &mut WriteHalf<TcpStream>,
    serial: i32,
    message: &MessageType,
    flush: bool,
) -> io::Result<()> {
    if let Some(message_id) = get_message_id(message) {
        let message_size = get_message_size(message);
        let mut buf = Vec::with_capacity(message_size + 12);

        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
        byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, -serial)?;
        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
        encode_raw_message(message, &mut buf);

        writer.write_all(&buf).await?;
        if flush {
            writer.flush().await?;
        }
    }
    Ok(())
}
