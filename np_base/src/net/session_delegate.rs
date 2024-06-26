use crate::net::WriterMessage;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
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
        if buffer.len() > 0 {
            if buffer[0] != 33u8 {
                return Err(anyhow!("Bad flag"));
            }
        }
        // 数据小于5字节,继续读取数据
        if buffer.len() < 5 {
            return Ok(None);
        }

        // 读取包长度
        let buf = buffer.get(1..5).unwrap();
        let len = BigEndian::read_u32(buf) as usize;

        // 超出最大限制
        if len <= 0 || len >= 1024 * 1024 * 5 {
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

    /// 收到一个完整的消息包
    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> anyhow::Result<()>;
}

pub type CreateSessionDelegateCallback = Box<dyn Fn() -> Box<dyn SessionDelegate> + Send + Sync>;
