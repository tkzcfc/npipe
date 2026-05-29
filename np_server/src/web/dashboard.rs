use super::proto;
use super::support::require_admin;
use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::GLOBAL_MANAGER;
use actix_identity::Identity;
use actix_web::{HttpResponse, Responder};
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

pub(super) async fn dashboard_overview(
    identity: Option<Identity>,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let total_players = GLOBAL_MANAGER.player_manager.player_map.len();
    let mut online_players = 0;
    for entry in GLOBAL_MANAGER.player_manager.player_map.iter() {
        if entry.value().read().await.is_online() {
            online_players += 1;
        }
    }

    let tunnels = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await;
    let total_tunnels = tunnels.len();
    let enabled_tunnels = tunnels.iter().filter(|tunnel| tunnel.enabled == 1).count();
    drop(tunnels);

    Ok(HttpResponse::Ok().json(proto::DashboardOverviewResponse {
        online_players,
        total_players,
        enabled_tunnels,
        total_tunnels,
        config: proto::DashboardConfigInfo {
            listen_addr: GLOBAL_CONFIG.listen_addr.clone(),
            web_addr: GLOBAL_CONFIG.web_addr.clone(),
            enable_tls: GLOBAL_CONFIG.enable_tls,
            web_enable_tls: GLOBAL_CONFIG.web_enable_tls,
            web_tls_cert: GLOBAL_CONFIG.web_tls_cert.clone(),
            web_tls_auto_self_signed: GLOBAL_CONFIG.web_tls_auto_self_signed,
            web_cookie_secure: GLOBAL_CONFIG.web_cookie_secure,
            tls_cert: GLOBAL_CONFIG.tls_cert.clone(),
            web_base_dir: GLOBAL_CONFIG.web_base_dir.clone(),
            illegal_traffic_forward: GLOBAL_CONFIG.illegal_traffic_forward.clone(),
            quiet: GLOBAL_CONFIG.quiet,
            log_dir: GLOBAL_CONFIG.log_dir.clone(),
            database: database_kind(&GLOBAL_CONFIG.database_url).to_string(),
        },
        system: collect_system_info().await,
    }))
}

async fn collect_system_info() -> proto::DashboardSystemInfo {
    let mut system = System::new_all();
    tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
    system.refresh_cpu_usage();
    system.refresh_memory();

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let memory_usage = if total_memory > 0 {
        used_memory as f32 * 100.0 / total_memory as f32
    } else {
        0.0
    };

    proto::DashboardSystemInfo {
        host_name: System::host_name().unwrap_or_default(),
        os_name: System::name().unwrap_or_default(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        uptime_secs: System::uptime(),
        cpu_usage: system.global_cpu_usage(),
        cpu_cores: system.cpus().len(),
        total_memory,
        used_memory,
        memory_usage,
    }
}

fn database_kind(database_url: &str) -> &'static str {
    if database_url.starts_with("sqlite:") {
        "SQLite"
    } else if database_url.starts_with("mysql:") {
        "MySQL"
    } else if database_url.starts_with("postgres:") || database_url.starts_with("postgresql:") {
        "PostgreSQL"
    } else {
        "Unknown"
    }
}
