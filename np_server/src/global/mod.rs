use crate::global::config::GLOBAL_CONFIG;
use crate::global::logger::init_logger;
use crate::global::manager::GLOBAL_MANAGER;
use crate::orm_entity::{tunnel, user};
use sea_orm::sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema, Statement,
};
use std::time::Duration;
use tokio::sync::OnceCell;

pub mod config;
pub mod logger;
pub mod manager;
pub mod opts;

pub(crate) static GLOBAL_DB_POOL: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub(crate) async fn init_global() -> anyhow::Result<()> {
    init_logger()?;

    let mut opt = ConnectOptions::new(&GLOBAL_CONFIG.database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Info);
    // .set_schema_search_path("my_schema"); // Setting default PostgreSQL schema

    // 初始化全局连接池
    GLOBAL_DB_POOL
        .get_or_init(|| async {
            Database::connect(opt)
                .await
                .expect("Database initialization failed")
        })
        .await;
    let db = GLOBAL_DB_POOL.get().unwrap();
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    match backend {
        DbBackend::MySql => {
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(user::Entity)
                    .if_not_exists()
                    .to_string(MysqlQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(tunnel::Entity)
                    .if_not_exists()
                    .to_string(MysqlQueryBuilder),
            ))
            .await?;
        }
        DbBackend::Postgres => {
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(user::Entity)
                    .if_not_exists()
                    .to_string(PostgresQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(tunnel::Entity)
                    .if_not_exists()
                    .to_string(PostgresQueryBuilder),
            ))
            .await?;
        }
        DbBackend::Sqlite => {
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(user::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(tunnel::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
        }
    }

    // 加载所有通道信息
    GLOBAL_MANAGER.tunnel_manager.load_all_tunnel().await?;

    // 加载所有的玩家信息
    GLOBAL_MANAGER.player_manager.load_all_player().await?;

    GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;

    Ok(())
}
