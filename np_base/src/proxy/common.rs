use crate::net::WriterMessage;
use crate::proxy::crypto::EncryptionMethod;
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::Semaphore;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

#[derive(Clone)]
pub struct FlowController {
    read_semaphore: Arc<Semaphore>,
    max_bytes: usize,
}

impl FlowController {
    fn new(max_bytes: usize) -> Self {
        Self {
            read_semaphore: Arc::new(Semaphore::new(max_bytes)),
            max_bytes,
        }
    }

    // 申请读取许可
    async fn acquire_read_permit(&self, size: usize) {
        // 等待有足够的空间可以读取
        let permit = self
            .read_semaphore
            .acquire_many(if size > self.max_bytes {
                self.max_bytes as u32
            } else {
                size as u32
            })
            .await
            .unwrap();
        permit.forget(); // 手动管理释放
    }

    // 写入成功后释放读取许可
    pub fn release_read_permit(&self, size: usize) {
        self.read_semaphore.add_permits(size);
    }
}

#[derive(Clone)]
pub struct SessionCommonInfo {
    // 是否压缩数据
    pub is_compressed: bool,
    // 加密方法
    pub encryption_method: EncryptionMethod,
    // 加密key
    pub encryption_key: Vec<u8>,
    // 信号量, 用于限制并发
    pub flow_controller: FlowController,
}

impl SessionCommonInfo {
    pub fn new(
        _is_inlet: bool,
        is_compressed: bool,
        encryption_method: EncryptionMethod,
        encryption_key: Vec<u8>,
    ) -> Self {
        Self {
            is_compressed,
            encryption_method,
            encryption_key,
            flow_controller: FlowController::new(1024 * 1024 * 1), // 默认最大1MB未处理数据
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

        self.flow_controller.acquire_read_permit(data.len()).await;

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
        Ok(addr)
    } else {
        match tokio::net::lookup_host(host).await?.next() {
            Some(addr) => Ok(addr),
            None => Err(anyhow!("No address resolved for '{}'", host)),
        }
    }
}
