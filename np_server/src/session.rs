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
                        if let Ok(result) = try_extract_frame(&mut buffer) {
                            if let Some(frame) = result {
                                self.on_recv_pkg_frame(frame);
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

    fn on_recv_pkg_frame(&self, frame : Vec<u8>) {
        info!("recv pkg frame: {}", frame.iter()
                                    .map(|x| (x + 0) as char)
                                    .collect::<String>());
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

