use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::{net_session, tls};
use log::{debug, error};
use log::{info, trace};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::ToSocketAddrs;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio_kcp::{KcpConfig, KcpListener};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

struct Server {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Server {
    async fn start_server(
        &self,
        mut listener: KcpListener,
        on_create_session_delegate_callback: CreateSessionDelegateCallback,
        tls_configuration: Option<tls::TlsConfiguration>,
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

        loop {
            let (stream, addr) = listener.accept().await?;

            let tls_acceptor = tls_acceptor.clone();
            let delegate = on_create_session_delegate_callback();
            let shutdown = self.notify_shutdown.subscribe();
            let shutdown_complete = self.shutdown_complete_tx.clone();

            // 新连接单独起一个异步任务处理
            tokio::spawn(async move {
                trace!("KCP Server new connection: {}", addr);

                if let Some(tls_acceptor) = tls_acceptor {
                    match tls::try_tls(stream, tls_acceptor).await {
                        Ok(stream) => {
                            net_session::run(
                                net_session::create_session_id(),
                                addr,
                                delegate,
                                shutdown,
                                stream,
                            )
                            .await;
                        }
                        Err(err) => {
                            debug!("KCP Server tls error: {err}");
                        }
                    }
                } else {
                    net_session::run(
                        net_session::create_session_id(),
                        addr,
                        delegate,
                        shutdown,
                        stream,
                    )
                    .await;
                }

                trace!("KCP Server disconnect: {}", addr);
                // 反向通知此会话结束
                drop(shutdown_complete);
            });
        }
    }
}

pub struct Builder {
    create_session_delegate_callback: CreateSessionDelegateCallback,
    kcp_config: KcpConfig,
    tls_configuration: Option<tls::TlsConfiguration>,
}

impl Builder {
    pub fn new(create_session_delegate_callback: CreateSessionDelegateCallback) -> Self {
        Self {
            create_session_delegate_callback,
            kcp_config: KcpConfig::default(),
            tls_configuration: None,
        }
    }

    pub fn set_kcp_config(mut self, config: KcpConfig) -> Self {
        self.kcp_config = config;
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
        listener: KcpListener,
        shutdown_condition: impl Future,
    ) -> anyhow::Result<()> {
        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

        let server = Server {
            notify_shutdown,
            shutdown_complete_tx,
        };

        select! {
            res = server.start_server(listener, self.create_session_delegate_callback, self.tls_configuration) => {
                if let Err(err) = res {
                    error!("KCP Server error: {}", err);
                }
            },
            _ = shutdown_condition => {
                info!("KCP Server shutting down");
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
        if let Err(_) = tokio::time::timeout(Duration::from_secs(600), wait_task).await {
            error!("KCP Server exit timeout, forced exit");
        }

        info!("KCP Server shutdown finish");

        Ok(())
    }

    pub async fn build<A: ToSocketAddrs>(
        self,
        addr: A,
        shutdown_condition: impl Future,
    ) -> anyhow::Result<()> {
        let listener = KcpListener::bind(self.kcp_config, &addr).await?;
        self.build_with_listener(listener, shutdown_condition).await
    }
}
