use crate::proxy::crypto::{get_method, EncryptionMethod};
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use base64::prelude::*;
use bytes::BytesMut;
use log::{debug, error, info, trace};
use socket2::{SockRef, TcpKeepalive};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use tokio::task::yield_now;
use tokio::time::{sleep, Instant};

const READ_BUF_MAX_LEN: usize = 1024 * 1024;

type TcpWriter = Mutex<WriteHalf<TcpStream>>;

#[derive(Clone)]
struct CommonData {
    // 是否压缩数据
    is_compressed: bool,
    // 加密方法
    encryption_method: EncryptionMethod,
    // 加密key
    encryption_key: Vec<u8>,
    // 读缓存大小
    read_buf_len: Arc<RwLock<usize>>,
}

enum ClientType {
    TCP(TcpWriter, CommonData),
    UDP(
        Arc<UdpSocket>,
        // 最后一次读/写时间点，用于判断udp超时断开
        Arc<RwLock<Instant>>,
        CommonData,
    ),
}

type ClientMapType = Arc<RwLock<HashMap<u32, ClientType>>>;

pub struct Outlet {
    client_map: ClientMapType,
    on_output_callback: OutputFuncType,
    description: String,
}

impl Outlet {
    pub fn new(on_output_callback: OutputFuncType, description: String) -> Self {
        Self {
            client_map: Arc::new(RwLock::new(HashMap::new())),
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
            ProxyMessage::I2oRecvDataResult(session_id, data_len) => {
                // trace!("I2oRecvDataResult: session_id:{session_id}, data_len:{data_len}");
                self.on_i2o_recv_data_result(session_id, data_len).await?;
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

        if self.client_map.read().await.contains_key(&session_id) {
            return Err(anyhow!("repeated connection"));
        }

        let common_data = CommonData {
            is_compressed,
            encryption_method: encryption_method.clone(),
            encryption_key: encryption_key.clone(),
            read_buf_len: Arc::new(RwLock::new(0)),
        };
        if is_tcp {
            tcp_connect(
                addr,
                session_id,
                common_data,
                self.on_output_callback.clone(),
                self.client_map.clone(),
            )
            .await
        } else {
            udp_connect(
                addr,
                session_id,
                common_data,
                self.on_output_callback.clone(),
                self.client_map.clone(),
            )
            .await
        }
    }

    async fn on_i2o_send_data(&self, session_id: u32, mut data: Vec<u8>) -> anyhow::Result<()> {
        if let Some(client_type) = self.client_map.read().await.get(&session_id) {
            let data_len = data.len();
            match client_type {
                ClientType::TCP(writer, common_data) => {
                    data = decode_data(data, common_data)?;
                    writer.lock().await.write_all(&data).await?;
                }
                ClientType::UDP(socket, last_active_time, common_data) => {
                    data = decode_data(data, common_data)?;
                    socket.send(&data).await?;

                    if last_active_time.read().await.elapsed() >= Duration::from_secs(1) {
                        let mut instant_write = last_active_time.write().await;
                        *instant_write = Instant::now();
                    }
                }
            }
            (self.on_output_callback)(ProxyMessage::O2iSendDataResult(session_id, data_len)).await;
        }
        Ok(())
    }

    async fn on_i2o_disconnect(&self, session_id: u32) -> anyhow::Result<()> {
        info!("disconnect session: {session_id}");

        if let Some(client_type) = self.client_map.write().await.remove(&session_id) {
            if let ClientType::TCP(writer, ..) = client_type {
                writer.into_inner().shutdown().await?;
            }
        }
        Ok(())
    }

    async fn on_i2o_recv_data_result(
        &self,
        session_id: u32,
        data_len: usize,
    ) -> anyhow::Result<()> {
        if let Some(client_type) = self.client_map.read().await.get(&session_id) {
            let common_data = match client_type {
                ClientType::TCP(_, common_data) => common_data,
                ClientType::UDP(_, _, common_data) => common_data,
            };

            let mut read_buf_len = common_data.read_buf_len.write().await;
            if *read_buf_len <= data_len {
                *read_buf_len = 0;
            } else {
                *read_buf_len = *read_buf_len - data_len;
            }
            drop(read_buf_len);
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
    common_data: CommonData,
    on_output_callback: OutputFuncType,
    client_map: ClientMapType,
) -> anyhow::Result<()> {
    let stream = TcpStream::connect(&addr).await?;

    // set tcp keepalive
    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;

    // split
    let (mut reader, writer) = tokio::io::split(stream);

    // clone
    let client_map_cloned = client_map.clone();
    let common_data_cloned = common_data.clone();

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
                        match encode_data(data, &common_data_cloned).await {
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
        client_map.write().await.remove(&session_id);
    });

    client_map_cloned
        .write()
        .await
        .insert(session_id, ClientType::TCP(Mutex::new(writer), common_data));

    Ok(())
}

async fn udp_connect(
    addr: String,
    session_id: u32,
    common_data: CommonData,
    on_output_callback: OutputFuncType,
    client_map: ClientMapType,
) -> anyhow::Result<()> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    socket.connect(addr).await?;

    // 最后活跃时间点
    let last_active_time = Arc::new(RwLock::new(Instant::now()));

    // clone
    let last_active_time_cloned = last_active_time.clone();
    let client_map_cloned = client_map.clone();
    let common_data_cloned = common_data.clone();

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

                        match encode_data(received_data.to_vec(), &common_data_cloned).await {
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
        client_map.write().await.remove(&session_id);
    });

    client_map_cloned.write().await.insert(
        session_id,
        ClientType::UDP(socket_cloned, last_active_time_cloned, common_data),
    );

    Ok(())
}

fn decode_data(mut data: Vec<u8>, common_data: &CommonData) -> anyhow::Result<Vec<u8>> {
    match common_data.encryption_method {
        EncryptionMethod::None => {}
        _ => {
            data = crypto::decrypt(
                &common_data.encryption_method,
                common_data.encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }
    }
    if common_data.is_compressed {
        data = crypto::decompress_data(data.as_slice())?;
    }

    Ok(data)
}

async fn encode_data(mut data: Vec<u8>, common_data: &CommonData) -> anyhow::Result<Vec<u8>> {
    if common_data.is_compressed {
        data = crypto::compress_data(data.as_slice())?;
    }

    match common_data.encryption_method {
        EncryptionMethod::None => {}
        _ => {
            data = crypto::encrypt(
                &common_data.encryption_method,
                common_data.encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }
    }

    while *common_data.read_buf_len.read().await > READ_BUF_MAX_LEN {
        yield_now().await;
    }

    let mut read_buf_len_rw = common_data.read_buf_len.write().await;
    *read_buf_len_rw = *read_buf_len_rw + data.len();
    drop(read_buf_len_rw);

    Ok(data)
}
