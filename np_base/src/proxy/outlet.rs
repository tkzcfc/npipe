use crate::proxy::crypto::{get_method, EncryptionMethod};
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use base64::prelude::*;
use bytes::BytesMut;
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Mutex;

type TcpWriter = Mutex<WriteHalf<TcpStream>>;

enum ClientType {
    TCP(TcpWriter, bool, EncryptionMethod, Vec<u8>),
    UDP(Arc<UdpSocket>, bool, EncryptionMethod, Vec<u8>),
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
            client_addr
            ) => {
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
                    error!("Failed to connect to {}, error: {}, remote client addr {}", addr, err.to_string(), client_addr);
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(
                            session_id,
                            false,
                            err.to_string(),
                        ))
                        .await
                    });
                } else {
                    info!("Successfully connected to {}, remote client addr {}", addr, client_addr);
                    tokio::spawn(async move {
                        output_callback(ProxyMessage::O2iConnect(session_id, true, "".into()))
                            .await;
                    });
                }
            }
            ProxyMessage::I2oSendData(session_id, data) => {
                self.on_i2o_send_data(session_id, data).await?;
            }
            ProxyMessage::I2oDisconnect(session_id) => {
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
            let client = udp_connect(
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
                ClientType::UDP(client, is_compressed, encryption_method, encryption_key),
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
                ClientType::UDP(socket, is_compressed, encryption_method, encryption_key) => {
                    data = decode_data(
                        data,
                        is_compressed.clone(),
                        encryption_method,
                        encryption_key,
                    )?;
                    socket.send(&data).await?;
                }
            }
        }
        Ok(())
    }

    async fn on_i2o_disconnect(&self, session_id: u32) -> anyhow::Result<()>{
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
) -> anyhow::Result<Arc<UdpSocket>> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    socket.connect(addr).await?;

    let socket_cloned = socket.clone();
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 65536];
        loop {
            match socket.recv_buf(&mut buffer).await {
                Ok(size)=>{
                    let received_data = &buffer[..size];
                    if size == 0 {
                        break;
                    }
                    match encode_data(
                        received_data.to_vec(),
                        is_compressed,
                        &encryption_method,
                        &encryption_key,
                    ) {
                        Ok(data) => {
                            on_output_callback(ProxyMessage::O2iRecvData(
                                session_id,
                                data,
                            ))
                                .await;
                        }
                        Err(err) => {
                            error!("Data encryption error: {}", err);
                            break;
                        }
                    }
                },
                Err(err)=>{
                    error!("Udp recv error: {}", err);
                    break;
                }
            }
        }
        on_output_callback(ProxyMessage::O2iDisconnect(session_id)).await;
        client_map.lock().await.remove(&session_id);
    });

    Ok(socket_cloned)
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
