use std::time::Duration;
use std::{env, io};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::sleep;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    run_client().await
}

async fn run_client() -> io::Result<()> {
    let addr = "127.0.0.1:2000".parse().unwrap();

    let socket = TcpSocket::new_v4()?;
    let mut stream = socket.connect(addr).await?;

    let mut frame: Vec<u8> = Vec::new();
    frame.extend("hello,world!".as_bytes());
    stream.write_u32(frame.len() as u32).await?;
    stream.flush().await?;
    sleep(Duration::from_secs(1)).await;

    stream.write_all(&frame[..]).await?;

    let mut frame: Vec<u8> = Vec::new();
    // frame.extend("ヾ(ToT)Bye~Bye~!".as_bytes());
    frame.extend("hello,worldd".as_bytes());
    stream.write_u32(frame.len() as u32).await?;
    stream.write_all(&frame[..]).await?;
    stream.flush().await?;

    Ok(())
}
