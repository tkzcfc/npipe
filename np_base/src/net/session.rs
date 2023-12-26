use crate::net::session_logic::SessionLogic;
use bytes::BytesMut;
use log::{debug, error};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadHalf, WriteHalf,
};
use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
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
    #[allow(dead_code)]
    session_id: u32,
    closed: bool,
    logic: Box<dyn SessionLogic>,
}

impl Drop for Session {
    fn drop(&mut self) {}
}

impl Session {
    pub fn new(
        tx: UnboundedSender<WriterMessage>,
        addr: SocketAddr,
        session_id: u32,
        logic: Box<dyn SessionLogic>,
    ) -> Session {
        Session {
            tx,
            addr,
            session_id,
            closed: false,
            logic,
        }
    }

    // 是否关闭会话
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.closed || self.tx.is_closed()
    }

    // 获取会话id
    #[inline]
    #[allow(dead_code)]
    pub fn get_session_id(&self) -> u32 {
        self.session_id
    }

    // 关闭会话
    #[inline]
    pub fn close_session(&mut self) {
        self.closed = true;
        let _ = self.tx.send(WriterMessage::Close);
    }

    pub async fn run<S>(
        &mut self,
        rx: UnboundedReceiver<WriterMessage>,
        reader: ReadHalf<S>,
        writer: WriteHalf<S>,
    ) where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        self.logic.on_session_start(self.tx.clone());
        select! {
            _ = self.poll_read(reader) => {}
            _ = Self::poll_write(rx, writer) => {}
        }
        self.logic.on_session_close().await;
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
                    loop {
                        if self.is_closed() {
                            return;
                        }

                        // 处理数据粘包
                        match self.logic.on_try_extract_frame(&mut buffer) {
                            Ok(result) => {
                                if let Some(frame) = result {
                                    // 收到完整消息
                                    if !self.logic.on_recv_frame(frame).await {
                                        // 消息处理失败
                                        self.close_session();
                                        return;
                                    }
                                } else {
                                    break;
                                }
                            }
                            Err(error) => {
                                error!("Try extract frame error {}", error);
                                self.close_session();
                                return;
                            }
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
}