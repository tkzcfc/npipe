use crate::proxy::{OutputFuncType, ProxyMessage};
use bytes::BytesMut;
use log::{debug, error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{lookup_host, TcpSocket, TcpStream, UdpSocket};
use tokio::sync::Mutex;

type TcpWriter = Mutex<WriteHalf<TcpStream>>;

enum ClientType {
    TCP(TcpWriter),
    UDP(SocketAddr),
}

pub struct Outlet {
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
    udp_socket: Mutex<Option<Arc<UdpSocket>>>,
    udp_session_id_map: Arc<Mutex<HashMap<SocketAddr, u32>>>,
    on_output_callback: OutputFuncType,
    description: String,
}

impl Outlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Self {
        Self {
            udp_socket: Mutex::new(None),
            client_map: Arc::new(Mutex::new(HashMap::new())),
            udp_session_id_map: Arc::new(Mutex::new(HashMap::new())),
            on_output_callback,
            description,
        }
    }

    pub async fn input(&self, message: ProxyMessage) {
        match message {
            ProxyMessage::I2oConnect(session_id, is_tcp, addr) => {
                info!("session: {session_id}, connect to: {addr}");
                if self.client_map.lock().await.contains_key(&session_id) {
                    let output_callback = self.on_output_callback.clone();
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(
                            session_id,
                            false,
                            "repeated connection".into(),
                        ))
                        .await
                    });
                } else {
                    let mut addr_opt = None;
                    // 地址解析
                    if let Ok(addr) = addr.parse::<SocketAddr>() {
                        addr_opt = Some(addr);
                    } else {
                        let mut addr_err = format!("The address format is invalid: '{}'", addr);
                        // 尝试解析域名
                        let s: Vec<&str> = addr.split(":").collect();
                        if s.len() == 2 && s[1].parse::<u16>().is_ok() {
                            let domain = s[0];
                            match lookup_host(domain).await {
                                Ok(mut addrs) => {
                                    while let Some(addr) = addrs.next() {
                                        // match addr {
                                        //     SocketAddr::V4(addr) => println!("IPv4: {}", addr),
                                        //     SocketAddr::V6(addr) => println!("IPv6: {}", addr),
                                        // }

                                        // 使用第一个解析到的地址
                                        let mut addr = addr.clone();
                                        // 端口重定向
                                        addr.set_port(s[1].parse::<u16>().unwrap());
                                        addr_opt = Some(addr);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    addr_err = format!("Failed to resolve domain: {}", e);
                                }
                            }
                        }

                        // 无法连接,地址解析失败
                        if addr_opt.is_none() {
                            let output_callback = self.on_output_callback.clone();
                            tokio::spawn(async move {
                                output_callback(ProxyMessage::O2iConnect(
                                    session_id, false, addr_err,
                                ))
                                .await
                            });
                        }
                    }

                    if let Some(addr) = addr_opt {
                        if is_tcp {
                            match tcp_connect(
                                addr,
                                session_id,
                                self.on_output_callback.clone(),
                                self.client_map.clone(),
                            )
                            .await
                            {
                                Ok(client) => {
                                    self.client_map
                                        .lock()
                                        .await
                                        .insert(session_id, ClientType::TCP(client));
                                }
                                Err(err) => {
                                    let output_callback = self.on_output_callback.clone();
                                    // 连接失败
                                    tokio::spawn(async move {
                                        output_callback(ProxyMessage::O2iConnect(
                                            session_id,
                                            false,
                                            err.to_string(),
                                        ))
                                        .await;
                                    });
                                    return;
                                }
                            }
                        } else {
                            self.client_map
                                .lock()
                                .await
                                .insert(session_id, ClientType::UDP(addr));
                            self.udp_session_id_map
                                .lock()
                                .await
                                .insert(addr, session_id);
                        }

                        let output_callback = self.on_output_callback.clone();
                        tokio::spawn(async move {
                            output_callback(ProxyMessage::O2iConnect(session_id, true, "".into()))
                                .await;
                        });
                    }
                }
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                if let Some(client_type) = self.client_map.lock().await.get(&session_id) {
                    match client_type {
                        ClientType::TCP(writer) => {
                            if let Err(err) = writer.lock().await.write_all(&data).await {
                                error!("[{session_id}] tcp write_all err: {err}");
                            }
                        }
                        ClientType::UDP(addr) => {
                            let mut socket = self.udp_socket.lock().await;
                            if socket.is_none() {
                                let udp_socket = Arc::new(
                                    UdpSocket::bind("0.0.0.0:0").await.expect("udp bind error"),
                                );
                                *socket = Some(udp_socket.clone());

                                let udp_session_id_map = self.udp_session_id_map.clone();
                                let on_output_callback = self.on_output_callback.clone();

                                tokio::spawn(async move {
                                    let mut buffer = Vec::with_capacity(65536);
                                    loop {
                                        buffer.clear();
                                        match udp_socket.recv_buf_from(&mut buffer).await {
                                            Ok((size, addr)) => {
                                                let received_data = &buffer[..size];
                                                if let Some(session_id) =
                                                    udp_session_id_map.lock().await.get(&addr)
                                                {
                                                    on_output_callback(ProxyMessage::O2iRecvData(
                                                        *session_id,
                                                        received_data.to_vec(),
                                                    ))
                                                    .await;
                                                }
                                            }
                                            Err(err) => {
                                                error!("Failed to read from udp socket: {}", err);
                                            }
                                        }
                                    }
                                });
                            }
                            if let Some(socket) = &*socket {
                                if let Err(err) = socket.send_to(&data, addr).await {
                                    error!("[{session_id}] udp send_to({addr}) err: {err}");
                                }
                            }
                        }
                    }
                }
            }
            ProxyMessage::I2oDisconnect(session_id) => {
                info!("disconnect session: {session_id}");

                if let Some(client_type) = self.client_map.lock().await.remove(&session_id) {
                    match client_type {
                        ClientType::UDP(addr) => {
                            self.udp_session_id_map.lock().await.remove(&addr);
                        }
                        ClientType::TCP(writer) => {
                            let _ = writer.into_inner().shutdown().await;
                        }
                    }
                }
            }
            _ => {
                panic!("error message")
            }
        }
    }

    pub fn description(&self) -> &String {
        &self.description
    }
}

async fn tcp_connect(
    addr: SocketAddr,
    session_id: u32,
    on_output_callback: OutputFuncType,
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
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
                Ok(n) => {
                    if n == 0 {
                        debug!("socket[{}] closed.", addr);
                        break;
                    } else {
                        on_output_callback(ProxyMessage::O2iRecvData(
                            session_id,
                            buffer.split().to_vec(),
                        ))
                        .await;
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket[{}]: {}", addr, e);
                    // socket读错误,主动断开
                    break;
                }
            }
        }
        on_output_callback(ProxyMessage::O2iDisconnect(session_id)).await;
        client_map.lock().await.remove(&session_id);
    });

    Ok(Mutex::new(writer))
}
