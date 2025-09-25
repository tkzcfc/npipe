use crate::net::session_delegate::SessionDelegate;
use crate::net::WriterMessage;
use crate::net::{tcp_server, udp_server};
use crate::proxy::common::SessionCommonInfo;
use crate::proxy::http::HttpContext;
use crate::proxy::proxy_context::{ProxyContext, ProxyContextData, UniversalProxy};
use crate::proxy::socks5::Socks5Context;
use crate::proxy::{common, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use async_trait::async_trait;
use log::{error, trace};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, RwLock};
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub enum InletProxyType {
    TCP,
    UDP,
    SOCKS5,
    HTTP,
    UNKNOWN,
}

impl InletProxyType {
    pub fn from_u32(value: u32) -> InletProxyType {
        match value {
            0 => InletProxyType::TCP,
            1 => InletProxyType::UDP,
            2 => InletProxyType::SOCKS5,
            3 => InletProxyType::HTTP,
            _ => InletProxyType::UNKNOWN,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            InletProxyType::TCP => 0,
            InletProxyType::UDP => 1,
            InletProxyType::SOCKS5 => 2,
            InletProxyType::HTTP => 3,
            InletProxyType::UNKNOWN => 255,
        }
    }

    pub fn is_socks5(&self) -> bool {
        matches!(self, InletProxyType::SOCKS5)
    }

    pub fn is_tcp(&self) -> bool {
        matches!(self, InletProxyType::TCP)
    }
}

struct SessionInfo {
    proxy_message_tx: UnboundedSender<ProxyMessage>,
    common_info: SessionCommonInfo,
}

type SessionInfoMap = Arc<RwLock<HashMap<u32, SessionInfo>>>;

pub struct Inlet {
    is_running: Arc<AtomicBool>,
    input: Option<UnboundedSender<ProxyMessage>>,
    session_info_map: SessionInfoMap,
    description: String,
    on_output_callback: OutputFuncType,
}

