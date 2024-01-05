use crate::global::config::GLOBAL_CONFIG;
use crate::global::logger::init_logger;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use tokio::sync::OnceCell;

pub mod config;
pub mod logger;
pub mod opts;

static GLOBAL_DB_POOL: OnceCell<MySqlPool> = OnceCell::const_new();

pub(crate) async fn init_global() -> anyhow::Result<()> {
    init_logger()?;

    // 初始化全局连接池
    GLOBAL_DB_POOL
        .get_or_init(|| async {
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

    Ok(())
}
