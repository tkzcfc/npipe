use crate::net::WriterMessage;
use async_trait::async_trait;
use bytes::BytesMut;
use std::net::SocketAddr;
use tokio::sync::mpsc::UnboundedSender;

#[async_trait]
pub trait SessionDelegate
where
    Self: Sync + Send,
{
    /// 会话开始
    ///
    /// [`session_id`] 会话id，大于0
    ///
    /// [`addr`] 对方地址
    ///
    /// [`tx`] 主动发送消息通道发送端
    async fn on_session_start(
        &mut self,
        session_id: u32,
        addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()>;

    /// 会话关闭
    async fn on_session_close(&mut self) -> anyhow::Result<()>;

    /// 数据粘包处理
    ///
    /// 注意：这个函数只能使用消耗 buffer 数据的函数，否则框架会一直循环调用本函数来驱动处理消息
    ///
    fn on_try_extract_frame(&self, buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
        // 此处使用 buffer.split().to_vec(); 而不是 buffer.to_vec();
        // 因为split().to_vec()更高效，少了一次内存分配和拷贝
        // 并且在 on_try_extract_frame 函数中只能使用消耗 buffer 数据的函数，否则框架会一直循环调用 on_try_extract_frame 来驱动处理消息
        let frame = buffer.split().to_vec();
        Ok(Some(frame))
    }

    /// 收到一个完整的消息包
    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> anyhow::Result<()>;
}

pub type CreateSessionDelegateCallback = Box<dyn Fn() -> Box<dyn SessionDelegate> + Send + Sync>;
