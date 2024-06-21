use crate::net::WriterMessage;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

pub mod inlet;
pub mod outlet;

pub enum ProxyMessage {
    // 向输出端请求发起连接(u32:会话id  bool:是否是TCP连接 SocketAddr:目标地址)
    I2oConnect(u32, bool, SocketAddr),
    // 连接结果(u32:会话id  bool:是否是成功)
    O2iConnect(u32, bool),
    // 向输出端请求发送数据(u32:会话id  Vec<u8>:数据)
    I2oSendData(u32, Vec<u8>),
    // 输出端收到数据返回给输入端(u32:会话id  Vec<u8>:数据)
    O2iRecvData(u32, Vec<u8>),
    // 断开连接
    I2oDisconnect(u32),
    // 断开连接
    O2iDisconnect(u32),
}

// 输出函数类型
pub type OutputFuncType = Arc<
    dyn Fn(ProxyMessage) -> Pin<Box<dyn Future<Output = anyhow::Result<bool>> + Send>>
        + Send
        + Sync,
>;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

// 发送通道集合
pub(crate) type SenderMap = Arc<Mutex<HashMap<u32, InputSenderType>>>;

#[cfg(test)]
mod tests {
    use crate::proxy::inlet::{Inlet, InletProxyType};
    use crate::proxy::OutputFuncType;
    use crate::proxy::ProxyMessage;
    use anyhow::anyhow;
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
                    Ok(frame.len() > *value_cloned)
                } else {
                    Err(anyhow!("bad message"))
                }
            })
        });

        let mut inlet = Inlet::new(InletProxyType::TCP);
        inlet
            .start("0.0.0.0:4000".into(), output.clone())
            .await
            .unwrap();

        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        inlet
            .start("0.0.0.0:4000".into(), output.clone())
            .await
            .unwrap();
        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        sleep(Duration::from_secs(1)).await;
        inlet.stop().await;
        inlet.stop().await;
    }
}
