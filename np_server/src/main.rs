mod global;
mod peer;
mod player;
mod utils;

use crate::global::config::GLOBAL_CONFIG;
use crate::peer::Peer;
use crate::player::manager::PLAYER_MANAGER;
use np_base::net::server;
use tokio::net::TcpStream;
use tokio::signal;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    global::init_global().await?;

    PLAYER_MANAGER.write().await.load_all_player().await?;

    let listener = server::bind(GLOBAL_CONFIG.listen_addr.as_str()).await?;
    server::run_server(
        listener,
        || Box::new(Peer::new()),
        |stream: TcpStream| async move { Ok(stream) },
        signal::ctrl_c(),
    )
    .await;
    Ok(())
}
