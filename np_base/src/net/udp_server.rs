use crate::net::tcp_server::CreateSessionLogicCallback;
use log::info;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::select;

pub async fn bind(addr: &str) -> io::Result<UdpSocket> {
    let addr = addr.parse::<SocketAddr>();
    match addr {
        Ok(addr) => {
            info!("UDP Server start listening: {}", addr);
            UdpSocket::bind(addr).await
        }
        Err(parse_error) => Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            parse_error.to_string(),
        )),
    }
}

pub async fn run_server(
    socket: UdpSocket,
    on_create_session_logic_callback: CreateSessionLogicCallback,
    shutdown: impl Future,
) {
    // 循环读取中...
    let mut buf = [0; 1024];
    let recv_task = async {
        loop {
            // 接收数据
            if let Ok((len, addr)) = socket.recv_from(&mut buf).await {
                println!("Received {} bytes from {}", len, addr);
            }
        }
    };

    select! {
    _= recv_task => {
        // error!("Failed to accept, {}", err);
    },
    _ = shutdown => {
        info!("UDP Server shutting down");
    }
    };
}
