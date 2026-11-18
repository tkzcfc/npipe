use crate::net::WriterMessage;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use std::net::SocketAddr;
use std::time::Instant;
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

    /// 读取循环在下一次 `read` 上应遵守的截止时间。
    ///
    /// 默认 `None` 表示无限等待（保持原有会话读循环语义）。
    /// 仅在需要有界嗅探时由具体 Delegate 返回固定截止时间；到期必须唤醒读取，
    /// 不能等到下一次对端发数据才检查。
    fn read_deadline(&self) -> Option<Instant> {
        None
    }

    /// 当前缓冲基础上，下一次最多再读多少字节。
    ///
    /// `None` 表示不限制（默认）。`Some(0)` 表示已达上限，读取循环不应再读，
    /// 而应调用 [`on_read_limit`]。
    fn max_additional_read(&self, _buffer_len: usize) -> Option<usize> {
        None
    }

    /// 读取截止时间到期。`buffer` 仍包含尚未消费的已读字节，实现应自行决定
    /// 转发或关闭，并在转发时消费这些字节以免丢失或重复。
    async fn on_read_timeout(&mut self, _buffer: &mut BytesMut) -> anyhow::Result<()> {
        Ok(())
    }

    /// 嗅探/预读缓冲达到上限。语义与超时收口相同，但原因是长度限制。
    async fn on_read_limit(&mut self, buffer: &mut BytesMut) -> anyhow::Result<()> {
        self.on_read_timeout(buffer).await
    }

    /// 对端在仍有未消费缓冲时读到 EOF（含写半关闭）。
    ///
    /// 默认什么都不做，由读取循环按原语义返回 “socket closed”。
    /// 若实现在此处建立了转发并消费了缓冲，读取循环仍会因 EOF 结束会话。
    async fn on_read_eof(&mut self, _buffer: &mut BytesMut) -> anyhow::Result<()> {
        Ok(())
    }

    async fn is_ready_for_read(&self) -> bool {
        true
    }
}

pub type CreateSessionDelegateCallback = Box<dyn Fn() -> Box<dyn SessionDelegate> + Send + Sync>;
