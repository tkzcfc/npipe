use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use log::error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{broadcast, Mutex};
use tokio::task::yield_now;
use tokio::time::sleep;

async fn poll_read(
    addr: SocketAddr,
    delegate: &mut Box<dyn SessionDelegate>,
    mut udp_recv_receiver: UnboundedReceiver<Vec<u8>>,
) {
    while let Some(data) = udp_recv_receiver.recv().await {
        if let Err(err) = delegate.on_recv_frame(data).await {
            error!("[{addr}] on_recv_frame error: {err}");
            break;
        }
    }
}

async fn poll_write(
    addr: SocketAddr,
    mut delegate_receiver: UnboundedReceiver<WriterMessage>,
    socket: Arc<UdpSocket>,
) {
    while let Some(message) = delegate_receiver.recv().await {
        match message {
            WriterMessage::Close => break,
            WriterMessage::CloseDelayed(duration) => {
                sleep(duration).await;
                break;
            }
            WriterMessage::Send(data, _flush) => {
                if data.is_empty() {
                    yield_now().await;
                    continue;
                }

                if let Err(error) = socket.send_to(&data, &addr).await {
                    error!("[{addr}] Error when udp socket send_to {:?}", error);
                    break;
                }
            }
            WriterMessage::Flush => {}
        }
    }

    delegate_receiver.close();
}

/// run
///
/// [`session_id`] 会话id
///
/// [`addr`] UDP发送端地址
///
/// [`delegate`] 会话代理
///
/// [`udp_recv_receiver`] 无界通道接收端，接收udp收到的数据
///
/// [`shutdown`] 监听退出消息
///
/// [`socket`] UdpSocket对象，用于写入udp数据
pub async fn run(
    session_id: u32,
    addr: SocketAddr,
    mut delegate: Box<dyn SessionDelegate>,
    udp_recv_receiver: UnboundedReceiver<Vec<u8>>,
    mut shutdown: broadcast::Receiver<()>,
    socket: Arc<UdpSocket>,
) {
    let (delegate_sender, delegate_receiver) = unbounded_channel::<WriterMessage>();

    if let Err(err) = delegate
        .on_session_start(session_id, &addr, delegate_sender)
        .await
    {
        error!("on_session_start error:{err}");
        return;
    }

    select! {
        _= poll_read(addr, &mut delegate, udp_recv_receiver) => {},
        _= poll_write(addr, delegate_receiver, socket) => {},
        _ = shutdown.recv() => {}
    }

    if let Err(err) = delegate.on_session_close().await {
        error!("[{addr}] on_session_close error:{err}");
    }
}
