use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::udp_session;
use log::{error, info, trace};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{broadcast, mpsc, Mutex};

pub async fn run_server(
    socket: UdpSocket,
    on_create_session_delegate_callback: CreateSessionDelegateCallback,
    shutdown: impl Future,
) {
    let (notify_shutdown, receiver_shutdown) = broadcast::channel::<()>(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);

    // 循环读取中...
    let recv_task = async {
        let mut session_id_seed = 0;
        let hashmap: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let mut buf = [0; 65535]; // 最大允许的UDP数据包大小
        let socket = Arc::new(socket);

        loop {
            let result = socket.recv_from(&mut buf).await;
            if result.is_err() {
                continue;
            }
            let (amt, addr) = result.unwrap();

            let received_data = Vec::from(&buf[..amt]);

            let contains_addr = hashmap.lock().await.contains_key(&addr);
            if !contains_addr {
                // 新的会话id
                session_id_seed += 1;
                let session_id = session_id_seed;

                let delegate = on_create_session_delegate_callback();

                // 通知会话结束
                let shutdown = receiver_shutdown.resubscribe();
                // 反向通知会话结束完毕
                let shutdown_complete = shutdown_complete_tx.clone();
                // UdpSocket cloned
                let socket_cloned = socket.clone();

                // 创建有界通道，限制积压消息数量
                let (udp_recv_sender, udp_recv_receiver) = mpsc::channel::<Vec<u8>>(100);
                hashmap
                    .lock()
                    .await
                    .insert(addr.clone(), udp_recv_sender.clone());

                let hashmap_cloned = hashmap.clone();
                // 新连接单独起一个异步任务处理
                tokio::spawn(async move {
                    trace!("UDP Server new connection: {}", addr);
                    udp_session::run(
                        session_id,
                        addr,
                        delegate,
                        Some(udp_recv_receiver),
                        shutdown,
                        socket_cloned,
                    )
                    .await;
                    hashmap_cloned.lock().await.remove(&addr);
                    trace!("UDP Server disconnect: {}", addr);
                    // 反向通知会话结束
                    drop(shutdown_complete);
                });
            }

            if let Some(sender) = hashmap.lock().await.get(&addr) {
                // 如果通道已满，丢弃当前的消息
                if sender.capacity() > 0 {
                    if let Err(err) = sender.send(received_data).await {
                        error!(
                            "Unable to process received data, data address: {addr}, error: {err}"
                        );
                    }
                }
            }
        }
    };

    select! {
    _= recv_task => {},
    _= shutdown => { info!("UDP Server shutting down"); }
    };

    // 销毁notify_shutdown 是为了触发 udp_session即将停止服务，立即停止其他操作
    drop(notify_shutdown);
    // 此处必须将 shutdown_complete_tx 并销毁，否则会一直卡在shutdown_complete_rx.recv().await
    drop(shutdown_complete_tx);

    // 等待服务器优雅退出任务
    let wait_task = async {
        let _ = shutdown_complete_rx.recv().await;
    };

    // 设置超时时间，无法优雅退出则强制退出
    if let Err(_) = tokio::time::timeout(Duration::from_secs(600), wait_task).await {
        error!("UDP Server exit timeout, forced exit");
    }

    info!("UDP Server shutdown finish");
}
