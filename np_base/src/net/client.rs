use bytes::BytesMut;
use log::{debug, error};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

enum ChannelMessage {
    Disconnect,
    RecvMessage(Vec<u8>),
}

pub struct Client {
    addr: SocketAddr,
    writer: Option<WriteHalf<TcpStream>>,
    closed: bool,
    rx: Option<UnboundedReceiver<ChannelMessage>>,
    callback: ExtractFrameCallback,
}

pub type ExtractFrameCallback = fn(buffer: &mut BytesMut) -> io::Result<Option<Vec<u8>>>;

impl Client {
    pub fn new(addr: SocketAddr, callback: ExtractFrameCallback) -> Client {
        Client {
            addr,
            writer: None,
            closed: true,
            rx: None,
            callback,
        }
    }

    // 连接服务器
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

        self.rx = Some(rx);
        let addr = self.addr.clone();

        let extract_frame_callback = self.callback.clone();
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
                            if let Ok(result) = extract_frame_callback(&mut buffer) {
                                if let Some(frame) = result {
                                    if let Err(error) = tx.send(ChannelMessage::RecvMessage(frame))
                                    {
                                        error!(
                                            "Send channel message(RecvMessage) error: {}",
                                            error
                                        );
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
                                    error!("Send channel message(Disconnect) error: {}", error);
                                }
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from socket[{}]: {}", addr, e);
                        // socket读错误,主动断开
                        if let Err(error) = tx.send(ChannelMessage::Disconnect) {
                            error!("Send channel message(Disconnect) error: {}", error);
                        }
                        return;
                    }
                }
            }
        });

        Ok(())
    }

    // 是否处于连接状态
    #[inline]
    pub fn is_connect(&self) -> bool {
        !self.closed
    }

    // 断开连接
    pub fn disconnect(&mut self) {
        self.closed = true;

        if let Some(mut rx) = self.rx.take() {
            rx.close();
        }

        if let Some(mut writer) = self.writer.take() {
            tokio::spawn(async move {
                if let Err(error) = writer.shutdown().await {
                    error!("Socket shutdown error: {}", error)
                }
            });
        }
    }

    // 发送消息
    pub async fn send(&mut self, buf: &Vec<u8>, flush: bool) -> io::Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.write_all(buf).await?;
            if flush {
                writer.flush().await?;
            }
            return Ok(());
        }
        Err(io::Error::new(ErrorKind::NotConnected, "Not connected"))
    }

    // 接收消息
    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        if let Some(ref mut rx) = self.rx {
            if let Ok(channel_message) = rx.try_recv() {
                match channel_message {
                    ChannelMessage::Disconnect => {
                        self.disconnect();
                    }
                    ChannelMessage::RecvMessage(message) => {
                        return Some(message);
                    }
                }
            }
        }
        None
    }
}
