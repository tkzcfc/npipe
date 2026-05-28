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
use bytes::Bytes;
use dashmap::DashMap;
use log::{error, trace};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, Mutex, Notify};
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

/// 使用 DashMap 替代 Arc<RwLock<HashMap>>:
/// - DashMap 使用分片锁 (shard-based locking)，并发读写不互斥
/// - 每次消息路由只需获取对应 shard 的锁，而非全局锁
/// - 读操作不阻塞其他 shard 的读写
type SessionInfoMap = Arc<DashMap<u32, SessionInfo>>;

pub struct Inlet {
    is_running: Arc<AtomicBool>,
    input: Option<UnboundedSender<ProxyMessage>>,
    session_info_map: SessionInfoMap,
    description: String,
    on_output_callback: OutputFuncType,
    /// stop() 等待服务停止时使用的通知, 替代 yield_now() spin loop
    stopped_notify: Arc<Notify>,
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
            session_info_map: Arc::new(DashMap::new()),
            input: None,
            description,
            on_output_callback,
            stopped_notify: Arc::new(Notify::new()),
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
        let stopped_notify = self.stopped_notify.clone();
        is_running.store(true, Ordering::Relaxed);

        match inlet_proxy_type_cloned {
            InletProxyType::TCP | InletProxyType::SOCKS5 | InletProxyType::HTTP => {
                let listener = TcpListener::bind(&listen_addr).await?;

                tokio::spawn(async move {
                    let server_task = tcp_server::Builder::new(create_session_delegate_func)
                        .set_on_stream_init_callback(Arc::new(|stream: TcpStream| {
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
                    stopped_notify.notify_waiters();
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
                    stopped_notify.notify_waiters();
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
        // 必须先订阅再检查 running()，否则存在竞态：
        // 若先 running() → true，再订阅 notified()，而任务恰好在两者之间执行了
        // is_running.store(false) + notify_waiters()，则通知对尚未订阅的 Notified 无效，
        // stop() 会等满 30 秒超时才返回。
        let notified = self.stopped_notify.notified();
        if self.running() {
            tokio::time::timeout(std::time::Duration::from_secs(30), notified)
                .await
                .ok();
        }
        self.session_info_map.clear();
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    async fn async_receive_input(
        mut input: UnboundedReceiver<ProxyMessage>,
        session_info_map: SessionInfoMap,
    ) {
        while let Some(message) = input.recv().await {
            if let Err(err) = Self::input_internal(message, &session_info_map) {
                error!("inlet async_receive_input error: {}", err);
            }
        }
    }

    fn input_internal(
        message: ProxyMessage,
        session_info_map: &SessionInfoMap,
    ) -> anyhow::Result<()> {
        match &message {
            ProxyMessage::O2iConnect(session_id, success, error_msg) => {
                trace!(
                    "O2iConnect: session_id:{session_id}, success:{success}, error_msg:{error_msg}"
                );
                if let Some(session) = session_info_map.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iDisconnect(session_id) => {
                trace!("O2iDisconnect: session_id:{session_id}");
                if let Some(session) = session_info_map.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iSendDataResult(session_id, data_len) => {
                if let Some(session) = session_info_map.get(session_id) {
                    session
                        .common_info
                        .flow_controller
                        .release_read_permit(*data_len);
                }
            }
            ProxyMessage::O2iRecvDataFrom(session_id, _data, _remote_addr) => {
                if let Some(session) = session_info_map.get(session_id) {
                    session.proxy_message_tx.send(message)?;
                }
            }
            ProxyMessage::O2iRecvData(session_id, _data) => {
                if let Some(session) = session_info_map.get(session_id) {
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
    /// Mutex 替代 RwLock: 两个访问点（poll_read task 和 proxy message task）
    /// 都持有**写锁**，RwLock 的"读共享"优势在此完全无法体现，反而因读写锁
    /// 内部状态更复杂而增加开销。Mutex 在 write-only 场景下更高效。
    proxy_ctx: Arc<Mutex<dyn ProxyContext + Send + Sync>>,
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
        let proxy_ctx: Arc<Mutex<dyn ProxyContext + Send + Sync>> = match inlet_proxy_type {
            InletProxyType::SOCKS5 => Arc::new(Mutex::new(Socks5Context::new())),
            InletProxyType::HTTP => Arc::new(Mutex::new(HttpContext::new())),
            _ => Arc::new(Mutex::new(UniversalProxy::new())),
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
                select! {
                    _ = cloned_token.cancelled() => { break; }
                    message = proxy_msg_rx.recv() => {
                        match message {
                            Some(message) => {
                                if let Err(err) = context_cloned
                                    .lock()
                                    .await
                                    .on_recv_proxy_message(message)
                                    .await
                                {
                                    error!("on_recv_proxy_message: {err}")
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        self.proxy_message_recv_task_cancel_token = Some(token);

        // DashMap::insert 只锁对应 shard，不阻塞其他会话
        self.session_info_map.insert(
            session_id,
            SessionInfo {
                proxy_message_tx: proxy_msg_tx,
                common_info: self.proxy_ctx_data.common_data.clone(),
            },
        );

        self.proxy_ctx
            .lock()
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

        self.session_info_map.remove(&session_id);
        self.proxy_ctx
            .lock()
            .await
            .on_stop(self.proxy_ctx_data.clone())
            .await?;
        Ok(())
    }

    async fn on_recv_frame(&mut self, frame: Bytes) -> anyhow::Result<()> {
        self.proxy_ctx
            .lock()
            .await
            .on_recv_peer_data(self.proxy_ctx_data.clone(), frame)
            .await
    }

    async fn is_ready_for_read(&self) -> bool {
        self.proxy_ctx.lock().await.is_ready_for_read()
    }
}
