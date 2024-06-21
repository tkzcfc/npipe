use crate::proxy::{OutputFuncType, ProxyMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::sync::Mutex;

enum ClientType {
    TCP(TcpClient),
    UDP(SocketAddr),
}

struct Outlet {
    client_map: Mutex<HashMap<u32, ClientType>>,
    udp_socket: Mutex<Option<UdpSocket>>,
    on_output_callback: OutputFuncType,
}

impl Outlet {
    pub fn new(on_output_callback: OutputFuncType) -> Self {
        Self {
            udp_socket: Mutex::new(None),
            client_map: Mutex::new(HashMap::new()),
            on_output_callback,
        }
    }

    pub async fn input(&self, message: ProxyMessage) -> anyhow::Result<()> {
        match message {
            ProxyMessage::I2oConnect(session_id, is_tcp, addr) => {
                if self.client_map.lock().await.contains_key(&session_id) {
                    let _ = (self.on_output_callback)(ProxyMessage::O2iConnect(
                        session_id,
                        false,
                        "Repeated connection".into(),
                    ))
                    .await?;
                    return Ok(());
                }
                if is_tcp {
                    match TcpClient::connect(addr).await {
                        Ok(client) => {
                            self.client_map
                                .lock()
                                .await
                                .insert(session_id, ClientType::TCP(client));
                        }
                        Err(err) => {
                            // 连接失败
                            let _ = (self.on_output_callback)(ProxyMessage::O2iConnect(
                                session_id,
                                false,
                                err.to_string(),
                            ))
                            .await?;
                            return Ok(());
                        }
                    }
                } else {
                    self.client_map
                        .lock()
                        .await
                        .insert(session_id, ClientType::UDP(addr));
                }
                let _ = (self.on_output_callback)(ProxyMessage::O2iConnect(
                    session_id,
                    true,
                    "".into(),
                ))
                .await?;
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                if let Some(client_type) = self.client_map.lock().await.get(&session_id) {
                    match client_type {
                        ClientType::TCP(client) => {
                            client.send(data).await?;
                        }
                        ClientType::UDP(addr) => {
                            let mut socket = self.udp_socket.lock().await;
                            if socket.is_none() {
                                *socket = Some(UdpSocket::bind("0.0.0.0:0").await?);
                            }
                            if let Some(socket) = &*socket {
                                socket.send_to(&data, addr).await?;
                            }
                        }
                    }
                }
            }
            ProxyMessage::I2oDisconnect(session_id) => {}
            _ => {
                panic!("error message")
            }
        }
        Ok(())
    }
}

struct TcpClient {
    reader: ReadHalf<TcpStream>,
    writer: Mutex<WriteHalf<TcpStream>>,
}

impl TcpClient {
    async fn connect(addr: SocketAddr) -> anyhow::Result<Self> {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            reader,
            writer: Mutex::new(writer),
        })
    }

    async fn send(&self, frame: Vec<u8>) -> anyhow::Result<()> {
        self.writer.lock().await.write_all(&frame).await?;
        Ok(())
    }
}

//
// struct UdpClient {
//     socket: UdpSocket,
// }
//
//
// impl UdpClient {
//     async fn connect(addr: SocketAddr) -> anyhow::Result<Self> {
//         let socket = UdpSocket::bind("0.0.0.0:0").await?;
//         socket.connect(&addr).await?;
//
//         let socket_cloned = socket.clone();
//
//         Ok(Self {
//             socket
//         })
//     }
//
//     async fn send(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
//         self.socket.send(&frame).await?;
//         Ok(())
//     }
// }
