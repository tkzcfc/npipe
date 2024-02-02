use serde::{Deserialize, Serialize};

/// 登录请求
#[derive(Serialize, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
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
    pub password: String,
    pub online: bool,
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

/// 玩家列表回复
#[derive(Serialize, Deserialize)]
pub struct ChannelListRequest {
    // 页码  从1开始
    pub page_number: usize,
    pub page_size: usize,
}

/// 通道列表子项
#[derive(Serialize, Deserialize, Clone)]
pub struct ChannelListItem {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: bool,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
}

/// 通道列表回复
#[derive(Serialize, Deserialize, Clone)]
pub struct ChannelListResponse {
    pub channels: Vec<ChannelListItem>,
    pub cur_page_number: usize,
    pub total_count: usize,
}

/// 删除通道请求
#[derive(Serialize, Deserialize)]
pub struct ChannelRemoveReq {
    pub id: u32,
}

/// 新增通道请求
#[derive(Serialize, Deserialize)]
pub struct ChannelAddReq {
    pub source: String,
    pub endpoint: String,
    pub enabled: u8,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
}

/// 修改通道请求
#[derive(Serialize, Deserialize)]
pub struct ChannelUpdateReq {
    pub id: u32,
    pub source: String,
    pub endpoint: String,
    pub enabled: u8,
    pub sender: u32,
    pub receiver: u32,
    pub description: String,
}
