use crate::net::tcp_session::WriterMessage;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use tokio::sync::mpsc::UnboundedSender;

#[async_trait]
pub trait SessionLogic
where
    Self: Sync + Send,
{
    /// 会话开始
    fn on_session_start(&mut self, session_id: u32, tx: UnboundedSender<WriterMessage>);

    /// 会话关闭
    async fn on_session_close(&mut self);

    /// 数据粘包处理
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
            return Err(anyhow!("Length too long"));
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
    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> bool;
}
