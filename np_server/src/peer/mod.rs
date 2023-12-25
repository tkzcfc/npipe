mod handle_push;
mod handle_request;
mod handle_response;

use crate::player::Player;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::error;
use np_base::net::session::WriterMessage;
use np_base::net::session_logic::SessionLogic;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use np_proto::{generic, message_map};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

pub struct Peer {
    tx: Option<UnboundedSender<WriterMessage>>,
    player: Option<Arc<RwLock<Player>>>,
}

impl Peer {
    pub(crate) fn new() -> Self {
        Peer {
            tx: None,
            player: None,
        }
    }

    #[inline]
    pub async fn package_and_send_message(
        &self,
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

            if let Some(ref tx) = self.tx {
                if let Err(error) = tx.send(WriterMessage::Send(buf, flush)) {
                    error!("Send message error: {}", error);
                }
            } else {
                error!("Send message error: tx is None");
            }
        }
        Ok(())
    }

    #[inline]
    pub(crate) async fn send_response(&self, serial: i32, message: &MessageType) -> io::Result<()> {
        self.package_and_send_message(serial, message, true).await
    }

    // #[inline]
    // pub(crate) async fn send_request(&self, _message: &MessageType) -> io::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    pub(crate) async fn send_push(&self, message: &MessageType) -> io::Result<()> {
        self.package_and_send_message(0, message, true).await
    }

    pub async fn handle_message(
        &self,
        serial: i32,
        message: MessageType,
    ) -> io::Result<MessageType> {
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
impl SessionLogic for Peer {
    fn on_session_start(&mut self, tx: UnboundedSender<WriterMessage>) {
        self.tx = Some(tx);
    }

    // 会话关闭回调
    async fn on_session_close(&mut self) {
        self.tx = None;
    }

    // 收到一个完整的消息包
    async fn on_recv_frame(&self, frame: Vec<u8>) -> bool {
        if frame.len() < 8 {
            return false;
        }
        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

        match message_map::decode_message(msg_id, &frame[8..]) {
            Ok(message) => match self.handle_message(serial, message).await {
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

                return false;
            }
        }

        true
    }
}
