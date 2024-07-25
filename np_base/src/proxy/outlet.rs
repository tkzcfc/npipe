use crate::proxy::crypto::{get_method, EncryptionMethod};
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use base64::prelude::*;
use bytes::BytesMut;
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use socket2::{SockRef, TcpKeepalive};
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Instant};

type TcpWriter = Mutex<WriteHalf<TcpStream>>;

enum ClientType {
    TCP(TcpWriter, bool, EncryptionMethod, Vec<u8>),
    UDP(Arc<UdpSocket>, Arc<RwLock<Instant>>, bool, EncryptionMethod, Vec<u8>),
}

pub struct Outlet {
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
    on_output_callback: OutputFuncType,
    description: String,
}

impl Outlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Self {
        Self {
            client_map: Arc::new(Mutex::new(HashMap::new())),
            on_output_callback,
            description,
        }
    }

    pub async fn input(&self, message: ProxyMessage) {
        if let Err(err) = self.input_internal(message).await {
            error!("outlet input error: {}", err.to_string());
        }
    }

    async fn input_internal(&self, message: ProxyMessage) -> anyhow::Result<()> {
        match message {
            ProxyMessage::I2oConnect(
                session_id,
                is_tcp,
                is_compressed,
                addr,
                encryption_method,
                encryption_key,
                client_addr,
            ) => {
                trace!("I2oConnect: session_id:{session_id}, addr:{addr}, is_tcp:{is_tcp}");
                let output_callback = self.on_output_callback.clone();
                if let Err(err) = self
                    .on_i2o_connect(
                        session_id,
                        is_tcp,
                        is_compressed,
                        addr.clone(),
                        encryption_method,
                        encryption_key,
                    )
                    .await
                {
                    error!(
                        "Failed to connect to {}, error: {}, remote client addr {}",
                        addr,
                        err.to_string(),
                        client_addr
                    );
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(
                            session_id,
                            false,
                            err.to_string(),
                        ))
                        .await
                    });
                } else {
                    info!(
                        "Successfully connected to {}, remote client addr {}",
                        addr, client_addr
                    );
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(session_id, true, "".into()))
                            .await;
                    });
                }
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                // trace!("I2oSendData: session_id:{session_id}");
                self.on_i2o_send_data(session_id, data).await?;
            }
            ProxyMessage::I2oDisconnect(session_id) => {
                trace!("I2oDisconnect: session_id:{session_id}");
                self.on_i2o_disconnect(session_id).await?;
            }
            _ => {
                return Err(anyhow!("Unknown message"));
            }
        }
        Ok(())
    }

    async fn on_i2o_connect(
        &self,
        session_id: u32,
        is_tcp: bool,
        is_compressed: bool,
        addr: String,
        encryption_method: String,
        encryption_key: String,
    ) -> anyhow::Result<()> {
        let encryption_method = get_method(&encryption_method);
        let encryption_key = BASE64_STANDARD.decode(encryption_key.as_bytes())?;

        if self.client_map.lock().await.contains_key(&session_id) {
            return Err(anyhow!("repeated connection"));
        }

        if is_tcp {
            let client = tcp_connect(
                addr,
                session_id,
                is_compressed,
                encryption_method.clone(),
                encryption_key.clone(),
                self.on_output_callback.clone(),
                self.client_map.clone(),
            )
            .await?;

            self.client_map.lock().await.insert(
                session_id,
                ClientType::TCP(client, is_compressed, encryption_method, encryption_key),
            );
        } else {
            let (client, last_active_time) = udp_connect(
                addr,
                session_id,
                is_compressed,
                encryption_method.clone(),
                encryption_key.clone(),
                self.on_output_callback.clone(),
                self.client_map.clone(),
            )
            .await?;

            self.client_map.lock().await.insert(
                session_id,
                ClientType::UDP(client, last_active_time, is_compressed, encryption_method, encryption_key),
            );
        }

        Ok(())
    }

    async fn on_i2o_send_data(&self, session_id: u32, mut data: Vec<u8>) -> anyhow::Result<()> {
        if let Some(client_type) = self.client_map.lock().await.get(&session_id) {
            match client_type {
                ClientType::TCP(writer, is_compressed, encryption_method, encryption_key) => {
                    data = decode_data(
                        data,
                        is_compressed.clone(),
                        encryption_method,
                        encryption_key,
                    )?;
                    writer.lock().await.write_all(&data).await?;
                }
                ClientType::UDP(socket, last_active_time, is_compressed, encryption_method, encryption_key) => {
                    data = decode_data(
                        data,
                        is_compressed.clone(),
                        encryption_method,
                        encryption_key,
                    )?;
                    socket.send(&data).await?;

                    if last_active_time.read().await.elapsed() >= Duration::from_secs(1) {
                        let mut instant_write = last_active_time.write().await;
                        *instant_write = Instant::now();
                    }
                }
            }
        }
        Ok(())
    }

    async fn on_i2o_disconnect(&self, session_id: u32) -> anyhow::Result<()> {
        info!("disconnect session: {session_id}");

        if let Some(client_type) = self.client_map.lock().await.remove(&session_id) {
            if let ClientType::TCP(writer, ..) = client_type {
                writer.into_inner().shutdown().await?;
            }
        }
        Ok(())
    }

    pub fn description(&self) -> &String {
        &self.description
    }
}

