use std::io;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use std::sync::Arc;
use bytes::{Buf, BytesMut};
use log::{error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use byteorder::{ByteOrder, BigEndian};

pub struct Session {
    pub socket : TcpStream,
    pub addr: SocketAddr
}

impl Session {

    pub fn new(socket: TcpStream, addr: SocketAddr) -> Arc<RwLock<Session>> {
        Arc::new(RwLock::new(Session {
            socket,
            addr
        }))
    }

    pub async fn disconnect(&mut self) {
        if let Err(err) = self.socket.shutdown().await {
            error!("socket[{}] shutdown error: {:?}", self.addr, err);
        }
        else {
            info!("socket[{}] shutdown success.", self.addr);
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
                    info!("socket[{}] closed.", self.addr);
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
                            info!("data parsing failed");
                            // 消息解析错误主动断开
                            self.disconnect().await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket[{}]: {}", self.addr, e);
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

