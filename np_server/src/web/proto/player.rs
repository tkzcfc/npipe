use super::LoginHistoryItem;
use serde::{Deserialize, Serialize};

/// 玩家列表回复
#[derive(Serialize, Deserialize)]
pub struct PlayerListRequest {
    // 页码实际从 0 开始（历史注释有误，保持兼容）
    pub page_number: usize,
    pub page_size: usize,
    /// 搜索用户名（大小写不敏感包含）；纯数字时同时精确匹配用户 ID
    #[serde(default)]
    pub q: Option<String>,
    /// 启用状态，仅允许 0 或 1
    #[serde(default)]
    pub enabled: Option<i64>,
    /// 后台访问权限，仅允许 0 或 1
    #[serde(default)]
    pub web_access: Option<i64>,
    /// 在线状态，仅允许 0 或 1（基于实时内存数据，筛选后再分页）
    #[serde(default)]
    pub online: Option<i64>,
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
    pub connection_protocol: String,
    pub online_time: i64,
    pub total_bytes_in: i64,
    pub total_bytes_out: i64,
}

/// 玩家列表回复
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerListResponse {
    pub players: Vec<PlayerListItem>,
    pub cur_page_number: usize,
    pub total_count: usize,
    /// 查询能力版本，始终为 LIST_QUERY_VERSION，与是否携带过滤条件无关
    pub query_version: u32,
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
    pub connection_protocol: String,
    pub online_time: i64,
    pub total_bytes_in: i64,
    pub total_bytes_out: i64,
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
