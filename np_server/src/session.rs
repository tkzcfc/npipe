use crate::player::Player;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::{debug, error};
use np_base::generic;
use np_base::message_map::{decode_message, encode_raw_message, MessageType};
use np_base::message_map::{get_message_id, get_message_size};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadHalf, WriteHalf,
};
use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::yield_now;
use tokio::time::sleep;

pub enum WriterMessage {
    Close,
    Flush,
    CloseDelayed(Duration),
    Send(Vec<u8>, bool),
}

pub struct Session {
    pub tx: UnboundedSender<WriterMessage>,
    pub addr: SocketAddr,
    pub player: Option<Arc<RwLock<Player>>>,
    session_id: u32,
    closed: bool,
}

impl Drop for Session {
    fn drop(&mut self) {}
}

impl Session {
    pub fn new(tx: UnboundedSender<WriterMessage>, addr: SocketAddr, session_id: u32) -> Session {
        Session {
            tx,
            addr,
            player: Option::None,
            session_id,
            closed: false,
        }
    }

    // 是否关闭会话
    #[inline]
    pub(crate) fn is_closed(&self) -> bool {
        self.closed || self.tx.is_closed()
    }

    // 获取会话id
    #[inline]
    pub(crate) fn get_session_id(&self) -> u32 {
        self.session_id
    }

    // clone tx
    #[inline]
    pub(crate) fn clone_tx(&self) -> UnboundedSender<WriterMessage> {
        self.tx.clone()
    }

    // 关闭会话
    #[inline]
    pub(crate) fn close_session(&mut self) {
        self.closed = true;
        let _ = self.tx.send(WriterMessage::Close);
    }

    async fn on_session_close(&mut self) {
        if let Some(player) = self.player.take() {
            if player.read().await.get_session_id() == self.session_id {
                player.write().await.on_disconnect_session().await;
            }
        }
    }

    pub(crate) async fn send_response(&self, serial: i32, message: &MessageType) -> io::Result<()> {
        package_and_send_message(&self.tx, serial, message, true).await
    }

    pub(crate) async fn send_request(&self, _message: &MessageType) -> io::Result<MessageType> {
        todo!();
    }

    pub(crate) async fn send_push(&self, message: &MessageType) -> io::Result<()> {
        package_and_send_message(&self.tx, 0, message, true).await
    }

    pub async fn run<S>(
        &mut self,
        rx: UnboundedReceiver<WriterMessage>,
        reader: ReadHalf<S>,
        writer: WriteHalf<S>,
    ) where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        select! {
            _ = self.poll_read(reader) => {}
            _ = Self::poll_write(rx, writer) => {}
        }
        self.on_session_close().await;
    }

    // 循环写入数据
    async fn poll_write<S>(mut rx: UnboundedReceiver<WriterMessage>, writer: WriteHalf<S>)
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let mut writer = BufWriter::new(writer);

        while let Some(message) = rx.recv().await {
            match message {
                WriterMessage::Close => break,
                WriterMessage::CloseDelayed(duration) => {
                    sleep(duration).await;
                    break;
                }
                WriterMessage::Send(data, flush) => {
                    if data.is_empty() {
                        yield_now().await;
                        continue;
                    }

                    if let Err(error) = writer.write_all(&data).await {
                        error!("Error when write_all {:?}", error);
                        break;
                    }

                    if flush {
                        if let Err(error) = writer.flush().await {
                            error!("Error when flushing {:?}", error);
                        }
                    }
                }
                WriterMessage::Flush => {
                    if let Err(error) = writer.flush().await {
                        error!("Error when flushing {:?}", error);
                    }
                }
            }

            yield_now().await;
        }

        rx.close();
    }

    // 循环读取数据
    async fn poll_read<S>(&mut self, mut reader: ReadHalf<S>)
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let mut buffer = BytesMut::with_capacity(1024);

        loop {
            match reader.read_buf(&mut buffer).await {
                // n为0表示对端已经关闭连接。
                Ok(n) if n == 0 => {
                    debug!("Socket[{}] closed.", self.addr);
                    // 客户端主动断开
                    self.close_session();
                    return;
                }
                Ok(_n) => {
                    while !self.is_closed() {
                        // 粘包处理
                        if let Ok(result) = try_extract_frame(&mut buffer) {
                            if let Some(frame) = result {
                                self.on_recv_pkg_frame(frame).await;
                            } else {
                                break;
                            }
                        } else {
                            debug!("Message too long");
                            // 消息过长, 主动断开
                            self.close_session();
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket[{}]: {}", self.addr, e);
                    // socket读错误
                    self.close_session();
                    return;
                }
            }
        }
    }

    // 收到一个完整的消息包
    async fn on_recv_pkg_frame(&mut self, frame: Vec<u8>) {
        if frame.len() < 8 {
            self.close_session();
            return;
        }
        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

        match decode_message(msg_id, &frame[8..]) {
            Ok(message) => match self.on_recv_message(serial, message).await {
                Ok(msg) => {
                    if serial < 0 {
                        if let MessageType::None = msg {
                            // 请求不应该不回复
                            error!("The response to request {} is empty", msg_id);
                            let _ = self
                                .send_response(
                                    serial,
                                    &MessageType::GenericError(generic::Error {
                                        number: generic::ErrorCode::InternalError.into(),
                                        message: format!("response is empty"),
                                    }),
                                )
                                .await;
                        } else {
                            let _ = self.send_response(serial, &msg).await;
                        }
                    }
                }
                Err(err) => {
                    error!("Request error: {}, message id: {}", err, msg_id);

                    let _ = self
                        .send_response(
                            serial,
                            &MessageType::GenericError(generic::Error {
                                number: generic::ErrorCode::InternalError.into(),
                                message: format!("{}", err),
                            }),
                        )
                        .await;
                }
            },
            Err(err) => {
                error!("Protobuf parse error: {}", err);
                let _ = self
                    .send_response(
                        serial,
                        &MessageType::GenericError(generic::Error {
                            number: generic::ErrorCode::InternalError.into(),
                            message: format!("{}", err),
                        }),
                    )
                    .await;

                self.close_session();
            }
        }
    }

    pub async fn on_recv_message(
        &mut self,
        serial: i32,
        message: MessageType,
    ) -> io::Result<MessageType> {
        return if serial < 0 {
            self.on_recv_request(message).await
        } else if serial > 0 {
            self.on_recv_response(message).await?;
            Ok(MessageType::None)
        } else {
            self.on_recv_push(message).await?;
            Ok(MessageType::None)
        };
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

#[inline]
pub(crate) async fn package_and_send_message(
    tx: &UnboundedSender<WriterMessage>,
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

        if let Err(error) = tx.send(WriterMessage::Send(buf, flush)) {
            error!("Send message error: {}", error);
        }
    }
    Ok(())
}
