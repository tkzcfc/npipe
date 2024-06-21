use crate::net::WriterMessage;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};

enum ClientType {
    TCP(TcpClient),
    UDP(SocketAddr),
}

struct OutletClient {
    inner: ClientType,
}

struct Outlet {
    client_map: HashMap<u32, OutletClient>,
    udp_socket: Option<UdpSocket>,
}

impl Outlet {
    pub fn new() -> Self {
        Self {
            udp_socket: None,
            client_map: HashMap::new(),
        }
    }

    // pub async fn send_to(&self, session_id: u32, is_tcp: bool, message: WriterMessage) -> anyhow::Result<()> {
    //
    // }
}

struct TcpClient {
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
}

impl TcpClient {
    async fn connect(addr: SocketAddr) -> anyhow::Result<Self> {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };
        let stream = socket.connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self { reader, writer })
    }

    async fn send(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
        self.writer.write_all(&frame).await?;
        Ok(())
    }
}

//
// struct UdpClient {
//     socket: UdpSocket,
// }
//
//
// impl UdpClient {
//     async fn connect(addr: SocketAddr) -> anyhow::Result<Self> {
//         let socket = UdpSocket::bind("0.0.0.0:0").await?;
//         socket.connect(&addr).await?;
//
//         let socket_cloned = socket.clone();
//
//         Ok(Self {
//             socket
//         })
//     }
//
//     async fn send(&mut self, frame: Vec<u8>) -> anyhow::Result<()> {
//         self.socket.send(&frame).await?;
//         Ok(())
//     }
// }
