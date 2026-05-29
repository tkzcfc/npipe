use serde::{Deserialize, Serialize};

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
