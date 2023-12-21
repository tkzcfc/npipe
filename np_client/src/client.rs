use bytes::BytesMut;
use log::{debug, error};
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
    stream: Option<Arc<RwLock<TcpStream>>>,
    serial: i32,
    response_map: Arc<RwLock<HashMap<i32, Response>>>,
}

impl Client {
    pub fn new(addr: SocketAddr) -> Arc<RwLock<Client>> {
        let client = Arc::new(RwLock::new(Client {
            addr,
            stream: None,
            serial: 0i32,
            response_map: Arc::new(RwLock::new(HashMap::new())),
        }));

        let client_cloned = client.clone();
        // tokio::spawn()

        client
    }

    pub async fn connect(&mut self) -> Result<(), io::Error> {
        let _ = self.disconnect().await;

        let socket = if self.addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(self.addr).await?;
        let stream = Arc::new(RwLock::new(stream));
        self.stream = Some(stream.clone());

        // 单独开一个协程处理读逻辑
        tokio::spawn(async move {
            // let mut buffer = BytesMut::with_capacity(1024);
            // loop {
            //     match stream.read_buf(&mut buffer).await {
            //         // n为0表示对端已经关闭连接。
            //         Ok(n) if n == 0 => {
            //             debug!("socket[{}] closed.", self.addr);
            //             // 客户端主动断开
            //             self.disconnect().await;
            //             return;
            //         }
            //         Ok(_n) => {
            //             // info!("socket[{}] read len: {}, total len: {}", self.addr, _n, buffer.len());
            //
            //             loop {
            //                 match self.status {
            //                     SessionStatus::Connected => {}
            //                     // 已经断开或正在端口，不继续处理后续数据
            //                     _ => break,
            //                 }
            //                 if let Ok(result) = try_extract_frame(&mut buffer) {
            //                     if let Some(frame) = result {
            //                         self.on_recv_pkg_frame(frame).await;
            //                     } else {
            //                         break;
            //                     }
            //                 } else {
            //                     debug!("data parsing failed");
            //                     // 消息解析错误主动断开
            //                     self.disconnect().await;
            //                     return;
            //                 }
            //             }
            //         }
            //         Err(e) => {
            //             error!("Failed to read from socket[{}]: {}", self.addr, e);
            //             // socket读错误
            //             self.disconnect().await;
            //             return;
            //         }
            //     }
            // }
        });

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), io::Error> {
        self.response_map.write().await.clear();
        if let Some(ref mut stream) = self.stream {
            stream.write().await.shutdown().await?;
            self.stream = None;
        }
        Ok(())
    }

    pub async fn send_request(&mut self, message: &MessageType) -> Result<MessageType, io::Error> {
        if let Some(ref mut stream) = self.stream {
            // 防止请求序号越界
            if self.serial >= std::i32::MAX {
                self.serial = 0;
            }
            self.serial += 1;

            if let Some((id, buf)) = encode_message(message) {
                let serial = -self.serial;
                // 防止stream 的生命周期将一直延续到当前作用域结束
                // 将相关操作放在一个单独的块，在使用 stream 后立刻释放锁
                {
                    let ref mut stream = stream.write().await;
                    stream.write_i32(serial).await?;
                    stream.write_u32(id).await?;
                    stream.write_all(&buf).await?;
                }

                self.response_map
                    .write()
                    .await
                    .insert(serial, Response::Waiting);

                let start = Instant::now();
                // 检测间隔时间 20毫秒检测一次
                let mut interval = time::interval(Duration::from_millis(20));
                // 10超时等待时间
                while Instant::now().duration_since(start) < Duration::from_secs(10) {
                    interval.tick().await;
                    if let Some(response) = self.response_map.read().await.get(&serial) {
                        match response {
                            Response::Message(_message) => {
                                if let Some(message) =
                                    self.response_map.write().await.remove(&serial)
                                {
                                    if let Response::Message(msg) = message {
                                        return Ok(msg);
                                    }
                                }
                                // 不可能出现的错误
                                self.response_map.write().await.remove(&serial);
                                return Err(io::Error::new(ErrorKind::Other, "impossible errors"));
                            }
                            Response::Waiting => {}
                            Response::Cancel => {
                                // 请求被取消
                                self.response_map.write().await.remove(&serial);
                                return Err(io::Error::new(
                                    ErrorKind::TimedOut,
                                    "request cancelled",
                                ));
                            }
                            Response::Error => {
                                self.response_map.write().await.remove(&serial);
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
                self.response_map.write().await.remove(&serial);
                return Err(io::Error::new(ErrorKind::TimedOut, "request timeout"));
            }

            error!("encode message error!");
            return Err(io::Error::new(ErrorKind::Other, "encode message error!"));
        }
        Err(io::Error::new(ErrorKind::NotConnected, "not connected"))
    }
}
