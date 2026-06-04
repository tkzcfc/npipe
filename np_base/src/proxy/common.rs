use crate::net::WriterMessage;
use crate::proxy::crypto::EncryptionMethod;
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use anyhow::anyhow;
use bytes::Bytes;
use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::Semaphore;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

/// 背压流量控制器
///
/// 使用 `Semaphore` 实现端到端背压：
/// - 发送端在 encode_data_and_limiting 时消耗 permits
/// - 接收端写入完成后通过 release_read_permit 归还 permits
/// - 当积压超过 max_bytes 时，发送侧会自动阻塞，避免内存无限增长
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

    // 申请读取许可（阻塞直到有足够空间）
    async fn acquire_read_permit(&self, size: usize) {
        let permits = if size > self.max_bytes {
            self.max_bytes as u32
        } else {
            size as u32
        };
        // unwrap: 只在 Semaphore 关闭时 panic，而我们的 Semaphore 不会被关闭
        self.read_semaphore
            .acquire_many(permits)
            .await
            .unwrap()
            .forget();
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
    // 加密key (Arc 避免 Clone 时复制 Vec)
    pub encryption_key: Arc<Vec<u8>>,
    // 流量控制器
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
            encryption_key: Arc::new(encryption_key),
            flow_controller: FlowController::new(4 * 1024 * 1024), // 默认最大4MB未处理数据
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

    /// 编码数据并申请背压许可
    ///
    /// 接受 `Bytes` 避免调用方 `to_vec()` 的无谓拷贝：
    /// - 无压缩无加密：直接返回入参，真正零拷贝
    /// - 有压缩：`compress_data(&data)` 接受 `&[u8]`，省去额外 Vec 分配
    /// - 仅加密：encrypt 需要 `Vec<u8>`，此处仍需一次 `to_vec()`（不可避免）
    pub async fn encode_data_and_limiting(&self, data: Bytes) -> anyhow::Result<Bytes> {
        // 快速路径：无压缩无加密 → 零拷贝，直接返回
        if self.encryption_method.is_none() && !self.is_compressed {
            self.flow_controller.acquire_read_permit(data.len()).await;
            return Ok(data);
        }

        let encoded: Vec<u8> = if self.is_compressed {
            // compress_data 接受 &[u8]，Bytes deref 为 &[u8]，无需 to_vec()
            let compressed = crypto::compress_data(&data)?;
            if !self.encryption_method.is_none() {
                // compressed 已是 Vec，Cow::Owned 让 Xor 原地修改（零额外分配）
                crypto::encrypt(
                    &self.encryption_method,
                    self.encryption_key.as_slice(),
                    Cow::Owned(compressed),
                )?
            } else {
                compressed
            }
        } else {
            // 仅加密，无压缩：Cow::Borrowed 让 AES 直接借用 Bytes，不再需要 to_vec()
            // Xor 仍需 into_owned()（一次拷贝），不可避免
            crypto::encrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                Cow::Borrowed(&data),
            )?
        };

        self.flow_controller
            .acquire_read_permit(encoded.len())
            .await;

        Ok(Bytes::from(encoded))
    }

    /// 解码数据
    ///
    /// 接受 `Bytes`，返回解密/解压后的 `Bytes`。
    pub fn decode_data(&self, data: Bytes) -> anyhow::Result<Bytes> {
        let decoded = if !self.encryption_method.is_none() {
            // Cow::Borrowed：AES 直接借用，零拷贝；Xor 内部 into_owned() 一次拷贝
            let decrypted = crypto::decrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                Cow::Borrowed(&data),
            )?;
            if self.is_compressed {
                crypto::decompress_data(decrypted.as_slice())?
            } else {
                decrypted
            }
        } else if self.is_compressed {
            crypto::decompress_data(&data)?
        } else {
            return Ok(data); // 无加密无压缩：真正零拷贝
        };
        Ok(Bytes::from(decoded))
    }
}

/// 输出回调消费循环
///
/// 修复: 原来的 `loop { if let Some(...) = rx.recv().await {} }` 在 channel 关闭后
/// `recv()` 返回 `None`，但 loop 不会停止，导致**无限循环**（busy loop on None）。
/// 改为 `while let Some(...)` 在 channel 关闭时自动退出。
pub async fn async_receive_output(
    mut output_rx: Receiver<ProxyMessage>,
    on_output_callback: OutputFuncType,
) {
    while let Some(message) = output_rx.recv().await {
        on_output_callback(message).await;
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
