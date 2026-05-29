use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::GLOBAL_MANAGER;
use crate::orm_entity::{
    login_history, operation_log, schema_version, traffic_hourly, tunnel, user,
};
use chrono::Utc;
use sea_orm::sea_query::{Index, MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    DbBackend, EntityTrait, QueryFilter, Schema, Statement,
};
use std::time::Duration;
use tokio::sync::OnceCell;

const CURRENT_SCHEMA_VERSION: i32 = 1;

pub(crate) static GLOBAL_DB_POOL: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub(crate) async fn init_database() -> anyhow::Result<()> {
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
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(traffic_hourly::Entity)
                    .if_not_exists()
                    .to_string(MysqlQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(login_history::Entity)
                    .if_not_exists()
                    .to_string(MysqlQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(operation_log::Entity)
                    .if_not_exists()
                    .to_string(MysqlQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(schema_version::Entity)
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
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(traffic_hourly::Entity)
                    .if_not_exists()
                    .to_string(PostgresQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(login_history::Entity)
                    .if_not_exists()
                    .to_string(PostgresQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(operation_log::Entity)
                    .if_not_exists()
                    .to_string(PostgresQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(schema_version::Entity)
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
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(traffic_hourly::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(login_history::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(operation_log::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                schema
                    .create_table_from_entity(schema_version::Entity)
                    .if_not_exists()
                    .to_string(SqliteQueryBuilder),
            ))
            .await?;
        }
    }

    run_schema_migrations(db, backend).await?;

    Ok(())
}

async fn run_schema_migrations(db: &DatabaseConnection, backend: DbBackend) -> anyhow::Result<()> {
    let version = current_schema_version(db).await?;
    if version > CURRENT_SCHEMA_VERSION {
        anyhow::bail!(
            "database schema version {} is newer than server supported version {}",
            version,
            CURRENT_SCHEMA_VERSION
        );
    }

    if version < 1 {
        ensure_indexes(db, backend).await?;
        ensure_user_columns(db, backend).await?;
        set_schema_version(db, CURRENT_SCHEMA_VERSION).await?;
    }

    Ok(())
}

async fn current_schema_version(db: &DatabaseConnection) -> anyhow::Result<i32> {
    let version = schema_version::Entity::find_by_id(1u32)
        .one(db)
        .await?
        .map(|row| row.version)
        .unwrap_or(0);
    Ok(version)
}

async fn set_schema_version(db: &DatabaseConnection, version: i32) -> anyhow::Result<()> {
    let now = Utc::now().naive_utc();
    if let Some(model) = schema_version::Entity::find_by_id(1u32).one(db).await? {
        let mut active: schema_version::ActiveModel = model.into();
        active.version = Set(version);
        active.updated_at = Set(now);
        active.update(db).await?;
    } else {
        let model = schema_version::ActiveModel {
            id: Set(1),
            version: Set(version),
            updated_at: Set(now),
        };
        model.insert(db).await?;
    }

    Ok(())
}

async fn ensure_indexes(db: &DatabaseConnection, backend: DbBackend) -> anyhow::Result<()> {
    let traffic_index = Index::create()
        .name("idx_traffic_hourly_user_id_hour")
        .table(traffic_hourly::Entity)
        .col(traffic_hourly::Column::UserId)
        .col(traffic_hourly::Column::Hour)
        .if_not_exists()
        .to_owned();

    let operation_index = Index::create()
        .name("idx_operation_log_created_at")
        .table(operation_log::Entity)
        .col(operation_log::Column::CreatedAt)
        .if_not_exists()
        .to_owned();

    for index in [traffic_index, operation_index] {
        let sql = match backend {
            DbBackend::MySql => index.to_string(MysqlQueryBuilder),
            DbBackend::Postgres => index.to_string(PostgresQueryBuilder),
            DbBackend::Sqlite => index.to_string(SqliteQueryBuilder),
        };
        db.execute(Statement::from_string(backend, sql)).await?;
    }
    Ok(())
}

async fn ensure_user_columns(db: &DatabaseConnection, backend: DbBackend) -> anyhow::Result<()> {
    let columns = match backend {
        DbBackend::MySql => vec![
            "ALTER TABLE user ADD COLUMN enabled TINYINT NOT NULL DEFAULT 1",
            "ALTER TABLE user ADD COLUMN web_access TINYINT NOT NULL DEFAULT 0",
        ],
        DbBackend::Postgres => vec![
            "ALTER TABLE \"user\" ADD COLUMN IF NOT EXISTS enabled SMALLINT NOT NULL DEFAULT 1",
            "ALTER TABLE \"user\" ADD COLUMN IF NOT EXISTS web_access SMALLINT NOT NULL DEFAULT 0",
        ],
        DbBackend::Sqlite => vec![
            "ALTER TABLE user ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE user ADD COLUMN web_access INTEGER NOT NULL DEFAULT 0",
        ],
    };

    for sql in columns {
        if let Err(err) = db.execute(Statement::from_string(backend, sql)).await {
            let msg = err.to_string().to_lowercase();
            if !(msg.contains("duplicate")
                || msg.contains("exists")
                || msg.contains("duplicate column"))
            {
                return Err(err.into());
            }
        }
    }

    Ok(())
}

pub(crate) fn start_traffic_flush_loop() {
    tokio::spawn(async move {
        traffic_flush_loop().await;
    });
}

/// 每 5 分钟将玩家内存中的流量计数刷入数据库
async fn traffic_flush_loop() {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;

        let hour = Utc::now().format("%Y-%m-%d %H").to_string();
        let db = GLOBAL_DB_POOL.get().expect("DB pool not initialized");

        for entry in GLOBAL_MANAGER.player_manager.player_map.iter() {
            let player_id = *entry.key();
            let player = entry.value().clone();
            let (rx, tx) = {
                let player = player.read().await;
                player.take_traffic()
            };
            if rx == 0 && tx == 0 {
                continue;
            }

            // 查找当前小时的记录
            let existing = traffic_hourly::Entity::find()
                .filter(traffic_hourly::Column::UserId.eq(player_id))
                .filter(traffic_hourly::Column::Hour.eq(&hour))
                .one(db)
                .await;

            let save_result = match existing {
                Ok(Some(model)) => {
                    // 累加
                    let mut active: traffic_hourly::ActiveModel = model.into();
                    active.bytes_in = Set(active.bytes_in.unwrap() + rx as i64);
                    active.bytes_out = Set(active.bytes_out.unwrap() + tx as i64);
                    active.update(db).await.map(|_| ())
                }
                Ok(None) => {
                    // 新建
                    let new_row = traffic_hourly::ActiveModel {
                        id: NotSet,
                        user_id: Set(player_id),
                        bytes_in: Set(rx as i64),
                        bytes_out: Set(tx as i64),
                        hour: Set(hour.clone()),
                    };
                    new_row.insert(db).await.map(|_| ())
                }
                Err(e) => {
                    log::error!("traffic flush query error: {}", e);
                    Err(e)
                }
            };

            if let Err(e) = save_result {
                let player = player.read().await;
                player.add_traffic(rx, tx);
                log::error!(
                    "traffic flush save error, restored counters, user_id:{}, rx:{}, tx:{}, error:{}",
                    player_id,
                    rx,
                    tx,
                    e
                );
            }
        }
    }
}
