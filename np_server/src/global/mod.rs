use crate::global::database::{init_database, start_traffic_flush_loop};
use crate::global::logger::init_logger;
use crate::global::manager::player::start_transport_idle_cleanup_loop;
use crate::global::manager::GLOBAL_MANAGER;

pub mod config;
pub mod database;
pub mod forward_rule;
pub mod logger;
pub mod manager;
pub mod opts;

pub(crate) use database::GLOBAL_DB_POOL;

pub(crate) async fn init_global() -> anyhow::Result<()> {
    init_logger()?;
    init_database().await?;

    // 加载所有通道信息
    GLOBAL_MANAGER.tunnel_manager.load_all_tunnel().await?;

    // 加载所有的玩家信息
    GLOBAL_MANAGER.player_manager.load_all_player().await?;

    GLOBAL_MANAGER.proxy_manager.sync_tunnels().await;

    // 启动流量定期刷库任务
    start_traffic_flush_loop();

    // 启动转发连接空闲清理任务
    start_transport_idle_cleanup_loop();

    Ok(())
}
