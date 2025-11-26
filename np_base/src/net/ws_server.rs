use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::ws_async_io::WebSocketAsyncIo;
use crate::net::{net_session, tls};
use log::{debug, error};
use log::{info, trace};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio_rustls::TlsAcceptor;

pub type StreamInitCallbackType = Arc<
    dyn Fn(TcpStream) -> Pin<Box<dyn Future<Output = anyhow::Result<TcpStream>> + Send>>
        + Send
        + Sync,
>;

struct Server {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Server {
    async fn start_server(
        &self,
        listener: TcpListener,
        on_create_session_delegate_callback: CreateSessionDelegateCallback,
        on_stream_init_callback: Option<StreamInitCallbackType>,
        tls_configuration: Option<tls::TlsConfiguration>,
    ) -> anyhow::Result<()> {
        let tls_acceptor = tls_configuration.map(TlsAcceptor::try_from).transpose()?;

        loop {
            let (mut stream, addr) = listener.accept().await?;

            if let Some(ref on_stream_init_callback) = on_stream_init_callback {
                match on_stream_init_callback(stream).await {
                    Ok(s) => {
                        stream = s;
                    }
                    Err(error) => {
                        error!("Websocket Server on_stream_init error:{}", error);
                        continue;
                    }
                }
            }

            let tls_acceptor = tls_acceptor.clone();
            let delegate = on_create_session_delegate_callback();
            let shutdown = self.notify_shutdown.subscribe();
            let shutdown_complete = self.shutdown_complete_tx.clone();

            // 新连接单独起一个异步任务处理
            tokio::spawn(async move {
                trace!("Websocket Server new connection: {}", addr);

                if let Some(tls_acceptor) = tls_acceptor {
                    match tls::try_tls(stream, tls_acceptor).await {
                        Ok(stream) => match tokio_tungstenite::accept_async(stream).await {
                            Ok(stream) => {
                                net_session::run(
                                    net_session::create_session_id(),
                                    addr,
                                    delegate,
                                    shutdown,
                                    WebSocketAsyncIo::new(stream),
                                )
                                .await;
                            }
                            Err(err) => {
                                error!("Websocket Server accept error: {err}");
                            }
                        },
                        Err(err) => {
                            debug!("Websocket Server tls error: {err}");
                        }
                    }
                } else {
                    match tokio_tungstenite::accept_async(stream).await {
                        Ok(stream) => {
                            net_session::run(
                                net_session::create_session_id(),
                                addr,
                                delegate,
                                shutdown,
                                WebSocketAsyncIo::new(stream),
                            )
                            .await;
                        }
                        Err(err) => {
                            error!("Websocket Server accept error: {err}");
                        }
                    }
                }

                trace!("Websocket Server disconnect: {}", addr);
                // 反向通知此会话结束
                drop(shutdown_complete);
            });
        }
    }
}

pub struct Builder {
    create_session_delegate_callback: CreateSessionDelegateCallback,
    tls_configuration: Option<tls::TlsConfiguration>,
    steam_init_callback: Option<StreamInitCallbackType>,
}

impl Builder {
    pub fn new(create_session_delegate_callback: CreateSessionDelegateCallback) -> Self {
        Self {
            create_session_delegate_callback,
            tls_configuration: None,
            steam_init_callback: None,
        }
    }

    pub fn set_on_steam_init_callback(
        mut self,
        steam_init_callback: StreamInitCallbackType,
    ) -> Self {
        self.steam_init_callback = Some(steam_init_callback);
        self
    }

    pub fn set_tls_configuration<A: ToString>(mut self, certificate: A, key: A) -> Self {
        self.tls_configuration = Some(tls::TlsConfiguration {
            certificate: certificate.to_string(),
            key: key.to_string(),
        });
        self
    }

    pub async fn build_with_listener(
        self,
        listener: TcpListener,
        shutdown_condition: impl Future,
    ) -> anyhow::Result<()> {
        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

        let server = Server {
            notify_shutdown,
            shutdown_complete_tx,
        };

        select! {
            res = server.start_server(listener, self.create_session_delegate_callback, self.steam_init_callback, self.tls_configuration) => {
                if let Err(err) = res {
                    error!("Websocket Server error: {}", err);
                }
            },
            _ = shutdown_condition => {
                info!("Websocket Server shutting down");
            }
        }

        // 解构server中的变量
        let Server {
            notify_shutdown,
            shutdown_complete_tx,
        } = server;

        // 销毁notify_shutdown 是为了触发 net_session run函数中shutdown.recv()返回
        drop(notify_shutdown);
        // 此处必须将 shutdown_complete_tx 并销毁，否则会一直卡在shutdown_complete_rx.recv().await
        drop(shutdown_complete_tx);

        // 等待服务器优雅退出任务
        let wait_task = async {
            let _ = shutdown_complete_rx.recv().await;
        };

        // 设置超时时间，无法优雅退出则强制退出
        if tokio::time::timeout(Duration::from_secs(60), wait_task)
            .await
            .is_err()
        {
            error!("Websocket Server exit timeout, forced exit");
        }

        info!("Websocket Server shutdown finish");

        Ok(())
    }

    pub async fn build<A: ToSocketAddrs>(
        self,
        addr: A,
        shutdown_condition: impl Future,
    ) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&addr).await?;
        self.build_with_listener(listener, shutdown_condition).await
    }
}
