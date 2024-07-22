use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub password: String,
    pub username: String,
    pub is_compressed: bool,
    pub encryption_method: String,
    pub custom_mapping: HashMap<String, String>,
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
}
