use std::io;
use std::net::SocketAddr;
use log::info;
use tokio::net::TcpListener;
use tokio::net::UdpSocket;

enum InletProxyType {
    TCP,
    UDP,
}

struct Inlet {
    proxy: InletProxyType,
}

impl Inlet {
    pub fn new(proxy: InletProxyType) -> Self {
        Self {
            proxy
        }
    }

    pub async fn start(&self, listen_addr: String) -> anyhow::Result<()> {
        let addr = listen_addr.parse::<SocketAddr>()?;
        match self.proxy {
            InletProxyType::TCP => {
                let listener = TcpListener::bind(addr).await?;
            },
            InletProxyType::UDP => {
                let udpsocket = UdpSocket::bind(addr).await?;

            }
        }


        // match addr {
        //     Ok(addr) => {
        //         info!("Start listening: {}", addr);
        //         TcpListener::bind(addr).await
        //     }
        //     Err(parse_error) => Err(std::io::Error::new(
        //         io::ErrorKind::InvalidInput,
        //         parse_error.to_string(),
        //     )),
        // }

        Ok(())
    }
}


