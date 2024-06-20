use crate::net::session_delegate::SessionDelegate;
use crate::net::tcp_session::WriterMessage;
use crate::net::{tcp_server, udp_server};
use anyhow::anyhow;
use async_trait::async_trait;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Sender, UnboundedSender};
use tokio::sync::{mpsc, Notify};

pub enum InletProxyType {
    TCP,
    UDP,
}

pub struct Inlet {
    inlet_proxy_type: InletProxyType,
    shutdown_tx: Option<Sender<()>>,
    notify: Arc<Notify>,
}

impl Inlet {
    pub fn new(inlet_proxy_type: InletProxyType) -> Self {
        Self {
            inlet_proxy_type,
            shutdown_tx: None,
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn start(&mut self, listen_addr: String) -> anyhow::Result<()> {
        // 重复调用启动函数
        if self.shutdown_tx.is_some() {
            return Err(anyhow!("Repeated start"));
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let worker_notify = self.notify.clone();

        match self.inlet_proxy_type {
            InletProxyType::TCP => {
                let listener = tcp_server::bind(&listen_addr).await?;
                self.shutdown_tx = Some(shutdown_tx);
                tokio::spawn(async move {
                    tcp_server::run_server(
                        listener,
                        || Box::new(InletSession::new()),
                        |stream: TcpStream| async move { Ok(stream) },
                        async move {
                            let _ = shutdown_rx.recv().await;
                        },
                    )
                    .await;
                    worker_notify.notify_one();
                });
            }
            InletProxyType::UDP => {
                let socket = udp_server::bind(&listen_addr).await?;
                self.shutdown_tx = Some(shutdown_tx);
                tokio::spawn(async move {
                    udp_server::run_server(socket, || Box::new(InletSession::new()), async move {
                        let _ = shutdown_rx.recv().await;
                    })
                    .await;
                    worker_notify.notify_one();
                });
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        if self.shutdown_tx.is_some() {
            self.shutdown_tx.take();
            // 等待退出完毕
            self.notify.notified().await;
        }
    }

    pub fn running(&self) -> bool {
        self.shutdown_tx.is_some()
    }
}

struct InletSession {}

impl InletSession {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SessionDelegate for InletSession {
    fn on_session_start(&mut self, _session_id: u32, _tx: UnboundedSender<WriterMessage>) {}

    async fn on_session_close(&mut self) {}

    fn on_try_extract_frame(&self, buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Some(buffer.to_vec()))
    }

    async fn on_recv_frame(&mut self, _frame: Vec<u8>) -> bool {
        true
    }
}
