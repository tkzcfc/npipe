mod handle_push;
mod handle_request;
mod handle_response;

use crate::global::config::GLOBAL_CONFIG;
use crate::player::Player;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::{debug, error, trace};
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::WriterMessage;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use np_proto::{generic, message_map};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::time::Instant;

pub struct Peer {
    tx: Option<UnboundedSender<WriterMessage>>,
    player: Option<Arc<RwLock<Player>>>,
    session_id: u32,
    traffic_forward_writer: Option<WriteHalf<TcpStream>>,
}

impl Peer {
    pub(crate) fn new() -> Self {
        Peer {
            tx: None,
            player: None,
            session_id: 0,
            traffic_forward_writer: None,
        }
    }

    #[inline]
    pub(crate) async fn send_response(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        assert!(serial < 0);
        package_and_send_message(&self.tx, -serial, message, true).await
    }

    // #[inline]
    // pub(crate) async fn send_request(&self, _message: &MessageType) -> anyhow::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn send_push(&self, message: &MessageType) -> anyhow::Result<()> {
        package_and_send_message(&self.tx, 0, message, true).await
    }

    pub async fn handle_message(
        &mut self,
        serial: i32,
        message: MessageType,
    ) -> anyhow::Result<MessageType> {
        match serial {
            s if s < 0 => self.handle_request(message).await,
            s if s > 0 => {
                self.handle_response(message).await?;
                Ok(MessageType::None)
            }
            _ => {
                // serial == 0
                self.handle_push(message).await?;
                Ok(MessageType::None)
            }
        }
    }

    // 模拟http 404请求结果
    async fn send_http_404_response(&self) -> anyhow::Result<()> {
        if let Some(ref tx) = self.tx {
            let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
            let formatted_time = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

            let str = format!(
                "HTTP/1.1 400 Bad Request\r\n\
                Server: nginx\r\n\
                Date: {formatted_time}\r\n\
                Content-Type: text/html\r\n\
                Content-Length: 150\r\n\
                Connection: close\r\n\
                \r\n\
                <html>\r\n\
                <head><title>400 Bad Request</title></head>\r\n\
                <body>\r\n\
                <center><h1>400 Bad Request</h1></center>\r\n\
                <hr><center>nginx</center>\r\n\
                </body>\r\n\
                </html>\r\n\
                "
            );

            tx.send(WriterMessage::Send(str.into(), true))?;
            tx.send(WriterMessage::Close)?;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    /// 创建流量转发通道
    async fn create_traffic_forward_channel(&mut self) -> anyhow::Result<()> {
        if GLOBAL_CONFIG.illegal_traffic_forward.is_empty() {
            return Err(anyhow!("no config illegal_traffic_forward"));
        }
        if let Some(ref tx) = self.tx {
            let stream = TcpStream::connect(&GLOBAL_CONFIG.illegal_traffic_forward).await?;

            // set tcp keepalive
            let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
            let sf = SockRef::from(&stream);
            sf.set_tcp_keepalive(&ka)?;

            let (mut reader, writer) = tokio::io::split(stream);

            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buffer = BytesMut::with_capacity(4096);

                while let Ok(size) = reader.read_buf(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    let frame = buffer.split().to_vec();
                    let _ = tx.send(WriterMessage::Send(frame, true));
                }

                let _ = tx.send(WriterMessage::Close);
            });
            self.traffic_forward_writer = Some(writer);
            Ok(())
        } else {
            Err(anyhow!("tx is none"))
        }
    }
}

