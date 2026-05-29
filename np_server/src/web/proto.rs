use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 登录请求
#[derive(Serialize, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

/// 登录响应（含角色信息）
#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub msg: String,
    pub code: i32,
    pub role: Option<String>,
    pub user_id: Option<u32>,
}

/// 通用回复
#[derive(Serialize, Deserialize)]
pub struct GeneralResponse {
    pub msg: String,
    pub code: i32,
}

/// 玩家列表回复
#[derive(Serialize, Deserialize)]
pub struct PlayerListRequest {
    // 页码  从1开始
    pub page_number: usize,
    pub page_size: usize,
}

/// 玩家列表子项
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerListItem {
    pub id: u32,
    pub username: String,
    pub enabled: bool,
    pub web_access: bool,
    pub online: bool,
    pub ip_addr: String,
    pub online_time: i64,
    pub bytes_in: i64,
    pub bytes_out: i64,
}

/// 玩家列表回复
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerListResponse {
    pub players: Vec<PlayerListItem>,
    pub cur_page_number: usize,
    pub total_count: usize,
}

/// 删除玩家
#[derive(Serialize, Deserialize)]
pub struct PlayerRemoveReq {
    pub id: u32,
}

/// 添加玩家
#[derive(Serialize, Deserialize)]
pub struct PlayerAddReq {
    pub username: String,
    pub password: String,
}

/// 更新玩家
#[derive(Serialize, Deserialize)]
pub struct PlayerUpdateReq {
    pub id: u32,
    pub username: String,
    pub password: String,
}

/// 修改玩家用户名
#[derive(Serialize, Deserialize)]
pub struct PlayerRenameReq {
    pub id: u32,
    pub username: String,
}

/// 重置玩家密码
#[derive(Serialize, Deserialize)]
pub struct PlayerResetPasswordReq {
    pub id: u32,
    pub password: String,
}

/// 修改玩家启用状态
#[derive(Serialize, Deserialize)]
pub struct PlayerStatusUpdateReq {
    pub id: u32,
    pub enabled: u8,
}

/// 修改玩家后台访问权限
#[derive(Serialize, Deserialize)]
pub struct PlayerWebAccessUpdateReq {
    pub id: u32,
    pub web_access: u8,
}

/// 踢玩家下线
#[derive(Serialize, Deserialize)]
pub struct KickPlayerReq {
    pub id: u32,
}

/// 玩家详情请求
#[derive(Serialize, Deserialize)]
pub struct PlayerDetailRequest {
    pub id: u32,
}

/// 玩家关联隧道
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerTunnelItem {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: bool,
    pub tunnel_type: u32,
    pub role: String,
    pub available: bool,
}

/// 玩家详情
#[derive(Serialize, Deserialize)]
pub struct PlayerDetailItem {
    pub id: u32,
    pub username: String,
    pub enabled: bool,
    pub web_access: bool,
    pub create_time: String,
    pub online: bool,
    pub ip_addr: String,
    pub online_time: i64,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub traffic_24h_in: i64,
    pub traffic_24h_out: i64,
    pub tunnels: Vec<PlayerTunnelItem>,
    pub recent_logins: Vec<LoginHistoryItem>,
}

/// 玩家详情响应
#[derive(Serialize, Deserialize)]
pub struct PlayerDetailResponse {
    pub player: Option<PlayerDetailItem>,
}

/// 流量统计请求
#[derive(Serialize, Deserialize)]
pub struct TrafficStatsRequest {
    pub user_id: u32,
    /// 查询最近多少小时，默认 24
    pub hours: Option<u32>,
}

/// 流量统计小时数据
#[derive(Serialize, Deserialize, Clone)]
pub struct TrafficHourItem {
    pub hour: String,
    pub bytes_in: i64,
    pub bytes_out: i64,
}

/// 流量统计响应
#[derive(Serialize, Deserialize)]
pub struct TrafficStatsResponse {
    pub items: Vec<TrafficHourItem>,
    pub total_in: i64,
    pub total_out: i64,
}

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

/// 登录历史请求
#[derive(Serialize, Deserialize)]
pub struct LoginHistoryRequest {
    pub user_id: Option<u32>,
    /// 页码从 0 开始
    pub page_number: Option<usize>,
    pub page_size: Option<usize>,
}

/// 登录历史子项
#[derive(Serialize, Deserialize, Clone)]
pub struct LoginHistoryItem {
    pub id: u32,
    pub user_id: u32,
    pub ip_addr: String,
    pub login_time: String,
    pub logout_time: String,
    pub duration_secs: i32,
}

/// 登录历史响应
#[derive(Serialize, Deserialize)]
pub struct LoginHistoryResponse {
    pub items: Vec<LoginHistoryItem>,
    pub total_count: usize,
}

