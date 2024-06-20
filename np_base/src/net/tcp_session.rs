use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use bytes::BytesMut;
use log::{debug, error};
use std::net::SocketAddr;
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadHalf, WriteHalf,
};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::yield_now;
use tokio::time::sleep;

/// run
///
/// [`session_id`] 会话id
///
/// [`addr`] 地址
///
/// [`delegate`] 会话代理
///
/// [`shutdown`] 监听退出消息
///
/// [`stream`] TcpStream
pub async fn run(
    session_id: u32,
    addr: SocketAddr,
    mut delegate: Box<dyn SessionDelegate>,
    mut shutdown: broadcast::Receiver<()>,
    stream: TcpStream,
) {
    let (reader, writer) = tokio::io::split(stream);
    let (delegate_sender, delegate_receiver) = unbounded_channel::<WriterMessage>();

    delegate
        .on_session_start(session_id, &addr, delegate_sender)
        .await;
    select! {
        _ = poll_read(addr, &mut delegate, reader) => {}
        _ = poll_write(delegate_receiver, writer) => {}
        _ = shutdown.recv() => {}
    }
    delegate.on_session_close().await;
}

/// 循环写入数据
async fn poll_write<S>(
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
                    error!("Error when write_all {:?}", error);
                    break;
                }

                if flush {
                    if let Err(error) = writer.flush().await {
                        error!("Error when flushing {:?}", error);
                    }
                }
            }
            WriterMessage::Flush => {
                if let Err(error) = writer.flush().await {
                    error!("Error when flushing {:?}", error);
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
) where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut buffer = BytesMut::with_capacity(1024);

    loop {
        match reader.read_buf(&mut buffer).await {
            // n为0表示对端已经关闭连接。
            Ok(n) if n == 0 => {
                // 客户端主动断开
                debug!("Socket[{}] closed.", addr);
                return;
            }
            // 正常收到数据
            Ok(_n) => {
                // 循环解包
                loop {
                    // 处理数据粘包
                    match delegate.on_try_extract_frame(&mut buffer) {
                        Ok(result) => {
                            if let Some(frame) = result {
                                // 收到完整消息
                                if !delegate.on_recv_frame(frame).await {
                                    // 消息处理失败
                                    error!("Socket [{}] message processing failed", addr);
                                    return;
                                }
                            } else {
                                // 消息包接收还未完成
                                break;
                            }
                        }
                        Err(error) => {
                            // 消息解包错误
                            error!("Socket [{}] Try extract frame error {}", addr, error);
                            return;
                        }
                    }
                }
            }
            Err(err) => {
                // socket读错误
                error!("Failed to read from socket[{}]: {}", addr, err);
                return;
            }
        }

        if buffer.capacity() > 1024 * 1024 * 5 {
            error!(
                "The buffer size is abnormal ({}), whether the buffer data has not been consumed",
                buffer.capacity()
            );
        }
    }
}
