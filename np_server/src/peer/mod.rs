mod handle_push;
mod handle_request;
mod handle_response;
use crate::global::config::GLOBAL_CONFIG;
use crate::global::forward_rule::match_rule;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::player::Player;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use log::{debug, error, trace};
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::WriterMessage;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use np_proto::{generic, message_map};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// `Peer` 当前承载的连接角色。
///
/// `Control` 是完整登录后的控制连接，负责账号在线状态和控制消息。
/// `Forward` 是通过临时令牌绑定的转发连接或 QUIC 流，只承载代理流量，不记录登录历史，也不顶掉控制连接。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PeerConnectionKind {
    /// 尚未完成登录或临时令牌绑定。
    Unknown,
    /// 完整登录后的控制连接。
    Control,
    /// 临时令牌绑定后的转发连接。
    Forward,
}

/// 服务端接收到的一条套接字连接或 QUIC 流的运行状态。
///
/// `Peer` 初始为 `Unknown`，完整登录后变为 `Control`，通过临时令牌绑定后变为 `Forward`。
/// 显式区分角色可以避免转发连接被误当作新的用户登录。
pub struct Peer {
    /// 当前连接所在监听器的协议名。
    connection_protocol: &'static str,
    /// 当前连接的写消息通道。
    tx: Option<UnboundedSender<WriterMessage>>,
    /// 当前连接绑定的在线用户；未登录或未完成令牌绑定时为空。
    player: Option<Arc<RwLock<Player>>>,
    /// `net_session` 分配的会话 ID。
    session_id: u32,
    /// 转发连接 ID；控制连接默认使用 `session_id` 派生。
    connection_id: u64,
    /// 当前连接角色，用于区分控制连接和转发连接。
    connection_kind: PeerConnectionKind,
    /// 非 npipe 协议流量命中转发规则后使用的原始 TCP 转发写半边。
    traffic_forward_writer: Option<WriteHalf<TcpStream>>,
    /// 当前连接的远端地址。
    addr: SocketAddr,
    /// 入站流量计数器，从 `Player` 克隆的共享引用，用于无锁累加。
    traffic_rx: Option<Arc<AtomicU64>>,
    /// 出站流量计数器，从 `Player` 克隆的共享引用，用于无锁累加。
    traffic_tx: Option<Arc<AtomicU64>>,
    /// 登录历史记录 ID，用于登出时更新记录。
    login_record_id: u32,
}

impl Peer {
    pub(crate) fn new(connection_protocol: &'static str) -> Self {
        Peer {
            connection_protocol,
            tx: None,
            player: None,
            session_id: 0,
            connection_id: 0,
            connection_kind: PeerConnectionKind::Unknown,
            traffic_forward_writer: None,
            addr: SocketAddr::from(([0, 0, 0, 0], 0)),
            traffic_rx: None,
            traffic_tx: None,
            login_record_id: 0,
        }
    }

    #[inline]
    pub(crate) async fn send_response(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        assert!(serial < 0);
        // 委托给 Player::send_response，统一统计出站流量
        if self.connection_kind == PeerConnectionKind::Control {
            if let Some(ref player) = self.player {
                return player.read().await.send_response(serial, message);
            }
        }
        package_and_send_message(&self.tx, -serial, message, true)
    }

    #[inline]
    pub(crate) fn mark_control_connection(&mut self) {
        self.connection_kind = PeerConnectionKind::Control;
        self.connection_id = u64::from(self.session_id);
    }

    #[inline]
    pub(crate) fn mark_forward_connection(&mut self, connection_id: u64) {
        self.connection_kind = PeerConnectionKind::Forward;
        self.connection_id = connection_id;
    }

    #[inline]
    pub(crate) fn tx(&self) -> Option<UnboundedSender<WriterMessage>> {
        self.tx.clone()
    }

    #[inline]
    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    #[inline]
    pub(crate) fn session_id(&self) -> u32 {
        self.session_id
    }

