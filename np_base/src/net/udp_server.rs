use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::{net_session, udp_session};
use bytes::Bytes;
use dashmap::DashMap;
use log::{error, info, trace, warn};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{broadcast, mpsc};

/// UDP 服务器
///
/// 优化说明:
/// - 用 `DashMap` 替代 `Mutex<HashMap>`，消除全局锁竞争
/// - 用 `Bytes` 替代 `Vec<u8>`，避免数据拷贝
pub async fn run_server(
    socket: UdpSocket,
    on_create_session_delegate_callback: CreateSessionDelegateCallback,
    shutdown: impl Future,
) {
    let (notify_shutdown, receiver_shutdown) = broadcast::channel::<()>(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);

    let recv_task = async {
        let session_map: Arc<DashMap<SocketAddr, mpsc::Sender<Bytes>>> = Arc::new(DashMap::new());
        let mut buf = [0u8; 65535];
        let socket = Arc::new(socket);
        /// 连续错误超过此值视为 socket 不可恢复，退出接收循环
        const MAX_CONSECUTIVE_ERRORS: u32 = 10;
        let mut consecutive_errors: u32 = 0;

        loop {
            let (amt, addr) = match socket.recv_from(&mut buf).await {
                Ok(v) => {
                    consecutive_errors = 0; // 成功后重置错误计数
                    v
                }
                Err(e) => {
                    consecutive_errors += 1;

                    // Windows: WSAECONNRESET (10054) —— 对端 ICMP "port unreachable"
                    // 反射回本 socket，属于可恢复错误，直接 continue。
                    // 但若连续错误过多，说明 socket 可能已损坏，退出避免 CPU 空转。
                    #[cfg(windows)]
                    if e.raw_os_error() == Some(10054) && consecutive_errors <= 3 {
                        continue;
                    }

                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        error!("UDP recv_from too many consecutive errors ({consecutive_errors}), last: {e}, exiting recv loop");
                        break;
                    }

                    warn!(
                        "UDP recv_from error ({consecutive_errors}/{MAX_CONSECUTIVE_ERRORS}): {e}"
                    );
                    // 指数退避，避免 CPU 空转：1ms → 2ms → 4ms … 上限 32ms
                    let backoff = Duration::from_millis(1u64 << (consecutive_errors - 1).min(5));
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            let received_data = Bytes::copy_from_slice(&buf[..amt]);

            let sender = session_map.entry(addr).or_insert_with(|| {
                // 全局唯一 session_id，与所有协议的会话共用同一空间
                let session_id = net_session::create_session_id();
                // 每个会话独立的 delegate 实例，避免跨会话状态干扰
                let delegate = on_create_session_delegate_callback();
                // 通知会话结束
                let shutdown = receiver_shutdown.resubscribe();
                // 反向通知会话结束完毕
                let shutdown_complete = shutdown_complete_tx.clone();
                // UDP 会话直接使用共享 socket，避免每个会话创建独立 socket 的资源开销和端口占用问题
                let socket_cloned = socket.clone();

                // 创建有界通道，限制积压消息数量
                let (udp_recv_sender, udp_recv_receiver) = mpsc::channel::<Bytes>(128);

                // clone session_map 供会话任务使用
                let session_map_cloned = session_map.clone();
                // 新连接单独起一个异步任务处理
                tokio::spawn(async move {
                    trace!("UDP Server new connection: {addr}, session_id: {session_id}");
                    udp_session::run(
                        session_id,
                        addr,
                        delegate,
                        Some(udp_recv_receiver),
                        shutdown,
                        socket_cloned,
                    )
                    .await;
                    session_map_cloned.remove(&addr);
                    trace!("UDP Server disconnect: {addr}");
                    // 反向通知会话结束
                    drop(shutdown_complete);
                });

                udp_recv_sender
            });

            match sender.try_send(received_data) {
                Ok(_) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // 通道已满：符合 UDP 丢包特性，warn 级别记录，不阻塞接收循环
                    warn!("UDP backpressure drop (channel full), addr: {addr}");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // 会话正在关闭，session_map 尚未清理，忽略即可
                    trace!("UDP send to closed session, addr: {addr}");
                }
            }
        }
    };

    select! {
        _ = recv_task => {},
        _ = shutdown => { info!("UDP Server shutting down"); }
    }

    drop(notify_shutdown);
    drop(shutdown_complete_tx);

    let wait_task = async {
        let _ = shutdown_complete_rx.recv().await;
    };

    if tokio::time::timeout(Duration::from_secs(60), wait_task)
        .await
        .is_err()
    {
        error!("UDP Server exit timeout, forced exit");
    }

    info!("UDP Server shutdown finish");
}
