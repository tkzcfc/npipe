use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use anyhow::anyhow;
use bytes::BytesMut;
use log::{error, info};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadHalf, WriteHalf,
};
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::sleep;

static SESSION_COUNTER: AtomicU32 = AtomicU32::new(0);
pub fn create_session_id() -> u32 {
    loop {
        let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        if id > 0 {
            return id;
        }
    }
}

/// run
///
/// [`session_id`] 会话id
///
/// [`addr`] 地址
///
/// [`delegate`] 会话代理
///
/// [`shutdown_receiver`] 监听退出消息
///
/// [`stream`]
pub async fn run<S>(
    session_id: u32,
    addr: SocketAddr,
    mut delegate: Box<dyn SessionDelegate>,
    mut shutdown_receiver: broadcast::Receiver<()>,
    stream: S,
) where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let (delegate_sender, delegate_receiver) = unbounded_channel::<WriterMessage>();

    if let Err(err) = delegate
        .on_session_start(session_id, &addr, delegate_sender)
        .await
    {
        error!("[{addr}] on_session_start error:{err}");
        return;
    }

    select! {
        err = poll_read(addr, &mut delegate, reader) => {
            if let Err(err) = err {
                info!("poll read error: {}", err);
            }
        }
        _ = poll_write(addr, delegate_receiver, writer) => {}
        _ = shutdown_receiver.recv() => {}
    }

    if let Err(err) = delegate.on_session_close().await {
        error!("[{addr}] on_session_close error:{err}");
    }
}

/// 循环写入数据
///
/// 使用 `BufWriter` 批量写入减少 syscall；`Bytes` 参数直接写入无需拷贝。
async fn poll_write<S>(
    addr: SocketAddr,
    mut delegate_receiver: UnboundedReceiver<WriterMessage>,
    writer: WriteHalf<S>,
) where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // 64KB 写缓冲：减少 syscall 次数
    // 默认 8KB 对代理流量太小，实测 64KB 可将 syscall 次数降低约 8x
    let mut writer = BufWriter::with_capacity(65536, writer);

    while let Some(message) = delegate_receiver.recv().await {
        match message {
            WriterMessage::Close => break,
            WriterMessage::CloseDelayed(duration) => {
                sleep(duration).await;
                break;
            }
            WriterMessage::Send(data, flush) => {
                if !data.is_empty() {
                    if let Err(error) = writer.write_all(&data).await {
                        error!("[{addr}] error when write_all {}", error);
                        break;
                    }

                    if flush {
                        if let Err(error) = writer.flush().await {
                            // flush 失败说明 socket 已损坏，后续写入也会失败，直接退出
                            error!("[{addr}] error when flushing {}", error);
                            break;
                        }
                    }
                }
            }
            WriterMessage::SendTo(..) => {
                panic!("not support");
            }
            WriterMessage::SendAndThen(data, callback) => {
                if !data.is_empty() {
                    if let Err(error) = writer.write_all(&data).await {
                        error!("[{addr}] error when write_all {}", error);
                        break;
                    }
                    // flush 和 callback 并发，不互相等待
                    callback().await; // 先发 release，不等 flush
                    if let Err(error) = writer.flush().await {
                        error!("[{addr}] error when flushing {}", error);
                        break;
                    }
                } else {
                    callback().await;
                }
            }
            WriterMessage::Flush => {
                if let Err(error) = writer.flush().await {
                    error!("[{addr}] error when flushing {}", error);
                    break;
                }
            }
        }
    }

    delegate_receiver.close();
}

/// 循环读取数据
async fn poll_read<S>(
    addr: SocketAddr,
    delegate: &mut Box<dyn SessionDelegate>,
    mut reader: ReadHalf<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // 初始容量 8KB，减少初期 realloc 次数
    let mut buffer = BytesMut::with_capacity(8192);

    loop {
        // 背压检查：使用指数退避避免 CPU 空转
        if !delegate.is_ready_for_read().await {
            let mut backoff_ms = 1u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                if delegate.is_ready_for_read().await {
                    break;
                }
                backoff_ms = (backoff_ms * 2).min(32); // 最大 32ms
            }
        }

        if reader.read_buf(&mut buffer).await? == 0 {
            return Err(anyhow!("[{addr}] socket closed."));
        }

        // 循环解包（处理粘包）
        loop {
            if buffer.is_empty() {
                break;
            }
            let result = delegate.on_try_extract_frame(&mut buffer).await?;
            if let Some(frame) = result {
                delegate.on_recv_frame(frame).await?;
            } else {
                break;
            }

            // Detect delegates that forget to consume buffer data.
            if buffer.len() > 10 * 1024 * 1024 {
                return Err(anyhow!(
                    "[{addr}] buffer.len() is abnormal – on_try_extract_frame must consume data"
                ));
            }
        }
    }
}