async fn tcp_connect(
    addr: String,
    session_id: u32,
    is_compressed: bool,
    encryption_method: EncryptionMethod,
    encryption_key: Vec<u8>,
    on_output_callback: OutputFuncType,
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
) -> anyhow::Result<TcpWriter> {
    let stream = TcpStream::connect(&addr).await?;
    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;
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
                        let data = buffer.split().to_vec();
                        match encode_data(data, is_compressed, &encryption_method, &encryption_key)
                        {
                            Ok(data) => {
                                on_output_callback(ProxyMessage::O2iRecvData(session_id, data))
                                    .await;
                            }
                            Err(err) => {
                                error!("Data encryption error: {}", err);
                                // socket读错误,主动断开
                                break;
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read from socket[{}]: {}", addr, err);
                    break;
                }
            }
        }
        on_output_callback(ProxyMessage::O2iDisconnect(session_id)).await;
        client_map.lock().await.remove(&session_id);
    });

    Ok(Mutex::new(writer))
}

async fn udp_connect(
    addr: String,
    session_id: u32,
    is_compressed: bool,
    encryption_method: EncryptionMethod,
    encryption_key: Vec<u8>,
    on_output_callback: OutputFuncType,
    client_map: Arc<Mutex<HashMap<u32, ClientType>>>,
) -> anyhow::Result<(Arc<UdpSocket>, Arc<RwLock<Instant>>)> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    socket.connect(addr).await?;

    let last_active_time = Arc::new(RwLock::new(Instant::now()));
    let last_active_time_cloned = last_active_time.clone();

    let socket_cloned = socket.clone();
    tokio::spawn(async move {
        let last_active_time_1 = last_active_time.clone();
        let recv_task = async {
            let write_timeout = Duration::from_secs(1);
            let mut buffer = vec![0u8; 65535];
            loop {
                match socket.recv_buf(&mut buffer).await {
                    Ok(size) => {
                        let received_data = &buffer[..size];
                        if size == 0 {
                            break;
                        }

                        if last_active_time_1.read().await.elapsed() >= write_timeout {
                            let mut instant_write = last_active_time_1.write().await;
                            *instant_write = Instant::now();
                        }

                        match encode_data(
                            received_data.to_vec(),
                            is_compressed,
                            &encryption_method,
                            &encryption_key,
                        ) {
                            Ok(data) => {
                                on_output_callback(ProxyMessage::O2iRecvData(session_id, data))
                                    .await;
                            }
                            Err(err) => {
                                error!("Data encryption error: {}", err);
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Udp recv error: {}", err);
                        break;
                    }
                }
            }
        };

        let timeout_task = async {
            let timeout = Duration::from_secs(10);
            loop {
                sleep(Duration::from_secs(1)).await;
                if last_active_time.read().await.elapsed() > timeout {
                    break;
                }
            }
        };

        select! {
            _=recv_task => {},
            _=timeout_task => {},
        }

        on_output_callback(ProxyMessage::O2iDisconnect(session_id)).await;
        client_map.lock().await.remove(&session_id);
    });

    Ok((socket_cloned, last_active_time_cloned))
}

fn decode_data(
    mut data: Vec<u8>,
    is_compressed: bool,
    encryption_method: &EncryptionMethod,
    encryption_key: &Vec<u8>,
) -> anyhow::Result<Vec<u8>> {
    match encryption_method {
        EncryptionMethod::None => {}
        _ => {
            data = crypto::decrypt(
                encryption_method,
                encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }
    }
    if is_compressed {
        data = crypto::decompress_data(data.as_slice())?;
    }

    Ok(data)
}

fn encode_data(
    mut data: Vec<u8>,
    is_compressed: bool,
    encryption_method: &EncryptionMethod,
    encryption_key: &Vec<u8>,
) -> anyhow::Result<Vec<u8>> {
    if is_compressed {
        data = crypto::compress_data(data.as_slice())?;
    }

    match encryption_method {
        EncryptionMethod::None => {}
        _ => {
            data = crypto::encrypt(
                encryption_method,
                encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }
    }

    Ok(data)
}
