mod logger;
mod opts;
mod peer;
mod player;

use crate::peer::Peer;
use np_base::net::server;
use tokio::net::TcpStream;
use tokio::signal;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    logger::init_logger()?;

    let listener = server::bind("0.0.0.0:8118").await?;
    server::run_server(
        listener,
        || Box::new(Peer::new()),
        |stream: TcpStream| async move { Ok(stream) },
        signal::ctrl_c(),
    )
    .await;
    Ok(())
}
