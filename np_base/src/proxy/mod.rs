pub mod inlet;
pub mod outlet;
mod test;
pub mod r#type;

#[cfg(test)]
mod tests {
    use crate::net::tcp_session::WriterMessage;
    use crate::proxy::inlet::{Inlet, InletProxyType};
    use crate::proxy::r#type::OutputFuncType;
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
