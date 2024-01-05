mod config;
mod logger;
mod opts;
mod peer;
mod player;

use crate::config::GLOBAL_CONFIG;
use crate::peer::Peer;
use np_base::net::server;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use tokio::net::TcpStream;
use tokio::signal;
use tokio::sync::OnceCell;

static POOL: OnceCell<MySqlPool> = OnceCell::const_new();

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    logger::init_logger()?;

    // 初始化全局连接池
    POOL.get_or_init(|| async {
        match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(GLOBAL_CONFIG.database_url.as_str())
            .await
        {
            Ok(pool) => pool,
            Err(error) => {
                panic!("{}", error);
            }
        }
    })
    .await;

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
