use crate::net::session_delegate::CreateSessionDelegateCallback;
use crate::net::{net_session, tls};
use log::error;
use log::{info, trace};
use s2n_quic::Server as QUICServer;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{broadcast, mpsc};

struct Server {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Server {
    async fn start_server(
        &self,
        io: &str,
        on_create_session_delegate_callback: CreateSessionDelegateCallback,
        tls_configuration: Option<tls::TlsConfiguration>,
    ) -> anyhow::Result<()> {
        let mut server = if let Some(tls_config) = &tls_configuration {
            QUICServer::builder()
                .with_tls((
                    Path::new(&tls_config.certificate),
                    Path::new(&tls_config.key),
                ))?
                .with_io(io)?
                .start()?
        } else {
            QUICServer::builder().with_io(io)?.start()?
        };

        let on_create_session_delegate_callback = Arc::new(on_create_session_delegate_callback);

        while let Some(mut connection) = server.accept().await {
            let remote_addr = connection.remote_addr()?;
            trace!("Accept connection from {}", remote_addr);

            let callback = on_create_session_delegate_callback.clone();
            let shutdown_complete_connection = self.shutdown_complete_tx.clone();
            let notify_shutdown_clone = self.notify_shutdown.clone();

            tokio::spawn(async move {
                while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
                    trace!("Stream opened from {}", remote_addr);

                    let delegate = callback();
                    let shutdown = notify_shutdown_clone.subscribe();
                    let shutdown_complete_stream = shutdown_complete_connection.clone();

                    // 新连接单独起一个异步任务处理
                    tokio::spawn(async move {
                        net_session::run(
                            net_session::create_session_id(),
                            remote_addr,
                            delegate,
                            shutdown,
                            stream,
                        )
                        .await;

                        trace!("Stream stopped from {}", remote_addr);
                        // 反向通知此会话结束
                        drop(shutdown_complete_stream);
                    });
                }
                drop(shutdown_complete_connection);
            });
        }
        Ok(())
    }
}

pub struct Builder {
    create_session_delegate_callback: CreateSessionDelegateCallback,
    tls_configuration: Option<tls::TlsConfiguration>,
}

impl Builder {
    pub fn new(create_session_delegate_callback: CreateSessionDelegateCallback) -> Self {
        Self {
            create_session_delegate_callback,
            tls_configuration: None,
        }
    }

    pub fn set_tls_configuration<A: ToString>(mut self, certificate: A, key: A) -> Self {
        self.tls_configuration = Some(tls::TlsConfiguration {
            certificate: certificate.to_string(),
            key: key.to_string(),
        });
        self
    }

    pub async fn build(self, addr: &str, shutdown_condition: impl Future) -> anyhow::Result<()> {
        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

        let server = Server {
            notify_shutdown,
            shutdown_complete_tx,
        };

        select! {
            res = server.start_server(addr, self.create_session_delegate_callback, self.tls_configuration) => {
                if let Err(err) = res {
                    error!("QUIC Server error: {}", err);
                }
            },
            _ = shutdown_condition => {
                info!("QUIC Server shutting down");
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
            error!("QUIC Server exit timeout, forced exit");
        }

        info!("QUIC Server shutdown finish");

        Ok(())
    }
}
