use crate::peer::package_and_send_message;
use log::trace;
use np_base::net::WriterMessage;
use np_proto::generic;
use np_proto::message_map::MessageType;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

pub type PlayerId = u32;

pub struct Player {
    tx: Option<UnboundedSender<WriterMessage>>,
    // 玩家id
    player_id: PlayerId,
    // 会话id
    session_id: u32,
}

impl Player {
    pub fn new(player_id: PlayerId) -> Arc<RwLock<Player>> {
        Arc::new(RwLock::new(Player {
            tx: None,
            player_id,
            session_id: 0,
        }))
    }

    // 获取玩家Id
    #[inline]
    pub fn get_player_id(&self) -> PlayerId {
        self.player_id
    }

    // 获取会话id
    #[inline]
    #[allow(dead_code)]
    pub fn get_session_id(&self) -> u32 {
        self.session_id
    }

    // 是否在线
    #[inline]
    pub fn is_online(&self) -> bool {
        self.session_id > 0
    }

    #[inline]
    #[allow(dead_code)]
    pub async fn send_response(&self, serial: i32, message: &MessageType) -> anyhow::Result<()> {
        assert!(serial < 0);
        package_and_send_message(&self.tx, -serial, message, true).await
    }

    // #[inline]
    // pub async fn send_request(&self, _message: &MessageType) -> anyhow::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    #[allow(dead_code)]
    pub async fn send_push(&self, message: &MessageType) -> anyhow::Result<()> {
        package_and_send_message(&self.tx, 0, message, true).await
    }

    #[inline]
    #[allow(dead_code)]
    pub fn flush(&self) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Flush);
        }
    }

    #[inline]
    pub fn close_session(&mut self) {
        trace!("close_session, player_id: {}", self.player_id);
        if let Some(ref tx) = self.tx {
            let _ = tx.send(WriterMessage::Close);
        }
    }

    // 重置会话信息
    #[inline]
    fn reset_session_info(&mut self) {
        trace!("reset_session_info, player_id: {}", self.player_id);
        self.session_id = 0;
        self.tx.take();
    }

    // 玩家上线
    pub async fn on_connect_session(
        &mut self,
        session_id: u32,
        tx: UnboundedSender<WriterMessage>,
    ) {
        trace!("on_connect_session, player_id: {}", self.player_id);
        assert!(!self.is_online());
        self.session_id = session_id;
        self.tx = Some(tx);
    }

    // 玩家离线
    #[allow(dead_code)]
    pub async fn on_disconnect_session(&mut self) {
        trace!("on_disconnect_session, player_id: {}", self.player_id);
        self.reset_session_info();
    }

    // 玩家被顶号，需要对旧的会话发送一些消息
    pub async fn on_terminate_old_session(&mut self) {
        trace!("on_terminate_old_session, player_id: {}", self.player_id);
        //
        self.close_session();

        // 重置会话信息
        self.reset_session_info();
    }

    // 玩家收到消息
    pub async fn handle_request(&mut self, _message: MessageType) -> anyhow::Result<MessageType> {
        // 客户端请求的消息，服务器未实现
        Ok(MessageType::GenericError(generic::Error {
            number: generic::ErrorCode::InterfaceAbsent.into(),
            message: "interface absent".into(),
        }))
    }
}
