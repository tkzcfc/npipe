use crate::player::Player;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::{debug, error, trace};
use np_base::generic;
use np_base::message_map::get_message_id;
use np_base::message_map::{decode_message, encode_message, MessageType};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
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
    Send(Vec<u8>),
}

pub struct Session {
    pub tx: UnboundedSender<WriterMessage>,
    pub addr: SocketAddr,
    pub player: Option<Arc<RwLock<Player>>>,
    session_id: u32,
    closed: bool,
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
        if let Some(ref player) = self.player {
            if player.read().await.get_session_id() == self.session_id {
                player.write().await.on_disconnect_session().await;
            }
        }
        self.player = None;
    }

    pub async fn send_response(&self, serial: i32, message: &MessageType) -> io::Result<()> {
        if let Some((id, buf)) = encode_message(message) {
            // self.socket.write_i32(-serial).await?;
            // self.socket.write_u32(id).await?;
            // self.socket.write_all(&buf).await?;
            return Ok(());
        }

        error!("encode message error!");
        Err(io::Error::new(ErrorKind::Other, "encode message error!"))
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
                WriterMessage::Send(data) => {
                    if data.is_empty() {
                        yield_now().await;
                        continue;
                    }

                    if let Err(error) = writer.write_all(&data).await {
                        log::error!("Error when write_all {:?}", error);
                        break;
                    }
                }
                WriterMessage::Flush => {
                    if let Err(error) = writer.flush().await {
                        log::error!("Error when flushing {:?}", error);
                    }
                }
            }

            yield_now().await;
        }

        rx.close();
    }

    async fn poll_read<S>(&mut self, mut reader: ReadHalf<S>)
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let mut buffer = BytesMut::with_capacity(1024);

        // 循环读取数据
        loop {
            match reader.read_buf(&mut buffer).await {
                // n为0表示对端已经关闭连接。
                Ok(n) if n == 0 => {
                    debug!("socket[{}] closed.", self.addr);
                    // 客户端主动断开
                    self.close_session();
                    return;
                }
                Ok(_n) => {
                    loop {
                        if self.is_closed() {
                            break;
                        }

                        if let Ok(result) = try_extract_frame(&mut buffer) {
                            if let Some(frame) = result {
                                self.on_recv_pkg_frame(frame).await;
                            } else {
                                break;
                            }
                        } else {
                            debug!("data parsing failed");
                            // 消息解析错误主动断开
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
            Ok(message) => match self.on_recv_message(&message).await {
                Ok(msg) => {
                    let _ = self.send_response(serial, &msg).await;
                }
                Err(err) => {
                    trace!(
                        "request error: {}, message id: {}",
                        err,
                        get_message_id(&message)
                    );

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
                error!("pb parse error: {}", err);
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
}

// 数据粘包处理
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
