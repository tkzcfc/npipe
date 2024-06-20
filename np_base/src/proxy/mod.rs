use crate::net::WriterMessage;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

pub mod inlet;
pub mod outlet;

// 输出函数类型
pub type OutputFuncType =
    Arc<dyn Fn(WriterMessage) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

// 发送通道集合
pub(crate) type SenderMap = Arc<Mutex<HashMap<u32, InputSenderType>>>;

#[cfg(test)]
mod tests {
    use crate::net::WriterMessage;
    use crate::proxy::inlet::{Inlet, InletProxyType};
    use crate::proxy::OutputFuncType;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn tes_inlet_stop() {
        // 创建一个Arc变量
        let value = Arc::new(10usize);

        // 创建一个异步回调函数
        let output: OutputFuncType = Arc::new(move |message: WriterMessage| {
            let value_cloned = value.clone();
            Box::pin(async move {
                if let WriterMessage::Send(frame, _) = message {
                    return frame.len() > *value_cloned;
                }
                return false;
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
