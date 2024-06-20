use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::tcp_session;
use log::error;
use log::{info, trace};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::{broadcast, mpsc};

pub type OnStreamInitReturnType = anyhow::Result<TcpStream>;
pub trait OnStreamInitCallback {
    fn call(
        &self,
        stream: TcpStream,
    ) -> Pin<Box<dyn Future<Output = OnStreamInitReturnType> + Send>>;
}

/// 实现 OnStreamInitCallback 为满足特定签名的闭包。
impl<F, Fut> OnStreamInitCallback for F
where
    F: Fn(TcpStream) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = OnStreamInitReturnType> + Send + 'static,
{
    fn call(
        &self,
        stream: TcpStream,
    ) -> Pin<Box<dyn Future<Output = OnStreamInitReturnType> + Send>> {
        Box::pin(self(stream))
    }
}

struct Server {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

pub async fn bind(addr: &str) -> io::Result<TcpListener> {
    let addr = addr.parse::<SocketAddr>();
    match addr {
        Ok(addr) => {
            info!("TCP Server start listening: {}", addr);
            TcpListener::bind(addr).await
        }
        Err(parse_error) => Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            parse_error.to_string(),
        )),
    }
}

/// Start TCP Server
pub async fn run_server(
    listener: TcpListener,
    on_create_session_delegate_callback: CreateSessionDelegateCallback,
    on_stream_init_callback: impl OnStreamInitCallback,
    shutdown: impl Future,
) {
    let (notify_shutdown, _) = broadcast::channel::<()>(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

    let server = Server {
        notify_shutdown,
        shutdown_complete_tx,
    };

    select! {
        res = server.run(listener, on_create_session_delegate_callback, on_stream_init_callback) => {
            if let Err(err) = res {
                error!("TCP Server failed to accept, {}", err);
            }
        },
        _ = shutdown => {
            info!("TCP Server shutting down");
        }
    }

    // 解构server中的变量
    let Server {
        notify_shutdown,
        shutdown_complete_tx,
    } = server;

    // 销毁notify_shutdown 是为了触发 tcp_session run函数中shutdown.recv()返回
    drop(notify_shutdown);
    // 此处必须将 shutdown_complete_tx 并销毁，否则会一直卡在shutdown_complete_rx.recv().await
    drop(shutdown_complete_tx);

    // 等待服务器优雅退出任务
    let wait_task = async {
        let _ = shutdown_complete_rx.recv().await;
    };

    // 设置超时时间，无法优雅退出则强制退出
    if let Err(_) = tokio::time::timeout(Duration::from_secs(600), wait_task).await {
        error!("TCP Server exit timeout, forced exit");
    }

    info!("TCP Server shutdown finish");
}

impl Server {
    async fn run(
        &self,
        listener: TcpListener,
        on_create_session_delegate_callback: CreateSessionDelegateCallback,
        on_stream_init_callback: impl OnStreamInitCallback,
    ) -> anyhow::Result<()> {
        let mut session_id_seed = 0;
        loop {
            let (stream, addr) = listener.accept().await?;

            match on_stream_init_callback.call(stream).await {
                Ok(stream) => {
                    session_id_seed += 1;
                    let session_id = session_id_seed;

                    let delegate = on_create_session_delegate_callback();
                    let shutdown = self.notify_shutdown.subscribe();
                    let shutdown_complete = self.shutdown_complete_tx.clone();

                    // 新连接单独起一个异步任务处理
                    tokio::spawn(async move {
                        trace!("TCP Server new connection: {}", addr);

                        tcp_session::run(session_id, addr, delegate, shutdown, stream).await;

                        trace!("TCP Server disconnect: {}", addr);
                        // 反向通知此会话结束
                        drop(shutdown_complete);
                    });
                }
                Err(error) => {
                    error!("TCP Server on_stream_init error:{}", error.to_string());
                }
            }
        }
    }
}
