use crate::net::WriterMessage;
use crate::proxy::crypto::EncryptionMethod;
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::yield_now;

const READ_BUF_MAX_LEN: usize = 1024 * 1024 * 1;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

#[derive(Clone)]
pub struct SessionCommonInfo {
    // 是否是通道入口
    is_inlet: bool,
    // 是否压缩数据
    pub is_compressed: bool,
    // 加密方法
    pub encryption_method: EncryptionMethod,
    // 加密key
    pub encryption_key: Vec<u8>,
    // 读缓存大小
    pub read_buf_len: Arc<RwLock<usize>>,
}

impl SessionCommonInfo {
    pub fn new(
        is_inlet: bool,
        is_compressed: bool,
        encryption_method: EncryptionMethod,
        encryption_key: Vec<u8>,
    ) -> Self {
        Self {
            is_inlet,
            is_compressed,
            encryption_method,
            encryption_key,
            read_buf_len: Arc::new(RwLock::new(0)),
        }
    }

    pub fn from_method_name(
        is_inlet: bool,
        is_compressed: bool,
        encryption_method: String,
    ) -> Self {
        let encryption_method = crypto::get_method(encryption_method.as_str());
        let encryption_key = crypto::generate_key(&encryption_method);
        Self::new(is_inlet, is_compressed, encryption_method, encryption_key)
    }

    pub async fn encode_data_and_limiting(&self, mut data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        if self.is_compressed {
            data = crypto::compress_data(data.as_slice())?;
        }

        if !self.encryption_method.is_none() {
            data = crypto::encrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                data,
            )?;
        }

        let read_buf_max: usize = if self.is_inlet {
            READ_BUF_MAX_LEN
        } else {
            READ_BUF_MAX_LEN * 5
        };

        while *self.read_buf_len.read().await > read_buf_max {
            yield_now().await;
        }

        let mut read_buf_len_rw = self.read_buf_len.write().await;
        *read_buf_len_rw = *read_buf_len_rw + data.len();
        drop(read_buf_len_rw);

        Ok(data)
    }

    pub fn decode_data(&self, mut data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        if !self.encryption_method.is_none() {
            data = crypto::decrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                data,
            )?;
        }
        if self.is_compressed {
            data = crypto::decompress_data(data.as_slice())?;
        }

        Ok(data)
    }
}

pub async fn async_receive_output(
    mut output_rx: Receiver<ProxyMessage>,
    on_output_callback: OutputFuncType,
) {
    loop {
        if let Some(message) = output_rx.recv().await {
            on_output_callback(message).await;
        }
    }
}

pub async fn parse_addr(host: &String) -> anyhow::Result<SocketAddr> {
    if let Ok(addr) = host.parse::<SocketAddr>() {
        return Ok(addr);
    } else {
        for addr in tokio::net::lookup_host(host).await? {
            return Ok(addr);
        }
    }
    return Err(anyhow!("The address format is invalid: '{}'", host));
}
