mod handle_push;
mod handle_request;
mod handle_response;

use crate::player::Player;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use log::{error, trace};
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::WriterMessage;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use np_proto::{generic, message_map};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::time::Instant;

pub struct Peer {
    tx: Option<UnboundedSender<WriterMessage>>,
    player: Option<Arc<RwLock<Player>>>,
    session_id: u32,
}

impl Peer {
    pub(crate) fn new() -> Self {
        Peer {
            tx: None,
            player: None,
            session_id: 0,
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
        return if serial < 0 {
            self.handle_request(message).await
        } else if serial > 0 {
            self.handle_response(message).await?;
            Ok(MessageType::None)
        } else {
            self.handle_push(message).await?;
            Ok(MessageType::None)
        };
    }
}

#[async_trait]
impl SessionDelegate for Peer {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        _addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) {
        self.tx = Some(tx);
        self.session_id = session_id;
    }

    // 会话关闭回调
    async fn on_session_close(&mut self) {
        self.tx.take();
        // 清退对应玩家
        if let Some(player) = self.player.take() {
            player.write().await.on_disconnect_session().await;
        }
    }

    // 收到一个完整的消息包
    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> bool {
        if frame.len() < 8 {
            return false;
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
                                let _ = self
                                    .send_response(
                                        serial,
                                        &MessageType::GenericError(generic::Error {
                                            number: generic::ErrorCode::InternalError.into(),
                                            message: "response is empty".to_string(),
                                        }),
                                    )
                                    .await;
                            } else {
                                if let Err(err) = self.send_response(serial, &msg).await {
                                    error!("Send response error: {}", err);
                                }
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
                }
            }
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

                return false;
            }
        }

        true
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
        error!("Send message error: tx is None");
    }
    Ok(())
}