    #[inline]
    pub(crate) fn connection_protocol(&self) -> &'static str {
        self.connection_protocol
    }

    // #[inline]
    // pub(crate) async fn send_request(&self, _message: &MessageType) -> anyhow::Result<MessageType> {
    //     todo!();
    // }

    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn send_push(&self, message: &MessageType) -> anyhow::Result<()> {
        // 委托给 Player::send_push，统一统计出站流量
        if self.connection_kind == PeerConnectionKind::Control {
            if let Some(ref player) = self.player {
                return player.read().await.send_push(message);
            }
        }
        package_and_send_message(&self.tx, 0, message, true)
    }

    pub async fn handle_message(
        &mut self,
        serial: i32,
        message: MessageType,
    ) -> anyhow::Result<MessageType> {
        match serial {
            s if s < 0 => self.handle_request(message).await,
            s if s > 0 => {
                self.handle_response(message).await?;
                Ok(MessageType::None)
            }
            _ => {
                // serial == 0
                self.handle_push(message).await?;
                Ok(MessageType::None)
            }
        }
    }

    // 模拟http 404请求结果
    async fn send_http_404_response(&self) -> anyhow::Result<()> {
        if let Some(ref tx) = self.tx {
            let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
            let formatted_time = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

            let str = format!(
                "HTTP/1.1 400 Bad Request\r\n\
                Server: nginx\r\n\
                Date: {formatted_time}\r\n\
                Content-Type: text/html\r\n\
                Content-Length: 150\r\n\
                Connection: close\r\n\
                \r\n\
                <html>\r\n\
                <head><title>400 Bad Request</title></head>\r\n\
                <body>\r\n\
                <center><h1>400 Bad Request</h1></center>\r\n\
                <hr><center>nginx</center>\r\n\
                </body>\r\n\
                </html>\r\n\
                "
            );

            tx.send(WriterMessage::Send(str.into(), true))?;
            tx.send(WriterMessage::Close)?;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    /// 创建流量转发通道
    async fn create_traffic_forward_channel(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        if let Some(ref tx) = self.tx {
            let target = GLOBAL_CONFIG
                .forward_rules
                .iter()
                .find(|rule| match_rule(&rule.matcher, buf))
                .map(|rule| rule.target);

            let addr = target.ok_or(anyhow!("No forward rule matched"))?;
            let stream = TcpStream::connect(addr).await?;

            // 设置 TCP 保活。
            let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
            let sf = SockRef::from(&stream);
            sf.set_tcp_keepalive(&ka)?;

            let (mut reader, writer) = tokio::io::split(stream);

            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buffer = BytesMut::with_capacity(4096);

                while let Ok(size) = reader.read_buf(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    let frame = buffer.split().freeze();
                    let _ = tx.send(WriterMessage::Send(frame, true));
                }

                let _ = tx.send(WriterMessage::Close);
            });
            self.traffic_forward_writer = Some(writer);
            Ok(())
        } else {
            Err(anyhow!("tx is none"))
        }
    }
}

#[async_trait]
impl SessionDelegate for Peer {
    async fn on_session_start(
        &mut self,
        session_id: u32,
        addr: &SocketAddr,
        tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        self.tx = Some(tx);
        self.session_id = session_id;
        self.addr = *addr;
        Ok(())
    }

    // 会话关闭回调
    async fn on_session_close(&mut self) -> anyhow::Result<()> {
        self.tx.take();

        // 更新登出时间
        if self.login_record_id > 0 {
            let now = Utc::now().naive_utc();
            let db = GLOBAL_DB_POOL.get().unwrap();
            if let Some(record) = login_history::Entity::find_by_id(self.login_record_id)
                .one(db)
                .await?
            {
                let login_time = record.login_time;
                let duration = (now - login_time).num_seconds().max(0) as i32;
                let mut active: login_history::ActiveModel = record.into();
                active.logout_time = Set(Some(now));
                active.duration_secs = Set(Some(duration));
                if let Err(e) = active.update(db).await {
                    error!(
                        "update login history logout failed, record_id:{}, error:{}",
                        self.login_record_id, e
                    );
                }
            }
        }
        self.login_record_id = 0;

        // 合并为单次写锁，避免 read-check → write-act 的 TOCTOU 窗口
        if let Some(player) = self.player.take() {
            let mut p = player.write().await;
            match self.connection_kind {
                PeerConnectionKind::Control | PeerConnectionKind::Unknown => {
                    if p.get_session_id() == self.session_id {
                        p.on_disconnect_session();
                    }
                }
                PeerConnectionKind::Forward => {
                    p.remove_forward_connection(self.connection_id);
                }
            }
        }
        // 关闭流量转发通道
        if let Some(mut writer) = self.traffic_forward_writer.take() {
            let _ = writer.shutdown().await;
        }
        Ok(())
    }

