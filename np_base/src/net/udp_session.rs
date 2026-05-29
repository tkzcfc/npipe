use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use bytes::Bytes;
use log::error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;
use tokio::time::Duration;

/// 获取当前时间戳（秒）
#[inline(always)]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// UDP 活跃时间追踪器
///
/// 用 `AtomicU64` 替代 `Arc<RwLock<Instant>>`，消除 hot path 上的 async 锁开销。
/// UDP 每收到一个包都需要更新活跃时间戳，如果使用 async 锁会导致性能瓶颈。
#[derive(Clone)]
struct LastActiveTime(Arc<AtomicU64>);

impl LastActiveTime {
    fn new() -> Self {
        Self(Arc::new(AtomicU64::new(now_secs())))
    }

    #[inline(always)]
    fn touch(&self) {
        self.0.store(now_secs(), Ordering::Relaxed);
    }

    #[inline(always)]
    fn elapsed_secs(&self) -> u64 {
        now_secs().saturating_sub(self.0.load(Ordering::Relaxed))
    }
}

/// 背压等待：若 delegate 暂时无法处理数据，使用指数退避等待直到就绪。
async fn wait_until_ready(delegate: &dyn SessionDelegate) {
    if delegate.is_ready_for_read().await {
        return;
    }
    let mut backoff_ms = 1u64;
    loop {
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        if delegate.is_ready_for_read().await {
            break;
        }
        backoff_ms = (backoff_ms * 2).min(32);
    }
}

async fn poll_read_from_bounded_receiver(
    addr: SocketAddr,
    delegate: &mut Box<dyn SessionDelegate>,
    mut udp_recv_receiver: mpsc::Receiver<Bytes>,
    last_active: LastActiveTime,
) {
    wait_until_ready(delegate.as_ref()).await;

    while let Some(data) = udp_recv_receiver.recv().await {
        last_active.touch(); // O(1) 原子写，无锁
        if let Err(err) = delegate.on_recv_frame(data).await {
            error!("[{addr}] on_recv_frame error: {err}");
            break;
        }
    }
    udp_recv_receiver.close();
}

async fn poll_read(
    addr: SocketAddr,
    delegate: &mut Box<dyn SessionDelegate>,
    socket: Arc<UdpSocket>,
    last_active: LastActiveTime,
) {
    let mut buf = [0; 65535];
    loop {
        wait_until_ready(delegate.as_ref()).await;

        match socket.recv_from(&mut buf).await {
            Err(_) => continue,
            Ok((amt, peer_addr)) => {
                last_active.touch();
                // Bytes::copy_from_slice 只做一次内存拷贝（从内核缓冲区到用户空间已不可避免）
                let data = Bytes::copy_from_slice(&buf[..amt]);
                if let Err(err) = delegate.on_recv_frame_from(data, peer_addr).await {
                    error!("[{addr}] on_recv_frame error: {err}");
                    break;
                }
            }
        }
    }
}

async fn poll_write(
    addr: SocketAddr,
    mut delegate_receiver: UnboundedReceiver<WriterMessage>,
    socket: Arc<UdpSocket>,
    last_active: LastActiveTime,
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
                    continue;
                }
                last_active.touch();
                if let Err(error) = socket.send_to(&data, &addr).await {
                    error!("[{addr}] Error when udp socket send_to {:?}", error);
                    break;
                }
            }
            WriterMessage::SendTo(data, target_addr) => {
                if data.is_empty() {
                    continue;
                }
                last_active.touch();
                if let Err(error) = socket.send_to(&data, &target_addr).await {
                    error!("[{addr}] Error when udp socket send_to {:?}", error);
                    break;
                }
            }
            WriterMessage::SendAndThen(data, callback) => {
                if !data.is_empty() {
                    last_active.touch();
                    if let Err(error) = socket.send_to(&data, &addr).await {
                        error!("[{addr}] Error when udp socket send_to {:?}", error);
                        break;
                    }
                }
                callback().await;
            }
            WriterMessage::Flush => {}
        }
    }

    delegate_receiver.close();
}

/// 超时检测：每秒检查一次，超过 10s 无活动则返回
async fn poll_timeout(last_active: LastActiveTime) {
    const TIMEOUT_SECS: u64 = 10;
    loop {
        sleep(Duration::from_secs(1)).await;
        if last_active.elapsed_secs() > TIMEOUT_SECS {
            break;
        }
    }
}

/// run UDP session
pub async fn run(
    session_id: u32,
    addr: SocketAddr,
    mut delegate: Box<dyn SessionDelegate>,
    udp_recv_receiver: Option<mpsc::Receiver<Bytes>>,
    mut shutdown_receiver: broadcast::Receiver<()>,
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

    let last_active = LastActiveTime::new();

    if let Some(udp_recv_receiver) = udp_recv_receiver {
        select! {
            _ = poll_read_from_bounded_receiver(addr, &mut delegate, udp_recv_receiver, last_active.clone()) => {},
            _ = poll_write(addr, delegate_receiver, socket, last_active.clone()) => {},
            _ = poll_timeout(last_active) => {},
            _ = shutdown_receiver.recv() => {}
        }
    } else {
        select! {
            _ = poll_read(addr, &mut delegate, socket.clone(), last_active.clone()) => {},
            _ = poll_write(addr, delegate_receiver, socket, last_active.clone()) => {},
            _ = poll_timeout(last_active) => {},
            _ = shutdown_receiver.recv() => {}
        }
    }

    if let Err(err) = delegate.on_session_close().await {
        error!("[{addr}] on_session_close error:{err}");
    }
}
