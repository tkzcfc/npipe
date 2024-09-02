use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::tcp_session;
use log::{debug, error};
use log::{info, trace};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

pub type StreamInitCallbackType = Arc<
    dyn Fn(TcpStream) -> Pin<Box<dyn Future<Output = anyhow::Result<TcpStream>> + Send>>
        + Send
        + Sync,
>;

struct TlsConfiguration {
    certificate: String,
    key: String,
}

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
        tls_configuration: Option<TlsConfiguration>,
    ) -> anyhow::Result<()> {
        let tls_acceptor: Option<TlsAcceptor> = match tls_configuration {
            Some(tls_configuration) => {
                let certs = super::tls::load_certs(&tls_configuration.certificate)?;
                let keys = super::tls::load_private_key(&tls_configuration.key)?;

                let server_config = ServerConfig::builder()
                    .with_safe_defaults()
                    .with_no_client_auth()
                    .with_single_cert(certs, keys)?;

                Some(TlsAcceptor::from(Arc::new(server_config)))
            }
            None => None,
        };

        let mut session_id_seed = 0;
        loop {
            let (mut stream, addr) = listener.accept().await?;

            if let Some(ref on_stream_init_callback) = on_stream_init_callback {
                match on_stream_init_callback(stream).await {
                    Ok(s) => {
                        stream = s;
                    }
                    Err(error) => {
                        error!("TCP Server on_stream_init error:{}", error.to_string());
                        continue;
                    }
                }
            }

            session_id_seed += 1;

            let session_id = session_id_seed;
            let tls_acceptor = tls_acceptor.clone();
            let delegate = on_create_session_delegate_callback();
            let shutdown = self.notify_shutdown.subscribe();
            let shutdown_complete = self.shutdown_complete_tx.clone();

            // 新连接单独起一个异步任务处理
            tokio::spawn(async move {
                trace!("TCP Server new connection: {}", addr);

                if let Some(tls_acceptor) = tls_acceptor {
                    match Self::try_tls(stream, tls_acceptor).await {
                        Ok(stream) => {
                            tcp_session::run(session_id, addr, delegate, shutdown, stream).await;
                        }
                        Err(err) => {
                            println!("TCP Server tls error: {err}");
                            debug!("TCP Server tls error: {err}");
                        }
                    }
                } else {
                    tcp_session::run(session_id, addr, delegate, shutdown, stream).await;
                }

                trace!("TCP Server disconnect: {}", addr);
                // 反向通知此会话结束
                drop(shutdown_complete);
            });
        }
    }

    const TIMEOUT_TLS: u64 = 15;

    // ref https://github.com/netskillzgh/rollo/blob/master/rollo/src/server/world_socket_mgr.rs#L183
    async fn try_tls(
        socket: TcpStream,
        tls_acceptor: TlsAcceptor,
    ) -> anyhow::Result<tokio_rustls::TlsStream<TcpStream>> {
        let stream = timeout(
            Duration::from_secs(Self::TIMEOUT_TLS),
            tls_acceptor.accept(socket),
        )
        .await??;
        Ok(tokio_rustls::TlsStream::Server(stream))
    }
}

pub struct Builder {
    create_session_delegate_callback: CreateSessionDelegateCallback,
    tls_configuration: Option<TlsConfiguration>,
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
        self.tls_configuration = Some(TlsConfiguration {
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
                    error!("TCP Server error: {}", err);
                }
            },
            _ = shutdown_condition => {
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
