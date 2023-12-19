use log::error;
use np_base::message_map::{encode_message, MessageType};
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::RwLock;
use tokio::time;
use tokio::time::Instant;

enum Response {
    // 请求回复的消息
    Message(MessageType),
    // 等待回复中...
    Waiting,
    // 请求被取消
    Cancel,
    // 请求出错，如服务器返回的消息客户端解码失败
    Error,
}

pub struct Client {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    serial: i32,
    response_map: HashMap<i32, Response>,
}

impl Client {
    pub fn new(addr: SocketAddr) -> Arc<RwLock<Client>> {
        Arc::new(RwLock::new(Client {
            addr,
            stream: None,
            serial: 0i32,
            response_map: HashMap::new(),
        }))
    }

    pub async fn connect(&mut self) -> Result<(), io::Error> {
        let _ = self.disconnect().await;

        let socket = if self.addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(self.addr).await?;
        self.stream = Some(stream);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), io::Error> {
        if let Some(ref mut stream) = self.stream {
            stream.shutdown().await?;
            self.stream = None;
        }
        Ok(())
    }

    pub async fn send_request(&mut self, message: &MessageType) -> Result<MessageType, io::Error> {
        if let Some(ref mut stream) = self.stream {
            self.serial += 1;
            if let Some((id, buf)) = encode_message(message) {
                let serial = -self.serial;

                stream.write_i32(serial).await?;
                stream.write_u32(id).await?;
                stream.write_all(&buf).await?;

                self.response_map.insert(serial, Response::Waiting);

                let start = Instant::now();
                // 检测间隔时间 20毫秒检测一次
                let mut interval = time::interval(Duration::from_millis(20));
                // 10超时等待时间
                while Instant::now().duration_since(start) < Duration::from_secs(10) {
                    interval.tick().await;
                    if let Some(response) = self.response_map.get(&serial) {
                        match response {
                            Response::Message(_message) => {
                                if let Some(message) = self.response_map.remove(&serial) {
                                    if let Response::Message(msg) = message {
                                        return Ok(msg);
                                    }
                                }

                                // 不可能出现的错误
                                self.response_map.remove(&serial);
                                return Err(io::Error::new(ErrorKind::Other, "impossible errors"));
                            }
                            Response::Waiting => {}
                            Response::Cancel => {
                                // 请求被取消
                                self.response_map.remove(&serial);
                                return Err(io::Error::new(
                                    ErrorKind::TimedOut,
                                    "request cancelled",
                                ));
                            }
                            Response::Error => {
                                self.response_map.remove(&serial);
                                return Err(io::Error::new(
                                    ErrorKind::Other,
                                    "protocol decoding failed",
                                ));
                            }
                        }
                    } else {
                        // 连接已重置
                        return Err(io::Error::new(
                            ErrorKind::ConnectionReset,
                            "connection reset",
                        ));
                    }
                }

                // 请求等待回复超时
                self.response_map.remove(&serial);
                return Err(io::Error::new(ErrorKind::TimedOut, "request timeout"));
            }

            error!("encode message error!");
            return Err(io::Error::new(ErrorKind::Other, "encode message error!"));
        }
        Err(io::Error::new(ErrorKind::NotConnected, "not connected"))
    }
}
