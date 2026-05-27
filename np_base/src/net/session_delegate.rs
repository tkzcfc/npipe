use crate::net::WriterMessage;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
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
    /// 返回 `Bytes`（零拷贝引用计数），避免 `to_vec()` 的额外分配。
    /// 注意：这个函数只能使用消耗 buffer 数据的函数，否则框架会一直循环调用本函数来驱动处理消息
    async fn on_try_extract_frame(
        &mut self,
        buffer: &mut BytesMut,
    ) -> anyhow::Result<Option<Bytes>> {
        // buffer.split() 是 O(1): 分割 BytesMut 不做内存拷贝
        // .freeze() 将 BytesMut 转换为 Bytes，同样 O(1)
        let frame = buffer.split().freeze();
        Ok(Some(frame))
    }

    /// 收到一个完整的消息包
    ///
    /// 参数为 `Bytes`，克隆时只增加引用计数，不拷贝数据。
    async fn on_recv_frame(&mut self, frame: Bytes) -> anyhow::Result<()>;

    async fn on_recv_frame_from(
        &mut self,
        _frame: Bytes,
        _peer_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        anyhow::bail!("on_recv_frame_from is not implemented for this delegate")
    }

    async fn is_ready_for_read(&self) -> bool {
        true
    }
}

pub type CreateSessionDelegateCallback = Box<dyn Fn() -> Box<dyn SessionDelegate> + Send + Sync>;
