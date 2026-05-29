use serde::{Deserialize, Serialize};

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
