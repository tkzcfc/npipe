mod log;
mod peer;
mod player;

use crate::log::install_log;
use crate::peer::Peer;
use np_base::net::server;
use std::{env, io};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::signal;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    install_log(true).expect("");

    let listener = server::bind("0.0.0.0:8118").await?;
    server::run_server(
        listener,
        || Box::new(Peer::new()),
        |mut stream: TcpStream| async move {
            stream.set_nodelay(false).unwrap();
            stream.shutdown().await?;
            Ok(stream)
        },
        signal::ctrl_c(),
    )
    .await;
    Ok(())
}
