use crate::global::config::GLOBAL_CONFIG;
use crate::global::logger::init_logger;
use log::{debug, info};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
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

    let query = "SELECT id, username, password, type FROM users";
    let rows = sqlx::query(query)
        .fetch_all(GLOBAL_DB_POOL.get().unwrap())
        .await?;
    for row in rows {
        let id: u32 = row.get("id");
        let username: String = row.get("username");
        let password: String = row.get("password");

        debug!("id: {}, name: {}, password: {}", id, username, password);
    }

    #[derive(Debug)]
    struct User {
        id: u32,
        username: String,
        password: String,
        r#type: u8,
    }
    let users: Vec<User> = sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(GLOBAL_DB_POOL.get().unwrap())
        .await?;

    for user in users {
        info!("{:?}", user);
    }

    Ok(())
}
