use crate::net::session::WriterMessage;
use async_trait::async_trait;
use bytes::BytesMut;
use std::io;
use tokio::sync::mpsc::UnboundedSender;

#[async_trait]
pub trait SessionLogic
where
    Self: Sync + Send,
{
    // 会话开始
    fn on_session_start(&mut self, tx: UnboundedSender<WriterMessage>);

    // 会话关闭
    async fn on_session_close(&mut self);

    // 数据粘包处理
    fn on_try_extract_frame(&self, buffer: &mut BytesMut) -> io::Result<Option<Vec<u8>>>;

    // 收到一个完整的消息包
    async fn on_recv_frame(&self, frame: Vec<u8>) -> bool;
}
