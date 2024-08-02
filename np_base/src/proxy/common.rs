use crate::net::WriterMessage;
use crate::proxy::crypto::EncryptionMethod;
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::yield_now;

const READ_BUF_MAX_LEN: usize = 1024 * 1024 * 1;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

#[derive(Clone)]
pub struct SessionCommonInfo {
    // 是否压缩数据
    pub is_compressed: bool,
    // 加密方法
    pub encryption_method: EncryptionMethod,
    // 加密key
    pub encryption_key: Vec<u8>,
    // 读缓存大小
    pub read_buf_len: Arc<RwLock<usize>>,
}

pub struct SessionInfo {
    pub sender: InputSenderType,
    pub common_info: SessionCommonInfo,
}

pub type SessionInfoMap = Arc<RwLock<HashMap<u32, SessionInfo>>>;

impl SessionCommonInfo {
    pub fn new(
        is_compressed: bool,
        encryption_method: EncryptionMethod,
        encryption_key: Vec<u8>,
    ) -> Self {
        Self {
            is_compressed,
            encryption_method,
            encryption_key,
            read_buf_len: Arc::new(RwLock::new(0)),
        }
    }

    pub fn from_method_name(is_compressed: bool, encryption_method: String) -> Self {
        let encryption_method = crypto::get_method(encryption_method.as_str());
        let encryption_key = crypto::generate_key(&encryption_method);
        Self::new(is_compressed, encryption_method, encryption_key)
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

        while *self.read_buf_len.read().await > READ_BUF_MAX_LEN {
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
