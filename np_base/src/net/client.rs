use anyhow::anyhow;
use bytes::BytesMut;
use log::{debug, error};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter, ReadHalf, WriteHalf};
use tokio::net::{TcpSocket, TcpStream};
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::yield_now;

enum ChannelMessage {
    OnDisconnect,
    OnRecvMessage(Vec<u8>),
    DoWriteData(Vec<u8>, bool),
    DoDisconnect,
}

pub struct Client {
    addr: SocketAddr,
    closed: bool,
    rx: Option<UnboundedReceiver<ChannelMessage>>,
    tx: Option<UnboundedSender<ChannelMessage>>,
    callback: ExtractFrameCallback,
}

pub type ExtractFrameCallback = fn(buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>>;

impl Client {
    pub fn new(addr: SocketAddr, callback: ExtractFrameCallback) -> Client {
        Client {
            addr,
            closed: true,
            rx: None,
            tx: None,
            callback,
        }
    }

    /// 连接服务器
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.disconnect();

        let socket = if self.addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(self.addr).await?;
        let (reader, writer) = tokio::io::split(stream);

        self.closed = false;

        let (tx1, rx1) = unbounded_channel();
        self.rx = Some(rx1);

        let (tx2, rx2) = unbounded_channel();
        self.tx = Some(tx2);

        let addr = self.addr.clone();
        let extract_frame_callback = self.callback.clone();
        tokio::spawn(async move {
            select! {
            _ = Self::poll_read(tx1.clone(), reader, addr, extract_frame_callback) => {},
            _ = Self::poll_write(rx2, writer) => {},
            }

            if !tx1.is_closed() {
                if let Err(error) = tx1.send(ChannelMessage::OnDisconnect) {
                    error!("Send channel message error: {}", error);
                }
            }
        });

        Ok(())
    }

    pub fn reset_addr(&mut self, addr: SocketAddr) {
        if self.addr != addr {
            self.addr = addr;
            self.disconnect();
        }
    }

    /// 是否处于连接状态
    #[inline]
    pub fn is_connect(&self) -> bool {
        !self.closed
    }

    /// 断开连接
    pub fn disconnect(&mut self) {
        self.closed = true;

        if let Some(mut rx) = self.rx.take() {
            rx.close();
        }

        if let Some(tx) = self.tx.take() {
            if let Err(error) = tx.send(ChannelMessage::DoDisconnect) {
                error!("Send Disconnect error: {}", error);
            }
        }
    }

    /// 发送消息
    pub fn send(&self, buf: Vec<u8>, flush: bool) -> anyhow::Result<()> {
        if let Some(ref tx) = self.tx {
            if let Err(error) = tx.send(ChannelMessage::DoWriteData(buf, flush)) {
                error!("Send Disconnect error: {}", error);
            }
            return Ok(());
        }
        Err(anyhow!("Not connected"))
    }

    /// 接收消息
    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        if let Some(ref mut rx) = self.rx {
            if let Ok(channel_message) = rx.try_recv() {
                match channel_message {
                    ChannelMessage::OnDisconnect => {
                        self.disconnect();
                    }
                    ChannelMessage::OnRecvMessage(message) => {
                        return Some(message);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    async fn poll_write(mut rx: UnboundedReceiver<ChannelMessage>, writer: WriteHalf<TcpStream>) {
        let mut writer = BufWriter::new(writer);
        while let Some(message) = rx.recv().await {
            match message {
                ChannelMessage::DoWriteData(data, flush) => {
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
                ChannelMessage::DoDisconnect => {
                    if let Err(error) = writer.shutdown().await {
                        error!("Error when shutdown {:?}", error);
                    }
                    break;
                }
                _ => {}
            }
            yield_now().await;
        }
        rx.close();
    }

    async fn poll_read(
        tx: UnboundedSender<ChannelMessage>,
        mut reader: ReadHalf<TcpStream>,
        addr: SocketAddr,
        extract_frame_callback: ExtractFrameCallback,
    ) {
        let mut buffer = BytesMut::with_capacity(1024);
        loop {
            if tx.is_closed() {
                break;
            }

            match reader.read_buf(&mut buffer).await {
                // n为0表示对端已经关闭连接。
                Ok(n) if n == 0 => {
                    debug!("socket[{}] closed.", addr);
                    return;
                }
                Ok(_n) => {
                    loop {
                        if let Ok(result) = extract_frame_callback(&mut buffer) {
                            if let Some(frame) = result {
                                if let Err(error) = tx.send(ChannelMessage::OnRecvMessage(frame)) {
                                    error!("Send channel message(RecvMessage) error: {}", error);
                                    return;
                                }
                            } else {
                                // 数据包长度不够,继续读
                                break;
                            }
                        } else {
                            error!("Message too long");
                            // 消息过长, 主动断开
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket[{}]: {}", addr, e);
                    // socket读错误,主动断开
                    return;
                }
            }
        }
    }
}
