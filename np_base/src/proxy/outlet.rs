use crate::proxy::{OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use bytes::BytesMut;
use log::{debug, error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{lookup_host, TcpStream, UdpSocket};
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
                let output_callback = self.on_output_callback.clone();
                if let Err(err) = self.on_i2o_connect(session_id, is_tcp, addr).await {
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(
                            session_id,
                            false,
                            err.to_string(),
                        ))
                        .await
                    });
                } else {
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(session_id, true, "".into()))
                            .await;
                    });
                }
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                self.on_i2o_send_data(session_id, data).await;
            }
            ProxyMessage::I2oDisconnect(session_id) => {
                self.on_i2o_disconnect(session_id).await;
            }
            _ => {
                panic!("error message")
            }
        }
    }

    async fn on_i2o_connect(
        &self,
        session_id: u32,
        is_tcp: bool,
        addr: String,
    ) -> anyhow::Result<()> {
        info!("session: {session_id}, connect to: {addr}");

        if self.client_map.lock().await.contains_key(&session_id) {
            return Err(anyhow!("repeated connection"));
        }

        if is_tcp {
            let client = tcp_connect(
                addr,
                session_id,
                self.on_output_callback.clone(),
                self.client_map.clone(),
            )
            .await?;

            self.client_map
                .lock()
                .await
                .insert(session_id, ClientType::TCP(client));
        } else {
            let addr = parse_socket_addr(addr).await?;
            self.client_map
                .lock()
                .await
                .insert(session_id, ClientType::UDP(addr.clone()));
            self.udp_session_id_map
                .lock()
                .await
                .insert(addr, session_id);
        }

        Ok(())
    }

    async fn on_i2o_send_data(&self, session_id: u32, data: Vec<u8>) {
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
                        let udp_socket =
                            Arc::new(UdpSocket::bind("0.0.0.0:0").await.expect("udp bind error"));
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

    async fn on_i2o_disconnect(&self, session_id: u32) {
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

    pub fn description(&self) -> &String {
        &self.description
    }
}

async fn tcp_connect(
    addr: String,
    session_id: u32,
    on_output_callback: OutputFuncType,
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
) -> anyhow::Result<TcpWriter> {
    let stream = TcpStream::connect(&addr).await?;
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

async fn parse_socket_addr(addr: String) -> anyhow::Result<SocketAddr> {
    // 地址解析
    if let Ok(addr) = addr.parse::<SocketAddr>() {
        return Ok(addr);
    }
    // 尝试解析域名
    let s: Vec<&str> = addr.split(":").collect();
    if s.len() == 2 && s[1].parse::<u16>().is_ok() {
        let domain = s[0];
        let mut addrs = lookup_host(domain).await?;
        while let Some(addr) = addrs.next() {
            // match addr {
            //     SocketAddr::V4(addr) => println!("IPv4: {}", addr),
            //     SocketAddr::V6(addr) => println!("IPv6: {}", addr),
            // }

            // 使用第一个解析到的地址
            let mut addr = addr.clone();
            // 端口重定向
            addr.set_port(s[1].parse::<u16>().unwrap());
            return Ok(addr);
        }
    }

    Err(anyhow!("The address format is invalid: '{}'", addr))
}
