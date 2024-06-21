use crate::proxy::{OutputFuncType, ProxyMessage};
use bytes::BytesMut;
use log::{debug, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::sync::Mutex;

type TcpWriter = Mutex<WriteHalf<TcpStream>>;

enum ClientType {
    TCP(TcpWriter),
    UDP(SocketAddr),
}

pub struct Outlet {
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
                info!("session: {session_id}, connect to: {addr}");
                if self.client_map.lock().await.contains_key(&session_id) {
                    (self.on_output_callback)(ProxyMessage::O2iConnect(
                        session_id,
                        false,
                        "repeated connection".into(),
                    ))
                    .await?;
                } else {
                    if is_tcp {
                        match tcp_connect(addr, session_id, self.on_output_callback.clone()).await {
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
                    (self.on_output_callback)(ProxyMessage::O2iConnect(
                        session_id,
                        true,
                        "".into(),
                    ))
                    .await?;
                }
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                if let Some(client_type) = self.client_map.lock().await.get(&session_id) {
                    match client_type {
                        ClientType::TCP(writer) => {
                            writer.lock().await.write_all(&data).await?;
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
            ProxyMessage::I2oDisconnect(session_id) => {
                info!("disconnect session: {session_id}");
                self.client_map.lock().await.remove(&session_id);
            }
            _ => {
                panic!("error message")
            }
        }
        Ok(())
    }
}

async fn tcp_connect(
    addr: SocketAddr,
    session_id: u32,
    on_output_callback: OutputFuncType,
) -> anyhow::Result<TcpWriter> {
    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    let stream = socket.connect(addr).await?;
    let (mut reader, writer) = tokio::io::split(stream);

    tokio::spawn(async move {
        let mut buffer = BytesMut::with_capacity(1024);
        loop {
            match reader.read_buf(&mut buffer).await {
                // n为0表示对端已经关闭连接。
                Ok(n) if n == 0 => {
                    debug!("socket[{}] closed.", addr);
                    break;
                }
                _ => {}
            }
        }
        let _ = on_output_callback(ProxyMessage::O2iDisconnect(session_id)).await;
    });

    Ok(Mutex::new(writer))
}