pub struct InletDataEx {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl InletDataEx {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

impl Inlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            session_info_map: Arc::new(RwLock::new(HashMap::new())),
            input: None,
            description,
            on_output_callback,
        }
    }

    pub async fn start(
        &mut self,
        inlet_proxy_type: InletProxyType,
        listen_addr: String,
        output_addr: String,
        is_compressed: bool,
        encryption_method: String,
        data_ex: InletDataEx,
    ) -> anyhow::Result<()> {
        // 重复调用启动函数
        if self.running() {
            return Err(anyhow!("Repeated start"));
        }

        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (output_tx, output_rx) = mpsc::channel::<ProxyMessage>(1000);

        self.input = Some(input_tx);

        let session_info_map = self.session_info_map.clone();
        let inlet_proxy_type_cloned = inlet_proxy_type.clone();
        let data_ex = Arc::new(data_ex);

        let create_session_delegate_func = Box::new(move || -> Box<dyn SessionDelegate> {
            Box::new(InletSession::new(
                inlet_proxy_type.clone(),
                output_addr.clone(),
                session_info_map.clone(),
                is_compressed,
                encryption_method.clone(),
                output_tx.clone(),
                data_ex.clone(),
            ))
        });

        let on_output_callback = self.on_output_callback.clone();
        let session_info_map = self.session_info_map.clone();
        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::Relaxed);

        match inlet_proxy_type_cloned {
            InletProxyType::TCP | InletProxyType::SOCKS5 | InletProxyType::HTTP => {
                let listener = TcpListener::bind(&listen_addr).await?;

                tokio::spawn(async move {
                    let server_task = tcp_server::Builder::new(create_session_delegate_func)
                        .set_on_steam_init_callback(Arc::new(|stream: TcpStream| {
                            Box::pin(async move {
                                stream.set_nodelay(true)?;
                                Ok(stream)
                            })
                        }))
                        .build_with_listener(
                            listener,
                            Self::async_receive_input(input_rx, session_info_map),
                        );

                    select! {
                        _= server_task => {},
                        _= common::async_receive_output(output_rx, on_output_callback) => {}
                    }

                    is_running.store(false, Ordering::Relaxed);
                });
            }
            InletProxyType::UDP => {
                let socket = UdpSocket::bind(&listen_addr).await?;

                tokio::spawn(async move {
                    let server_task = udp_server::run_server(
                        socket,
                        create_session_delegate_func,
                        Self::async_receive_input(input_rx, session_info_map),
                    );

                    select! {
                        _= server_task => {},
                        _= common::async_receive_output(output_rx, on_output_callback) => {}
                    }

                    is_running.store(false, Ordering::Relaxed);
                });
            }
            InletProxyType::UNKNOWN => {
                return Err(anyhow!("Unknown inlet proxy type"));
            }
        };

        Ok(())
    }

    pub async fn input(&self, proxy_message: ProxyMessage) {
        if let Some(sender) = &self.input {
            let _ = sender.send(proxy_message);
        }
    }

    pub fn running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub async fn stop(&mut self) {
        self.input.take();
        while self.running() {
            yield_now().await;
        }
        self.session_info_map.write().await.clear();
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    async fn async_receive_input(
        mut input: UnboundedReceiver<ProxyMessage>,
        session_info_map: SessionInfoMap,
    ) {
        while let Some(message) = input.recv().await {
            if let Err(err) = Self::input_internal(message, &session_info_map).await {
                error!("inlet async_receive_input error: {}", err.to_string());
            }
        }
    }

    async fn input_internal(
        message: ProxyMessage,
        session_info_map: &SessionInfoMap,
    ) -> anyhow::Result<()> {
        match &message {
            ProxyMessage::O2iConnect(session_id, success, error_msg) => {
                trace!(
                    "O2iConnect: session_id:{session_id}, success:{success}, error_msg:{error_msg}"
                );

                if let Some(session) = session_info_map.read().await.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iDisconnect(session_id) => {
                trace!("O2iDisconnect: session_id:{session_id}");
                if let Some(session) = session_info_map.read().await.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iSendDataResult(session_id, data_len) => {
                // trace!("O2iSendDataResult: session_id:{session_id}, data_len:{data_len}");
                if let Some(session) = session_info_map.read().await.get(session_id) {
                    session
                        .common_info
                        .flow_controller
                        .release_read_permit(*data_len);
                }
            }
            ProxyMessage::O2iRecvDataFrom(session_id, _data, _remote_addr) => {
                if let Some(session) = session_info_map.read().await.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iRecvData(session_id, _data) => {
                // trace!("O2iRecvData: session_id:{session_id}");
                if let Some(session) = session_info_map.read().await.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            _ => {
                return Err(anyhow!("Unknown message"));
            }
        }

        Ok(())
    }
}

struct InletSession {
    session_info_map: SessionInfoMap,
    proxy_ctx: Arc<RwLock<dyn ProxyContext + Send + Sync>>,
    proxy_ctx_data: Arc<ProxyContextData>,
    proxy_message_recv_task_cancel_token: Option<CancellationToken>,
}

impl InletSession {
    pub fn new(
        inlet_proxy_type: InletProxyType,
        output_addr: String,
        session_info_map: SessionInfoMap,
        is_compressed: bool,
        encryption_method: String,
        output: Sender<ProxyMessage>,
        data_ex: Arc<InletDataEx>,
    ) -> Self {
        let proxy_ctx: Arc<RwLock<dyn ProxyContext + Send + Sync>> = match inlet_proxy_type {
            InletProxyType::SOCKS5 => Arc::new(RwLock::new(Socks5Context::new())),
            InletProxyType::HTTP => Arc::new(RwLock::new(HttpContext::new())),
            _ => Arc::new(RwLock::new(UniversalProxy::new())),
        };

        Self {
            session_info_map,
            proxy_ctx,
            proxy_ctx_data: Arc::new(ProxyContextData::new(
                inlet_proxy_type,
                output_addr,
                output,
                SessionCommonInfo::from_method_name(true, is_compressed, encryption_method),
                data_ex,
            )),
            proxy_message_recv_task_cancel_token: None,
        }
    }
}

#[async_trait]
impl SessionDelegate for InletSession {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        addr: &SocketAddr,
        write_msg_tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        trace!("inlet on session({session_id}) start {addr}");

        self.proxy_ctx_data.set_session_id(session_id);

        let (proxy_msg_tx, mut proxy_msg_rx) = mpsc::unbounded_channel::<ProxyMessage>();

        let token = CancellationToken::new();
        let cloned_token = token.clone();

        let context_cloned = self.proxy_ctx.clone();

        tokio::spawn(async move {
            loop {
                // 检查是否取消
                select! {
                    _ = cloned_token.cancelled() => {
                        // println!("Task cancelled, cleaning up...");
                        break;
                    }
                    message = proxy_msg_rx.recv() => {
                        if let Some(message) = message {
                            // 处理消息...
                            if let Err(err) = context_cloned
                            .write()
                            .await
                            .on_recv_proxy_message(message)
                            .await
                            {
                                error!("on_recv_proxy_message: {err}")
                            }
                        } else {
                            break; // 通道关闭
                        }
                    }
                }
            }
        });

        self.proxy_message_recv_task_cancel_token = Some(token);

        self.session_info_map.write().await.insert(
            session_id,
            SessionInfo {
                proxy_message_tx: proxy_msg_tx,
                common_info: self.proxy_ctx_data.common_data.clone(),
            },
        );

        self.proxy_ctx
            .write()
            .await
            .on_start(self.proxy_ctx_data.clone(), *addr, write_msg_tx)
            .await?;

        Ok(())
    }

    async fn on_session_close(&mut self) -> anyhow::Result<()> {
        let session_id = self.proxy_ctx_data.get_session_id();
        trace!("inlet on session({}) close", session_id);

        if let Some(token) = self.proxy_message_recv_task_cancel_token.take() {
            token.cancel();
        }

        self.session_info_map.write().await.remove(&session_id);
        self.proxy_ctx
            .write()
            .await
            .on_stop(self.proxy_ctx_data.clone())
            .await?;
        Ok(())
    }

    async fn on_recv_frame(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
        self.proxy_ctx
            .write()
            .await
            .on_recv_peer_data(self.proxy_ctx_data.clone(), frame)
            .await
    }

    async fn is_ready_for_read(&self) -> bool {
        self.proxy_ctx.read().await.is_ready_for_read()
    }
}
