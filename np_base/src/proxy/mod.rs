mod inlet;
mod outlet;

#[cfg(test)]
mod tests {
    use crate::proxy::inlet::{Inlet, InletProxyType};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn tes_inlet_stop() {
        let mut inlet = Inlet::new(InletProxyType::TCP);
        inlet.start("0.0.0.0:4000".into()).await.unwrap();

        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        inlet.start("0.0.0.0:4000".into()).await.unwrap();
        sleep(Duration::from_secs(1)).await;

        inlet.stop().await;
        sleep(Duration::from_secs(1)).await;
        inlet.stop().await;
        inlet.stop().await;
    }
}