    /// 数据粘包处理
    ///
    /// 注意：这个函数只能使用消耗 buffer 数据的函数，否则框架会一直循环调用本函数来驱动处理消息
    ///
    async fn on_try_extract_frame(
        &mut self,
        buffer: &mut BytesMut,
    ) -> anyhow::Result<Option<Bytes>> {
        if !buffer.is_empty()
            && buffer[0] != 33u8
            && self.traffic_forward_writer.is_none()
            && self.create_traffic_forward_channel(buffer).await.is_err()
        {
            debug!("bad flag");
            self.send_http_404_response().await?;
            return Err(anyhow!("Bad flag"));
        }

        if let Some(ref mut writer) = self.traffic_forward_writer {
            let frame = buffer.split().freeze();
            writer.write_all(&frame).await?;
            return Ok(None);
        }

        // 数据小于5字节,继续读取数据
        if buffer.len() < 5 {
            return Ok(None);
        }

        // 读取包长度
        let buf = buffer.get(1..5).unwrap();
        let len = BigEndian::read_u32(buf) as usize;

        // 超出最大限制
        if len == 0 || len >= 1024 * 1024 * 2 {
            debug!("Message too long");
            self.send_http_404_response().await?;
            return Err(anyhow!("Message too long"));
        }

        // 数据不够,继续读取数据
        if buffer.len() < 5 + len {
            return Ok(None);
        }

        // 拆出这个包的数据
        let frame = buffer.split_to(5 + len).split_off(5).freeze();

        Ok(Some(frame))
    }

    // 收到一个完整的消息包
    async fn on_recv_frame(&mut self, frame: Bytes) -> anyhow::Result<()> {
        if frame.len() < 8 {
            debug!("message length is too small");
            self.send_http_404_response().await?;
            return Err(anyhow!("message length is too small"));
        }

        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

        match message_map::decode_message(msg_id, &frame[8..]) {
            Ok(message) => {
                let start_time = Instant::now();

                let result = self.handle_message(serial, message).await;

                // 记录耗时比较长的接口
                let ms = Instant::now().duration_since(start_time).as_millis();
                if ms > 20 {
                    trace!("Request {} consumes {}ms", msg_id, ms);
                }

                match result {
                    Ok(msg) => {
                        if serial < 0 {
                            if let MessageType::None = msg {
                                // 请求不应该不回复
                                error!("The response to request {} is empty", msg_id);
                                self.send_response(
                                    serial,
                                    &MessageType::GenericError(generic::Error {
                                        number: generic::ErrorCode::InternalError.into(),
                                        message: "response is empty".to_string(),
                                    }),
                                )
                                .await?;
                            } else {
                                self.send_response(serial, &msg).await?;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Request error: {}, message id: {}", err, msg_id);

                        self.send_response(
                            serial,
                            &MessageType::GenericError(generic::Error {
                                number: generic::ErrorCode::InternalError.into(),
                                message: format!("{}", err),
                            }),
                        )
                        .await?;
                    }
                }
            }
            Err(err) => {
                debug!("decode message error: {err}");
                // self.send_http_404_response().await?;

                // 消息解码失败
                self.send_response(
                    serial,
                    &MessageType::GenericError(generic::Error {
                        number: generic::ErrorCode::InternalError.into(),
                        message: format!("decode message error: {}", err),
                    }),
                )
                .await?;

                return Err(anyhow!(err));
            }
        }

        Ok(())
    }
}

#[inline]
pub(crate) fn package_and_send_message(
    tx: &Option<UnboundedSender<WriterMessage>>,
    serial: i32,
    message: &MessageType,
    flush: bool,
) -> anyhow::Result<()> {
    if let Some(ref tx) = tx {
        if let Some(message_id) = get_message_id(message) {
            let message_size = get_message_size(message);
            let mut buf = Vec::with_capacity(message_size + 14);

            byteorder::WriteBytesExt::write_u8(&mut buf, 33u8)?;
            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
            byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
            encode_raw_message(message, &mut buf);

            if let Err(error) = tx.send(WriterMessage::Send(Bytes::from(buf), flush)) {
                error!("Failed to send message: {}", error);
                return Err(anyhow!("Failed to send message: {}", error));
            }
        }
    } else {
        debug!("tx is none, cannot send message");
    }
    Ok(())
}
