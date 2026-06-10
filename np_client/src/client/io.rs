//! 底层帧读写、心跳与消息编解码。

use super::now_secs;
use super::transport::{ClientTransport, IncomingFrame, TransportEvent};
use anyhow::anyhow;
use byteorder::{BigEndian, ByteOrder};
use bytes::{Bytes, BytesMut};
use np_proto::{
    generic,
    message_map::{encode_raw_message, get_message_id, get_message_size, MessageType},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

/// 心跳循环：定期发送 ping，检测连接活性。
pub(super) async fn ping_forever<S>(
    transport: ClientTransport<S>,
    last_active_secs: Arc<AtomicU64>,
    last_read_secs: Arc<AtomicU64>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// 心跳间隔（秒）。
    const PING_INTERVAL_SECS: u64 = 5;
    /// 软超时（秒）：写方向能通说明连接大概率活着。
    const PING_TIMEOUT_SECS: u64 = 15;
    /// 纯读方向的硬超时（秒）：即使写方向通畅（如 KCP/UDP sendto 永不断错），
    /// 超过此时间没有收到任何字节也判定连接已死。
    const HARD_READ_TIMEOUT_SECS: u64 = 60;
    loop {
        sleep(Duration::from_secs(1)).await;

        // 快速路径：最近有数据活动，跳过
        let elapsed_active = now_secs().saturating_sub(last_active_secs.load(Ordering::Relaxed));
        if elapsed_active < PING_INTERVAL_SECS {
            continue;
        }

        // 硬超时：基于纯读方向，KCP/QUIC 等基于 UDP 的传输也能正确检测断连
        let elapsed_read = now_secs().saturating_sub(last_read_secs.load(Ordering::Relaxed));
        if elapsed_read > HARD_READ_TIMEOUT_SECS {
            return Err(anyhow!(
                "ping timeout: no data received for {}s",
                elapsed_read
            ));
        }

        // 软超时：写方向能通说明 TCP 连接大概率活着，仅读不到数据不判死
        if elapsed_active > PING_TIMEOUT_SECS {
            return Err(anyhow!("ping timeout"));
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        transport
            .send_control_message(
                -2,
                &MessageType::GenericPing(generic::Ping {
                    ticks: nanos as i64,
                }),
            )
            .await?;

        // 成功发出 ping 说明写方向通畅，更新活跃时间避免拥塞链路上误判超时
        last_active_secs.store(now_secs(), Ordering::Relaxed);
    }
}

/// 持续读取传输路径上的帧，通过事件通道发送给客户端会话。
pub(super) async fn read_transport_events<S>(
    mut reader: ReadHalf<S>,
    path_id: Option<u64>,
    event_tx: mpsc::UnboundedSender<TransportEvent>,
    last_active_secs: Arc<AtomicU64>,
    last_read_secs: Arc<AtomicU64>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut buffer = BytesMut::with_capacity(65536);

    loop {
        match reader.read_buf(&mut buffer).await {
            Ok(0) => {
                let _ = event_tx.send(TransportEvent::Closed {
                    path_id,
                    reason: "disconnect from the server".to_string(),
                });
                return Err(anyhow!("disconnect from the server"));
            }
            Ok(_) => {
                let now = now_secs();
                last_active_secs.store(now, Ordering::Relaxed);
                last_read_secs.store(now, Ordering::Relaxed);

                loop {
                    if buffer.is_empty() {
                        break;
                    }

                    match try_extract_frame(&mut buffer) {
                        Ok(Some(frame)) => {
                            if event_tx
                                .send(TransportEvent::Frame(IncomingFrame { path_id, frame }))
                                .is_err()
                            {
                                return Ok(());
                            }
                        }
                        Ok(None) => break,
                        Err(err) => {
                            let _ = event_tx.send(TransportEvent::Closed {
                                path_id,
                                reason: err.to_string(),
                            });
                            return Err(err);
                        }
                    }
                }
            }
            Err(err) => {
                let _ = event_tx.send(TransportEvent::Closed {
                    path_id,
                    reason: err.to_string(),
                });
                return Err(err.into());
            }
        }
    }
}

/// 将消息编码为协议帧并写入指定的写半边。
///
/// 每次写入后立即 flush，确保 KCP 等基于 UDP 的传输及时发包。
#[inline]
pub(super) async fn package_and_send_message<S>(
    writer: Arc<Mutex<WriteHalf<S>>>,
    serial: i32,
    message: &MessageType,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    if let Some(message_id) = get_message_id(message) {
        let message_size = get_message_size(message);
        let mut buf = Vec::with_capacity(message_size + 14);

        byteorder::WriteBytesExt::write_u8(&mut buf, 33u8)?;
        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
        byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
        byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
        encode_raw_message(message, &mut buf);

        let mut w = writer.lock().await;
        w.write_all(&buf).await?;
        w.flush().await?;
        Ok(())
    } else {
        Err(anyhow!("Message id not found"))
    }
}

/// 粘包拆帧：从缓冲区中提取一帧完整消息，返回 `Bytes`（零拷贝）。
pub(super) fn try_extract_frame(buffer: &mut BytesMut) -> anyhow::Result<Option<Bytes>> {
    if !buffer.is_empty() && buffer[0] != 33u8 {
        return Err(anyhow!("Bad flag"));
    }
    if buffer.len() < 5 {
        return Ok(None);
    }

    let buf = buffer.get(1..5).unwrap();
    let len = BigEndian::read_u32(buf) as usize;

    if len >= 1024 * 1024 * 5 {
        return Err(anyhow!("Message too long"));
    }

    if buffer.len() < 5 + len {
        return Ok(None);
    }

    let frame = buffer.split_to(5 + len).split_off(5).freeze();

    Ok(Some(frame))
}
