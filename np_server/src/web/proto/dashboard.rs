use serde::{Deserialize, Serialize};

/// 运行概览响应
#[derive(Serialize, Deserialize)]
pub struct DashboardOverviewResponse {
    pub online_players: usize,
    pub total_players: usize,
    pub enabled_tunnels: usize,
    pub total_tunnels: usize,
    pub config: DashboardConfigInfo,
    pub system: DashboardSystemInfo,
}

/// 可展示的服务器配置信息（不包含密码、密钥、数据库连接串）
#[derive(Serialize, Deserialize)]
pub struct DashboardConfigInfo {
    pub listen_addr: String,
    pub web_addr: String,
    pub enable_tls: bool,
    pub web_enable_tls: bool,
    pub web_tls_cert: String,
    pub web_tls_auto_self_signed: bool,
    pub web_cookie_secure: bool,
    pub tls_cert: String,
    pub web_base_dir: String,
    pub illegal_traffic_forward: String,
    pub quiet: bool,
    pub log_dir: String,
    pub database: String,
}

/// 服务器机器信息与资源使用率
#[derive(Serialize, Deserialize)]
pub struct DashboardSystemInfo {
    pub host_name: String,
    pub os_name: String,
    pub kernel_version: String,
    pub uptime_secs: u64,
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub total_memory: u64,
    pub used_memory: u64,
    pub memory_usage: f32,
}
