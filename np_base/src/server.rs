
use tokio::net::TcpListener;

#[derive(Debug)]
struct Listener {
    listener : TcpListener,
}


impl Listener {
    async fn run() -> crate::Result<()> {
        info!("accepting inbound connections");

        Ok()
    }

    async fn accept(&mut self) -> create::Result<()> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                OK(socket) => return OK(socket),
                Err(err) => {
                    if backoff > 5 {
                        return Err(err.into());
                    }
                }
            }
                  
            backoff += 1;
        }
    }
}