/// 操作日志请求
#[derive(Serialize, Deserialize)]
pub struct OperationLogRequest {
    pub page_number: Option<usize>,
    pub page_size: Option<usize>,
}

/// 操作日志子项
#[derive(Serialize, Deserialize, Clone)]
pub struct OperationLogItem {
    pub id: u32,
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: u32,
    pub target_name: String,
    pub detail: String,
    pub created_at: String,
}

/// 操作日志响应
#[derive(Serialize, Deserialize)]
pub struct OperationLogResponse {
    pub items: Vec<OperationLogItem>,
    pub total_count: usize,
}

/// 数据库清理请求
#[derive(Serialize, Deserialize)]
pub struct CleanupDatabaseRequest {
    pub login_history_keep_days: Option<u32>,
    pub operation_log_keep_days: Option<u32>,
    pub traffic_hourly_keep_days: Option<u32>,
}

/// 数据库清理响应
#[derive(Serialize, Deserialize)]
pub struct CleanupDatabaseResponse {
    pub login_history_deleted: u64,
    pub operation_log_deleted: u64,
    pub traffic_hourly_deleted: u64,
}

/// 数据库维护表信息
#[derive(Serialize, Deserialize)]
pub struct DatabaseMaintenanceTableInfo {
    pub total_count: u64,
    pub cleanup_count: u64,
    pub oldest: String,
    pub newest: String,
}

/// 数据库维护信息响应
#[derive(Serialize, Deserialize)]
pub struct DatabaseMaintenanceInfoResponse {
    pub login_history: DatabaseMaintenanceTableInfo,
    pub operation_log: DatabaseMaintenanceTableInfo,
    pub traffic_hourly: DatabaseMaintenanceTableInfo,
}

/// 玩家列表请求
#[derive(Serialize, Deserialize)]
pub struct TunnelListRequest {
    // 页码  从1开始
    pub page_number: usize,
    pub page_size: usize,
}

/// 通道列表子项
#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelListItem {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: bool,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
    pub tunnel_type: u32,
    pub username: String,
    pub is_compressed: bool,
    pub encryption_method: String,
    pub custom_mapping: HashMap<String, String>,
    pub sender_online: bool,
    pub receiver_online: bool,
    pub available: bool,
}

/// 通道详情请求
#[derive(Serialize, Deserialize)]
pub struct TunnelDetailRequest {
    pub id: u32,
}

/// 通道详情子项，包含编辑所需的敏感字段
#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelDetailItem {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: bool,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
    pub tunnel_type: u32,
    pub password: String,
    pub username: String,
    pub is_compressed: bool,
    pub encryption_method: String,
    pub custom_mapping: HashMap<String, String>,
    pub sender_online: bool,
    pub receiver_online: bool,
    pub available: bool,
}

/// 通道详情回复
#[derive(Serialize, Deserialize)]
pub struct TunnelDetailResponse {
    pub tunnel: Option<TunnelDetailItem>,
}

/// 通道列表回复
#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelListResponse {
    pub tunnels: Vec<TunnelListItem>,
    pub cur_page_number: usize,
    pub total_count: usize,
}

/// 删除通道请求
#[derive(Serialize, Deserialize)]
pub struct TunnelRemoveReq {
    pub id: u32,
}

/// 修改通道启用状态请求
#[derive(Serialize, Deserialize)]
pub struct TunnelStatusUpdateReq {
    pub id: u32,
    pub enabled: u8,
}

/// 通道诊断请求
#[derive(Serialize, Deserialize)]
pub struct TunnelDiagnoseRequest {
    pub id: Option<u32>,
    pub source: String,
    pub endpoint: String,
    pub sender: u32,
    pub receiver: u32,
    pub tunnel_type: u32,
}

/// 通道诊断子项
#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelDiagnoseItem {
    pub key: String,
    pub level: String,
    pub message: String,
}

/// 通道诊断响应
#[derive(Serialize, Deserialize)]
pub struct TunnelDiagnoseResponse {
    pub ok: bool,
    pub items: Vec<TunnelDiagnoseItem>,
}

/// 新增通道请求
#[derive(Serialize, Deserialize)]
pub struct TunnelAddReq {
    pub source: String,
    pub endpoint: String,
    pub enabled: u8,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
    pub tunnel_type: u32,
    pub password: String,
    pub username: String,
    pub is_compressed: u8,
    pub encryption_method: String,
    pub custom_mapping: HashMap<String, String>,
}

/// 修改通道请求
#[derive(Serialize, Deserialize)]
pub struct TunnelUpdateReq {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: u8,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
    pub tunnel_type: u32,
    pub password: String,
    pub username: String,
    pub is_compressed: u8,
    pub encryption_method: String,
    pub custom_mapping: HashMap<String, String>,
    pub preserve_password: Option<bool>,
}
