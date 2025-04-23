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
use tokio::task::yield_now;
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
                info!("poll read error: {}", err.to_string());
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
async fn poll_write<S>(
    addr: SocketAddr,
    mut delegate_receiver: UnboundedReceiver<WriterMessage>,
    writer: WriteHalf<S>,
) where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut writer = BufWriter::new(writer);

    while let Some(message) = delegate_receiver.recv().await {
        match message {
            WriterMessage::Close => break,
            WriterMessage::CloseDelayed(duration) => {
                sleep(duration).await;
                break;
            }
            WriterMessage::Send(data, flush) => {
                if data.is_empty() {
                    yield_now().await;
                    continue;
                }

                if let Err(error) = writer.write_all(&data).await {
                    error!("[{addr}] error when write_all {:?}", error);
                    break;
                }

                if flush {
                    if let Err(error) = writer.flush().await {
                        error!("[{addr}] error when flushing {:?}", error);
                    }
                }
            }
            WriterMessage::SendTo(_, ..) => {
                panic!("not support");
            }
            WriterMessage::SendAndThen(data, callback) => {
                if data.is_empty() {
                    callback().await;
                    continue;
                }

                if let Err(error) = writer.write_all(&data).await {
                    error!("[{addr}] error when write_all {:?}", error);
                    break;
                }
                if let Err(error) = writer.flush().await {
                    error!("[{addr}] error when flushing {:?}", error);
                }
                callback().await;
            }
            WriterMessage::Flush => {
                if let Err(error) = writer.flush().await {
                    error!("[{addr}] error when flushing {:?}", error);
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
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        while !delegate.is_ready_for_read().await {
            // 暂停读取数据
            yield_now().await;
        }

        if reader.read_buf(&mut buffer).await? == 0 {
            // 客户端主动断开
            return Err(anyhow!("[{addr}] socket closed."));
        }

        // 循环解包
        loop {
            if buffer.is_empty() {
                break;
            }
            // 处理数据粘包
            let result = delegate.on_try_extract_frame(&mut buffer).await?;
            if let Some(frame) = result {
                // 收到完整消息
                delegate.on_recv_frame(frame).await?;
            } else {
                // 消息包接收还未完成
                break;
            }

            if buffer.capacity() > 1024 * 1024 * 10 {
                error!("[{addr}] The buffer size is abnormal ({}), whether the buffer data has not been consumed",buffer.capacity());
            }
        }
    }
}
