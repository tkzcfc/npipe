use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use log::info;
use tokio::sync::RwLock;
use crate::connection::Connection;


// pub type OnNewConnectionCallbackType = Fn(Arc<RwLock<Connection>>)-< Future<Output=crate::Res>;

type CallbackConnectionType = Arc<RwLock<Connection>>;


pub struct TcpServer {

}

impl TcpServer {
    pub async fn run<F, Fut>(&self, listener: TcpListener, on_new_connection_callback: F) -> crate::Result<()>
        where F: Fn(CallbackConnectionType) -> Fut + Copy + Send + Sync + 'static,
              Fut: Future<Output = crate::Result<()>> + Send + 'static
    {
        info!("accepting inbound connections");
        loop {
            let (socket, addr) = self.accept(&listener).await?;
            info!("new connection: {}", addr);
            tokio::spawn(async move {
                let connection = Connection::new(socket, addr);
                if let Err(err) = on_new_connection_callback(connection).await {

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

