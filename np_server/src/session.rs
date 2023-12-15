use std::io;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use std::sync::Arc;
use bytes::{BytesMut};
use log::{debug, error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use byteorder::{ByteOrder, BigEndian};
use crate::player::Player;
use prost::Message;
use np_base::client_server;
use np_base::message_map::parse_message;


#[derive(PartialEq)]
pub enum SessionStatus {
    Connected,
    Disconnecting,
    Disconnected
}

pub struct Session {
    pub socket : TcpStream,
    pub addr: SocketAddr,
    pub player: Option<Arc<RwLock<Player>>>,
    status: SessionStatus,
}

impl Session {

    pub fn new(socket: TcpStream, addr: SocketAddr) -> Arc<RwLock<Session>> {
        Arc::new(RwLock::new(Session {
            socket,
            addr,
            player: Option::None,
            status: SessionStatus::Connected
        }))
    }

    pub async fn disconnect(&mut self) {
        match self.status {
            SessionStatus::Connected => {
                self.status = SessionStatus::Disconnecting;

                if let Some(ref player) = self.player {
                    player.write().await.on_disconnect_session().await;
                    self.player = Option::None;
                }

                if let Err(err) = self.socket.shutdown().await {
                    error!("socket[{}] shutdown error: {:?}", self.addr, err);
                }
                else {
                    debug!("socket[{}] shutdown success.", self.addr);
                }
                self.status = SessionStatus::Disconnected;
            }
            _=>{}
        }
    }

    pub async fn write_frame(&mut self, frame: Vec<u8>) -> io::Result<()> {
        self.socket.write_u32(frame.len() as u32).await?;
        self.socket.write_all(&frame[..]).await
    }


    pub fn status(&self) -> &SessionStatus {
        return &self.status;
    }

    pub async fn reset_player(&mut self, value: Option<Arc<RwLock<Player>>>) {
        // 一般只有账号在其他地方登录才会导致重置player，此时可以向这个会话发送被顶号消息然后关闭连接
        if let Some(ref player) = self.player {
            // 这个时候应该是穿的None才对
            assert!(value.is_none());

            // 发送顶号通知
            player.write().await.on_terminate_old_session().await;
            self.disconnect().await;
        }
        else {
            // 正常初始化
            assert!(value.is_some());
            self.player = value;

            // 玩家登录成功
            if let Some(ref player) = self.player {
                player.write().await.on_connect_session().await;
            }
        }
    }

    pub async fn read_poll(&mut self) {
        let mut buffer = BytesMut::with_capacity(1024);

        // 循环读取数据
        loop {
            match self.socket.read_buf(&mut buffer).await {
                // n为0表示对端已经关闭连接。
                Ok(n) if n == 0 => {
                    debug!("socket[{}] closed.", self.addr);
                    // 客户端主动断开
                    self.disconnect().await;
                    return;
                }
                Ok(_n) => {
                    // info!("socket[{}] read len: {}, total len: {}", self.addr, _n, buffer.len());

                    loop {
                        match self.status {
                            SessionStatus::Connected=>{},
                            // 已经断开或正在端口，不继续处理后续数据
                            _=>break
                        }
                        if let Ok(result) = try_extract_frame(&mut buffer) {
                            if let Some(frame) = result {
                                self.on_recv_pkg_frame(frame).await;
                            }
                            else {
                                break;
                            }
                        }
                        else {
                            debug!("data parsing failed");
                            // 消息解析错误主动断开
                            self.disconnect().await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket[{}]: {}", self.addr, e);
                    // socket读错误
                    self.disconnect().await;
                    return;
                }
            }
        }
    }

    async fn on_recv_pkg_frame(&mut self, frame : Vec<u8>) {
        if frame.len() < 8 {
            self.disconnect().await;
            return;
        }
        // 消息序号
        let serial: i32 = BigEndian::read_i32(&frame[0..4]);
        // 消息类型id
        let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);
        // // 消息数据
        // let bytes = &frame[8..];
        // println!("msglen:{}", bytes.len());

        match parse_message(msg_id, &frame[8..]) {
            Ok(message) => {

            },
            Err(err) => {
                error!("pb parse error: {}", err);
            }
        }
    }

    async fn on_login_requst(&mut self, message: client_server::LoginReq) {

    }
}

// 数据粘包处理
fn try_extract_frame(buffer: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
    // 数据小于4字节
    if buffer.len() < 4 {
        return Ok(None);
    }

    let bin = buffer.get(0..4).unwrap();
    let len = BigEndian::read_u32(bin) as usize;

    // 超出最大限制
    if len <= 0 || len >= 1024 * 1024 * 5 {
        return Err(io::Error::new(io::ErrorKind::Other, String::from("bad length")));
    }

    // 数据不够
    if buffer.len() < 4 + len {
        return Ok(None);
    }

    let frame = buffer.split_to(4 + len).split_off(4).to_vec();

    Ok(Some(frame))
}

