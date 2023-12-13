use std::net::SocketAddr;
use std::sync::Arc;
use bytes::{BytesMut};
use log::trace;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::RwLock;


#[derive(Debug)]
pub struct Connection {
    pub stream: BufWriter<TcpStream>,
    pub addr: SocketAddr,
    pub buffer: BytesMut,
}


impl Connection {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Arc<RwLock<Connection>> {
        Arc::new(
            RwLock::new(
                Connection {
                    stream: BufWriter::new(socket),
                    buffer: BytesMut::with_capacity(4 * 1024),
                    addr,
                }
            )
        )
    }

    pub async fn read(&mut self) -> crate::Result<()> {
        trace!("start read:{}", self.addr);

        Ok(())
    }

    #[inline]
    pub async fn write_all(&mut self, bytes: &[u8]) -> crate::Result<()> {
        self.stream.write_u32(bytes.len() as u32).await?;
        self.stream.write_all(bytes).await?;
        self.stream.flush().await?;
        Ok(())
    }

    #[inline]
    pub async fn disconnect(&mut self) ->crate::Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }

    #[inline]
    pub fn get_stream_ref(&mut self) -> &BufWriter<TcpStream> {
        &self.stream
    }

    #[inline]
    pub fn get_stream_mut(&mut self) -> &mut BufWriter<TcpStream> {
        &mut self.stream
    }
}