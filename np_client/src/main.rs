mod client;

use np_proto::client_server;
use np_proto::message_map::{encode_message, MessageType};
use std::time::Duration;
use std::{env, io};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;
use tokio::time::sleep;
use tokio::try_join;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    try_join!(
        run_client(),
        run_client(),
        run_client(),
        run_client(),
        run_client(),
        run_client()
    )?;

    Ok(())
}

async fn run_client() -> io::Result<()> {
    let addr = "127.0.0.1:2000".parse().unwrap();

    let socket = TcpSocket::new_v4()?;
    let mut stream = socket.connect(addr).await?;

    let message = MessageType::ClientServerLoginReq(client_server::LoginReq {
        username: "test".into(),
        password: "pass".into(),
    });

    if let Some((id, bytes)) = encode_message(&message) {
        for _i in 0..10 {
            let len = bytes.len() as u32 + 8;
            stream.write_u32(len).await?;
            stream.write_i32(0).await?;
            stream.write_u32(id).await?;
            stream.write_all(&bytes).await?;
            stream.flush().await?;
            sleep(Duration::from_millis(1000)).await;
        }
    }

    // stream.write_all(&frame[..]).await?;
    //
    // let mut frame: Vec<u8> = Vec::new();
    // // frame.extend("ヾ(ToT)Bye~Bye~!".as_bytes());
    // frame.extend("hello,worldd".as_bytes());
    // stream.write_u32(frame.len() as u32).await?;
    // stream.write_all(&frame[..]).await?;
    // stream.flush().await?;

    Ok(())
}
