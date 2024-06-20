use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use crate::net::{tcp_server, udp_server};
use crate::proxy::OutputFuncType;
use crate::proxy::SenderMap;
use anyhow::anyhow;
use async_trait::async_trait;
use bytes::BytesMut;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Sender, UnboundedSender};
use tokio::sync::{mpsc, Mutex, Notify};

pub enum InletProxyType {
    TCP,
    UDP,
}

pub struct Inlet {
    inlet_proxy_type: InletProxyType,
    shutdown_tx: Option<Sender<()>>,
    notify: Arc<Notify>,
    sender_map: SenderMap,
}

impl Inlet {
    pub fn new(inlet_proxy_type: InletProxyType) -> Self {
        Self {
            inlet_proxy_type,
            shutdown_tx: None,
            notify: Arc::new(Notify::new()),
            sender_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(
        &mut self,
        listen_addr: String,
        on_output_callback: OutputFuncType,
    ) -> anyhow::Result<()> {
        // 重复调用启动函数
        if self.shutdown_tx.is_some() {
            return Err(anyhow!("Repeated start"));
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let worker_notify = self.notify.clone();
        let sender_map = self.sender_map.clone();

        let create_session_delegate_func = Box::new(move || -> Box<dyn SessionDelegate> {
            Box::new(InletSession::new(
                sender_map.clone(),
                on_output_callback.clone(),
            ))
        });

        match self.inlet_proxy_type {
            InletProxyType::TCP => {
                let listener = tcp_server::bind(&listen_addr).await?;
                self.shutdown_tx = Some(shutdown_tx);
                tokio::spawn(async move {
                    tcp_server::run_server(
                        listener,
                        create_session_delegate_func,
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
                    udp_server::run_server(socket, create_session_delegate_func, async move {
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

    pub async fn send_to(&self, session_id: u32, message: WriterMessage) -> anyhow::Result<()> {
        if let Some(sender) = self.sender_map.lock().await.get(&session_id) {
            sender.send(message)?
        }
        Err(anyhow!("invalid session: {session_id}"))
    }
}

struct InletSession {
    sender_map: SenderMap,
    session_id: u32,
    on_output_callback: OutputFuncType,
}

impl InletSession {
    pub fn new(sender_map: SenderMap, on_output_callback: OutputFuncType) -> Self {
        Self {
            sender_map,
            session_id: 0,
            on_output_callback,
        }
    }
}

#[async_trait]
impl SessionDelegate for InletSession {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        _addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) {
        self.session_id = session_id;
        self.sender_map.lock().await.insert(session_id, tx);
    }

    async fn on_session_close(&mut self) {
        self.sender_map.lock().await.remove(&self.session_id);
    }

    fn on_try_extract_frame(&self, buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
        // 此处使用 buffer.split().to_vec(); 而不是 buffer.to_vec();
        // 因为split().to_vec()更高效，少了一次内存分配和拷贝
        // 并且在 on_try_extract_frame 函数中只能使用消耗 buffer 数据的函数，否则框架会一直循环调用 on_try_extract_frame 来驱动处理消息
        let frame = buffer.split().to_vec();
        Ok(Some(frame))
    }

    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> bool {
        let _ = (self.on_output_callback)(WriterMessage::Send(frame, true)).await;
        true
    }
}
