use crate::net::session_delegate::SessionDelegate;
use crate::net::{net_session, udp_session, SendMessageFuncType, WriterMessage};
use crate::proxy::common::{InputSenderType, SessionCommonInfo};
use crate::proxy::crypto::get_method;
use crate::proxy::inlet::InletProxyType;
use crate::proxy::ProxyMessage;
use crate::proxy::{common, OutputFuncType};
use anyhow::anyhow;
use async_trait::async_trait;
use base64::prelude::*;
use bytes::Bytes;
use dashmap::DashMap;
use log::{debug, error, info, trace};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{broadcast, mpsc, Notify, RwLock};

struct SessionInfo {
    sender: InputSenderType,
    common_info: SessionCommonInfo,
}

type SessionInfoMap = Arc<DashMap<u32, SessionInfo>>;

pub struct Outlet {
    session_info_map: SessionInfoMap,
    description: String,
    notify_shutdown: RwLock<Option<broadcast::Sender<()>>>,
    receiver_shutdown: broadcast::Receiver<()>,
    output: mpsc::Sender<ProxyMessage>,
    input: UnboundedSender<ProxyMessage>,
    /// 当前活跃会话数，用于 stop() 等待所有会话关闭时避免 spin loop
    session_count: Arc<AtomicUsize>,
    /// 每当 session_count 降为 0 时通知 stop()
    all_sessions_closed: Arc<Notify>,
}

impl Outlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Arc<Self> {
        let (notify_shutdown, mut receiver_shutdown) = broadcast::channel::<()>(1);
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (output_tx, output_rx) = mpsc::channel::<ProxyMessage>(1000);

        let outlet = Arc::new(Self {
            session_info_map: Arc::new(DashMap::new()),
            description,
            notify_shutdown: RwLock::new(Some(notify_shutdown)),
            receiver_shutdown: receiver_shutdown.resubscribe(),
            output: output_tx,
            input: input_tx,
            session_count: Arc::new(AtomicUsize::new(0)),
            all_sessions_closed: Arc::new(Notify::new()),
        });

        let outlet_cloned = outlet.clone();

        // 通知会话结束
        tokio::spawn(async move {
            select! {
                _= common::async_receive_output(output_rx, on_output_callback) => {}
                _= receiver_shutdown.recv() => {}
                _= outlet.async_receive_input(input_rx) => {}
            }
            trace!("outlet async_receive_output finish");
        });