#[async_trait]
impl SessionDelegate for Peer {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        _addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        self.tx = Some(tx);
        self.session_id = session_id;
        Ok(())
    }

    // 会话关闭回调
    async fn on_session_close(&mut self) -> anyhow::Result<()> {
        self.tx.take();
        // 清退对应玩家
        if let Some(player) = self.player.take() {
            if player.read().await.get_session_id() == self.session_id {
                player.write().await.on_disconnect_session().await;
            }
        }
        // 关闭流量转发通道
        if let Some(mut writer) = self.traffic_forward_writer.take() {
            let _ = writer.shutdown().await;
        }
        Ok(())
    }

    /// 数据粘包处理
    ///
    /// 注意：这个函数只能使用消耗 buffer 数据的函数，否则框架会一直循环调用本函数来驱动处理消息
    ///
    async fn on_try_extract_frame(
        &mut self,
        buffer: &mut BytesMut,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        if !buffer.is_empty()
            && buffer[0] != 33u8
            && self.traffic_forward_writer.is_none()
            && self.create_traffic_forward_channel().await.is_err()
        {
            debug!("bad flag");
            self.send_http_404_response().await?;
            return Err(anyhow!("Bad flag"));
        }

        if let Some(ref mut writer) = self.traffic_forward_writer {
            let frame = buffer.split().to_vec();
            writer.write_all(&frame).await?;
            return Ok(None);
        }

        // 数据小于5字节,继续读取数据
        if buffer.len() < 5 {
            return Ok(None);
        }

        // 读取包长度
        let buf = buffer.get(1..5).unwrap();
        let len = BigEndian::read_u32(buf) as usize;

        // 超出最大限制
        if len == 0 || len >= 1024 * 1024 * 2 {
            debug!("Message too long");
            self.send_http_404_response().await?;
            return Err(anyhow!("Message too long"));
        }

        // 数据不够,继续读取数据
        if buffer.len() < 5 + len {
            return Ok(None);
        }

        // 拆出这个包的数据
        let frame = buffer.split_to(5 + len).split_off(5).to_vec();

        Ok(Some(frame))
    }

    // 收到一个完整的消息包
    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
        if frame.len() < 8 {
            debug!("message length is too small");
            self.send_http_404_response().await?;
            return Err(anyhow!("message length is too small"));
        }
        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

        match message_map::decode_message(msg_id, &frame[8..]) {
            Ok(message) => {
                let start_time = Instant::now();

                let result = self.handle_message(serial, message).await;

                // 记录耗时比较长的接口
                let ms = Instant::now().duration_since(start_time).as_millis();
                if ms > 20 {
                    trace!("Request {} consumes {}ms", msg_id, ms);
                }

                match result {
                    Ok(msg) => {
                        if serial < 0 {
                            if let MessageType::None = msg {
                                // 请求不应该不回复
                                error!("The response to request {} is empty", msg_id);
                                self.send_response(
                                    serial,
                                    &MessageType::GenericError(generic::Error {
                                        number: generic::ErrorCode::InternalError.into(),
                                        message: "response is empty".to_string(),
                                    }),
                                )
                                .await?;
                            } else {
                                self.send_response(serial, &msg).await?;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Request error: {}, message id: {}", err, msg_id);

                        self.send_response(
                            serial,
                            &MessageType::GenericError(generic::Error {
                                number: generic::ErrorCode::InternalError.into(),
                                message: format!("{}", err),
                            }),
                        )
                        .await?;
                    }
                }
            }
            Err(err) => {
                debug!("decode message error: {err}");
                self.send_http_404_response().await?;

                // // 消息解码失败
                // self.send_response(
                //     serial,
                //     &MessageType::GenericError(generic::Error {
                //         number: generic::ErrorCode::InternalError.into(),
                //         message: format!("{}", err),
                //     }),
                // )
                // .await?;

                return Err(anyhow!(err));
            }
        }

        Ok(())
    }
}

#[inline]
pub(crate) async fn package_and_send_message(
    tx: &Option<UnboundedSender<WriterMessage>>,
    serial: i32,
    message: &MessageType,
    flush: bool,
) -> anyhow::Result<()> {
    if let Some(ref tx) = tx {
        if let Some(message_id) = get_message_id(message) {
            let message_size = get_message_size(message);
            let mut buf = Vec::with_capacity(message_size + 14);

            byteorder::WriteBytesExt::write_u8(&mut buf, 33u8)?;
            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
            byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
            encode_raw_message(message, &mut buf);

            if let Err(error) = tx.send(WriterMessage::Send(buf, flush)) {
                error!("Send message error: {}", error);
            }
        }
    } else {
        debug!("Send message error: tx is None");
    }
    Ok(())
}
