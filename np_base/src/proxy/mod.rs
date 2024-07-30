use crate::net::WriterMessage;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub mod crypto;
pub mod inlet;
pub mod outlet;

pub enum ProxyMessage {
    // 向输出端请求发起连接(u32:会话id  bool:是否是TCP连接 bool:是否压缩数据 String:目标地址 String:加密方式 String:加密密码 String:客户端地址)
    I2oConnect(u32, bool, bool, String, String, String, String),
    // 连接结果(u32:会话id  bool:是否是成功 String:错误信息)
    O2iConnect(u32, bool, String),
    // 向输出端请求发送数据(u32:会话id  Vec<u8>:数据)
    I2oSendData(u32, Vec<u8>),
    // 发送结果(u32:会话id, u32:完成长度)
    O2iSendDataResult(u32, usize),
    // 输出端收到数据返回给输入端(u32:会话id  Vec<u8>:数据)
    O2iRecvData(u32, Vec<u8>),
    // 接收数据处理结果(u32:会话id, u32:完成长度)
    I2oRecvDataResult(u32, usize),
    // 断开连接
    I2oDisconnect(u32),
    // 断开连接
    O2iDisconnect(u32),
}

// 输出函数类型
pub type OutputFuncType =
    Arc<dyn Fn(ProxyMessage) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

#[cfg(test)]
mod tests {
    use crate::proxy::inlet::{Inlet, InletProxyType};
    use crate::proxy::ProxyMessage;
    use crate::proxy::{crypto, OutputFuncType};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn tes_inlet_stop() {
        // 创建一个Arc变量
        let value = Arc::new(10usize);

        // 创建一个异步回调函数
        let output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
            let value_cloned = value.clone();
            Box::pin(async move {
                if let ProxyMessage::I2oSendData(_, frame) = message {
                    if *value_cloned > frame.len() {
                        println!("ok");
                    }
                }
            })
        });

        let mut inlet = Inlet::new(output.clone(), "".into());
        inlet
            .start(
                InletProxyType::TCP,
                "0.0.0.0:4000".into(),
                "www.baidu.com:80".into(),
                false,
                "None".into(),
            )
            .await
            .unwrap();

        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        inlet
            .start(
                InletProxyType::TCP,
                "0.0.0.0:4000".into(),
                "www.baidu.com:80".into(),
                false,
                "None".into(),
            )
            .await
            .unwrap();
        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        sleep(Duration::from_secs(1)).await;
        inlet.stop().await;
        inlet.stop().await;
    }

    #[test]
    fn test_crypto() {
        let raw_str = String::from("xxtea-nostd is an implementation of the XXTEA encryption algorithm designed for no-std environments. The code uses native endianess to interpret the byte slices passed to the library as 4-byte words.");

        // XSalsa20Poly1305
        let method = crypto::get_method("XSalsa20Poly1305");
        let key = crypto::generate_key(&method);
        println!("key:{:?}", key);

        let cipher_text = crypto::encrypt(&method, key.as_slice(), raw_str.as_bytes()).unwrap();
        println!("cipher_text:{:?}", cipher_text);

        let compressed_data = crypto::compress_data(cipher_text.as_slice()).unwrap();
        println!(
            "raw len: {}, compressed_data len: {}",
            cipher_text.len(),
            compressed_data.len()
        );
        let decompressed_data = crypto::decompress_data(compressed_data.as_slice()).unwrap();

        let plain_text =
            crypto::decrypt(&method, key.as_slice(), decompressed_data.as_slice()).unwrap();
        println!(
            "plain_text len:{} data: {}",
            plain_text.len(),
            String::from_utf8_lossy(plain_text.as_slice())
        );

        // None
        let method = crypto::get_method("None");
        let key = crypto::generate_key(&method);
        println!("key:{:?}", key);

        let cipher_text = crypto::encrypt(&method, key.as_slice(), raw_str.as_bytes()).unwrap();
        println!("cipher_text:{:?}", cipher_text);

        let compressed_data = crypto::compress_data(cipher_text.as_slice()).unwrap();
        println!(
            "raw len: {}, compressed_data len: {}",
            cipher_text.len(),
            compressed_data.len()
        );
        let decompressed_data = crypto::decompress_data(compressed_data.as_slice()).unwrap();

        let plain_text =
            crypto::decrypt(&method, key.as_slice(), decompressed_data.as_slice()).unwrap();
        println!(
            "plain_text len:{} data: {}",
            plain_text.len(),
            String::from_utf8_lossy(plain_text.as_slice())
        );
    }
}
