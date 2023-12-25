use byteorder::BigEndian;
use log::error;
use np_base::net::session::WriterMessage;
use np_proto::generic;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

pub type PlayerId = u32;

#[warn(dead_code)]
pub struct Player {
    tx: Option<UnboundedSender<WriterMessage>>,
    player_id: PlayerId,
    session_id: u32,
}

impl Player {
    pub fn new(player_id: PlayerId) -> Arc<RwLock<Player>> {
        Arc::new(RwLock::new(Player {
            tx: None,
            player_id,
            session_id: 032,
        }))
    }

    // 获取玩家Id
    #[inline]
    pub fn get_player_id(&self) -> PlayerId {
        self.player_id
    }

    // 获取会话id
    #[inline]
    pub fn get_session_id(&self) -> u32 {
        self.session_id
    }

    // 是否在线
    #[inline]
    pub fn is_online(&self) -> bool {
        self.session_id > 0
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
    pub async fn send_response(&self, serial: i32, message: &MessageType) -> io::Result<()> {
        self.package_and_send_message(serial, message, true).await
    }

    // #[inline]
    // pub async fn send_request(&self, _message: &MessageType) -> io::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    pub async fn send_push(&self, message: &MessageType) -> io::Result<()> {
        self.package_and_send_message(0, message, true).await
    }

    #[inline]
    pub fn flush(&self) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Flush);
        }
    }

    #[inline]
    pub fn close_session(&mut self) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Close);
        }
    }

    // 重置会话信息
    #[inline]
    fn reset_session_info(&mut self) {
        self.session_id = 0;
        self.tx.take();
    }

    // 玩家上线
    pub async fn on_connect_session(
        &mut self,
        session_id: u32,
        tx: UnboundedSender<WriterMessage>,
    ) {
        assert_eq!(self.is_online(), false);
        self.session_id = session_id;
        self.tx = Some(tx);
    }

    // 玩家离线
    pub async fn on_disconnect_session(&mut self) {
        self.reset_session_info();
    }

    // 玩家被顶号，需要对旧的会话发送一些消息
    pub async fn on_terminate_old_session(&mut self) {
        //

        // 重置会话信息
        self.reset_session_info();
    }

    // 玩家收到消息
    pub async fn handle_request(&mut self, message: MessageType) -> io::Result<MessageType> {
        // 客户端请求的消息，服务器未实现
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::InterfaceAbsent.into(),
            message: "interface absent".into(),
        }))
    }
}
