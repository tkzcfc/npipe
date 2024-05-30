use crate::net::session::WriterMessage;
use crate::net::session_logic::SessionLogic;
use crate::net::tcp_server;
use async_trait::async_trait;
use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedSender};
use tokio::sync::{mpsc, Notify};

pub enum InletProxyType {
    TCP,
    UDP,
}

pub struct Inlet {
    proxy: InletProxyType,
    shutdown_tx: Option<Sender<()>>,
    notify: Arc<Notify>,
}

impl Inlet {
    pub fn new(proxy: InletProxyType) -> Self {
        Self {
            proxy,
            shutdown_tx: None,
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn start(&mut self, listen_addr: String) -> anyhow::Result<()> {
        self.stop().await;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        self.shutdown_tx = Option::Some(shutdown_tx);

        let addr = listen_addr.parse::<SocketAddr>()?;
        let worker_notify = self.notify.clone();

        match self.proxy {
            InletProxyType::TCP => {
                let listener = TcpListener::bind(addr).await?;
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
                let udpsocket = UdpSocket::bind(addr).await?;
                tokio::spawn(async move {
                    // 循环读取中...
                    let mut buf = [0; 1024];
                    let recv_task = async { loop {
                        // 接收数据
                        if let Ok((len, addr)) = udpsocket.recv_from(&mut buf).await {
                            println!("Received {} bytes from {}", len, addr);
                        }
                    }};

                    select! {
                        _= recv_task => {},
                        _= shutdown_rx.recv() => {}
                    };
                    worker_notify.notify_one();
                });
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        if self.shutdown_tx.is_some() {
            self.shutdown_tx.take();
            self.notify.notified().await;
        }
    }
}

struct InletSession {}

impl InletSession {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SessionLogic for InletSession {
    fn on_session_start(&mut self, _session_id: u32, _tx: UnboundedSender<WriterMessage>) {}

    async fn on_session_close(&mut self) {}

    fn on_try_extract_frame(&self, buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Some(buffer.to_vec()))
    }

    async fn on_recv_frame(&mut self, _frame: Vec<u8>) -> bool {
        true
    }
}
