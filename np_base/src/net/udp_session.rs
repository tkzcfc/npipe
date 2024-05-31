use crate::net::session_delegate::SessionDelegate;
use crate::net::tcp_session::WriterMessage;
use log::error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{broadcast, Mutex};
use tokio::task::yield_now;
use tokio::time::sleep;


async fn poll_read(delegate: &mut Box<dyn SessionDelegate>, mut udp_recv_receiver: UnboundedReceiver<(Vec<u8>)>) {
    while let Some(data) = udp_recv_receiver.recv().await {
        if !delegate.on_recv_frame(data).await {
            break;
        }
    }
}

async fn poll_write(
    mut delegate_receiver: UnboundedReceiver<WriterMessage>,
    socket: Arc<Mutex<UdpSocket>>,
    addr: SocketAddr,
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

                if let Err(error) = socket.lock().await.send_to(&data, &addr).await {
                    error!("Error when udp socket send_to {:?}", error);
                    break;
                }
            }
            WriterMessage::Flush => {}
        }

        yield_now().await;
    }

    delegate_receiver.close();
}

/// run
///
/// [`logic_receiver`] 接收logic发送的各种(写/关闭等)指令
///
/// [`udp_recv_receiver`] 接收udp收到的数据
///
/// [`shutdown`] 监听退出消息
///
/// [`socket`] 写操作对象
pub async fn run(
    session_id: u32,
    addr: SocketAddr,
    mut delegate: Box<dyn SessionDelegate>,
    udp_recv_receiver: UnboundedReceiver<(Vec<u8>)>,
    mut shutdown: broadcast::Receiver<()>,
    socket: Arc<Mutex<UdpSocket>>,
) {
    let (delegate_sender, delegate_receiver) = unbounded_channel::<WriterMessage>();
    delegate.on_session_start(session_id, delegate_sender);
    select! {
            _= poll_read(&mut delegate, udp_recv_receiver) => {},
            _= poll_write(delegate_receiver, socket, addr) => {},
            _ = shutdown.recv() => {}
        }

    delegate.on_session_close().await;
}