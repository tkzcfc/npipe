use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use log::{error, info};
use tokio::sync::RwLock;
use crate::connection::Connection;


// pub type OnNewConnectionCallbackType = Fn(Arc<RwLock<Connection>>)-< Future<Output=crate::Res>;

type CallbackConnectionType = Arc<RwLock<Connection>>;


pub struct TcpServer {

}

impl TcpServer {
    pub async fn run<F, Fut>(&self, listener: TcpListener, on_new_connection_callback: F) -> crate::Result<()>
        where F: Fn(CallbackConnectionType) -> Fut + Send + Sync + Clone + 'static,
              Fut: Future<Output = crate::Result<()>> + Send + 'static
    {
        info!("accepting inbound connections");
        loop {
            let (socket, addr) = self.accept(&listener).await?;
            info!("new connection: {}", addr);

            let callback = on_new_connection_callback.clone();
            tokio::spawn(async move {
                let connection = Connection::new(socket, addr);
                if let Err(_err) = callback(Arc::clone(&connection)).await {
                    error!("{}", addr);
                }
            });
        }
    }

    #[inline]
    async fn accept(&self, listener: &TcpListener) -> crate::Result<(TcpStream, SocketAddr)> {
        let mut backoff = 1;

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => return Ok((socket, addr)),
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