        outlet_cloned
    }

    pub async fn input(&self, proxy_message: ProxyMessage) {
        let _ = self.input.send(proxy_message);
    }

    pub async fn stop(&self) {
        let notify_shutdown = self.notify_shutdown.write().await.take();
        if let Some(notify_shutdown) = notify_shutdown {
            drop(notify_shutdown);

            // 先订阅再检查计数，避免竞态：
            // 若先 load() 得到 > 0，再调用 notified()，恰好在两者之间最后一个会话
            // 关闭并发出 notify_waiters()，通知会被丢失，导致等满 10s 超时。
            let wait = async {
                loop {
                    let notified = self.all_sessions_closed.notified(); // 先订阅
                    if self.session_count.load(Ordering::Acquire) == 0 {
                        break; // 已经全部关闭
                    }
                    notified.await; // 等通知，收到后重新检查
                }
            };

            if tokio::time::timeout(Duration::from_secs(10), wait)
                .await
                .is_err()
            {
                error!("Timeout waiting for client to stop");
            }
        }
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    async fn async_receive_input(&self, mut input: UnboundedReceiver<ProxyMessage>) {
        while let Some(message) = input.recv().await {
            if let Err(err) = self.input_internal(message).await {
                error!("outlet async_receive_input error: {}", err);
            }
        }
    }

    async fn input_internal(&self, message: ProxyMessage) -> anyhow::Result<()> {
        match message {
            ProxyMessage::I2oConnect(
                session_id,
                tunnel_type,
                is_tcp,
                is_compressed,
                addr,
                encryption_method,
                encryption_key,
                client_addr,
            ) => {
                trace!(
                    "I2oConnect: session_id:{session_id}, addr:{addr}, tunnel_type:{tunnel_type}"
                );

                let session_info_map = self.session_info_map.clone();
                let shutdown_receiver = self.receiver_shutdown.resubscribe();
                let output = self.output.clone();
                let session_count = self.session_count.clone();
                let all_sessions_closed = self.all_sessions_closed.clone();
                tokio::spawn(async move {
                    if let Err(err) = Self::on_i2o_connect(
                        session_info_map,
                        session_id,
                        InletProxyType::from_u32(tunnel_type as u32),
                        is_tcp,
                        is_compressed,
                        addr.clone(),
                        encryption_method,
                        encryption_key,
                        shutdown_receiver,
                        output.clone(),
                        session_count,
                        all_sessions_closed,
                    )
                    .await
                    {
                        error!(
                            "Failed to connect to {}, error: {}, remote client addr {}",
                            addr, err, client_addr
                        );

                        let err_info = if is_tcp {
                            format!("target=tcp://{}, reason={}", addr, err)
                        } else {
                            format!("target=udp://{}, reason={}", addr, err)
                        };

                        let _ = output
                            .send(ProxyMessage::O2iConnect(session_id, false, err_info))
                            .await;
                    } else {
                        info!(
                            "Successfully connected to {}, remote client addr {}",
                            addr, client_addr
                        );
                    }
                });
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                self.on_i2o_send_data(session_id, data).await?;
            }
            ProxyMessage::I2oSendToData(session_id, data, target_addr) => {
                self.on_i2o_send_to_data(session_id, data, target_addr)
                    .await?;
            }
            ProxyMessage::I2oDisconnect(session_id) => {
                trace!("I2oDisconnect: session_id:{session_id}");
                self.on_i2o_disconnect(session_id).await?;
            }
            ProxyMessage::I2oRecvDataResult(session_id, data_len) => {
                self.on_i2o_recv_data_result(session_id, data_len).await?;
            }
            _ => {
                return Err(anyhow!("Unknown message"));
            }
        }
        Ok(())
    }

    async fn on_i2o_send_data(&self, session_id: u32, data: Bytes) -> anyhow::Result<()> {
        // 从 DashMap 取出需要的字段后立即 drop ref，避免跨 await 持有 shard 锁
        let (decoded, data_len, sender) = {
            if let Some(session) = self.session_info_map.get(&session_id) {
                let data_len = data.len();
                let decoded = session.common_info.decode_data(data)?;
                let sender = session.sender.clone();
                (decoded, data_len, sender)
            } else {
                return Ok(());
            }
            // DashMap Ref 在这里自动 drop
        };

        let output = self.output.clone();
        let callback: SendMessageFuncType = Box::new(move || {
            let output = output.clone();
            Box::pin(async move {
                let _ = output
                    .send(ProxyMessage::O2iSendDataResult(session_id, data_len))
                    .await;
            })
        });

        sender.send(WriterMessage::SendAndThen(decoded, callback))?;
        Ok(())
    }

    async fn on_i2o_send_to_data(
        &self,
        session_id: u32,
        data: Bytes,
        target_addr: String,
    ) -> anyhow::Result<()> {
        let (decoded, data_len, sender) = {
            if let Some(session) = self.session_info_map.get(&session_id) {
                let data_len = data.len();
                let decoded = session.common_info.decode_data(data)?;
                let sender = session.sender.clone();
                (decoded, data_len, sender)
            } else {
                return Ok(());
            }
        };

        let target_addr = common::parse_addr(&target_addr).await?;
        sender.send(WriterMessage::SendTo(decoded, target_addr))?;

        let _ = self
            .output
            .send(ProxyMessage::O2iSendDataResult(session_id, data_len))
            .await;
        Ok(())
    }

    async fn on_i2o_disconnect(&self, session_id: u32) -> anyhow::Result<()> {
        info!("disconnect session: {session_id}");
        if let Some((_, client)) = self.session_info_map.remove(&session_id) {
            client.sender.send(WriterMessage::Close)?;
        }
        Ok(())
    }

    async fn on_i2o_recv_data_result(
        &self,
        session_id: u32,
        data_len: usize,
    ) -> anyhow::Result<()> {
        if let Some(client) = self.session_info_map.get(&session_id) {
            client
                .common_info
                .flow_controller
                .release_read_permit(data_len);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn on_i2o_connect(
        session_info_map: SessionInfoMap,
        session_id: u32,
        tunnel_type: InletProxyType,
        is_tcp: bool,
        is_compressed: bool,
        mut addr: String,
        encryption_method: String,
        encryption_key: String,
        shutdown_receiver: broadcast::Receiver<()>,
        output: mpsc::Sender<ProxyMessage>,
        session_count: Arc<AtomicUsize>,
        all_sessions_closed: Arc<Notify>,
    ) -> anyhow::Result<()> {
        if session_info_map.contains_key(&session_id) {
            return Err(anyhow!("repeated connection: session_id:{session_id}"));
        }

        let encryption_method = get_method(&encryption_method);
        let encryption_key = BASE64_STANDARD.decode(encryption_key.as_bytes())?;
        let common_info =
            SessionCommonInfo::new(false, is_compressed, encryption_method, encryption_key);

        let connect_with_tcp = match tunnel_type {
            InletProxyType::UDP => false,
            InletProxyType::SOCKS5 => {
                if !is_tcp {
                    addr = String::from("");
                }
                is_tcp
            }
            _ => true,
        };

        if connect_with_tcp {
            debug!("tcp_connect: {}", addr);
            let stream = TcpStream::connect(&addr).await?;

            let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
            let sf = SockRef::from(&stream);
            sf.set_tcp_keepalive(&ka)?;

            let addr = stream.peer_addr()?;

            tokio::spawn(async move {
                net_session::run(
                    session_id,
                    addr,
                    Box::new(OutletSession::new(
                        session_info_map,
                        common_info,
                        output,
                        tunnel_type,
                        session_count,
                        all_sessions_closed,
                    )),
                    shutdown_receiver,
                    stream,
                )
                .await;
                trace!("tcp client stop, peer addr: {}", addr);
            });
        } else {
            debug!("udp_connect: {}", addr);
            let any_addr = "0.0.0.0:0".parse::<SocketAddr>()?;
            let socket = Arc::new(UdpSocket::bind(any_addr).await?);

            let addr = if addr.is_empty() {
                any_addr
            } else {
                socket.connect(addr).await?;
                socket.peer_addr()?
            };

            tokio::spawn(async move {
                udp_session::run(
                    session_id,
                    addr,
                    Box::new(OutletSession::new(
                        session_info_map,
                        common_info,
                        output,
                        tunnel_type,
                        session_count,
                        all_sessions_closed,
                    )),
                    None,
                    shutdown_receiver,
                    socket.clone(),
                )
                .await;
                trace!("udp client stop, peer addr: {}", addr);
            });
        }

        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////////////////// OutletSession ////////////////////////////////////////////////////////////////////////////////////
struct OutletSession {
    session_info_map: SessionInfoMap,
    session_id: u32,
    common_data: SessionCommonInfo,
    output: mpsc::Sender<ProxyMessage>,
    tunnel_type: InletProxyType,
    session_count: Arc<AtomicUsize>,
    all_sessions_closed: Arc<Notify>,
}

impl OutletSession {
    fn new(
        session_info_map: SessionInfoMap,
        common_data: SessionCommonInfo,
        output: mpsc::Sender<ProxyMessage>,
        tunnel_type: InletProxyType,
        session_count: Arc<AtomicUsize>,
        all_sessions_closed: Arc<Notify>,
    ) -> Self {
        Self {
            session_info_map,
            session_id: 0,
            common_data,
            output,
            tunnel_type,
            session_count,
            all_sessions_closed,
        }
    }
}

#[async_trait]
impl SessionDelegate for OutletSession {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        trace!("outlet on session({session_id}) start {addr}");
        self.session_id = session_id;
        self.session_info_map.insert(
            session_id,
            SessionInfo {
                sender: tx,
                common_info: self.common_data.clone(),
            },
        );

        // 先发送成功通知再计数；若发送失败则 net_session::run 不会调用 on_session_close，
        // 提前计数会导致 session_count 永久偏高，使 stop() 等满超时。
        if let Err(err) = self
            .output
            .send(ProxyMessage::O2iConnect(session_id, true, "".to_string()))
            .await
        {
            self.session_info_map.remove(&session_id);
            tokio::time::sleep(Duration::from_secs(5)).await;
            Err(anyhow!("on_session_start: {}", err))
        } else {
            // 只有成功时才增加计数，与 on_session_close 的 fetch_sub 对称
            self.session_count.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
    }

    async fn on_session_close(&mut self) -> anyhow::Result<()> {
        trace!("outlet on session({}) close", self.session_id);
        self.session_info_map.remove(&self.session_id);
        let _ = self
            .output
            .send(ProxyMessage::O2iDisconnect(self.session_id))
            .await;
        // 原子减少计数，为 0 时通知 stop() 等待者
        let prev = self.session_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            self.all_sessions_closed.notify_waiters();
        }
        Ok(())
    }

    async fn on_recv_frame(&mut self, frame: Bytes) -> anyhow::Result<()> {
        let encoded = self
            .common_data
            .encode_data_and_limiting(frame) // Bytes 直接传入，无需 to_vec()
            .await?;
        self.output
            .send(ProxyMessage::O2iRecvData(self.session_id, encoded))
            .await?;
        Ok(())
    }

    async fn on_recv_frame_from(
        &mut self,
        frame: Bytes,
        peer_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        if self.tunnel_type.is_socks5() {
            let encoded = self
                .common_data
                .encode_data_and_limiting(frame) // Bytes 直接传入，无需 to_vec()
                .await?;
            self.output
                .send(ProxyMessage::O2iRecvDataFrom(
                    self.session_id,
                    encoded,
                    peer_addr.to_string(),
                ))
                .await?;
        } else {
            self.on_recv_frame(frame).await?;
        }
        Ok(())
    }
}
