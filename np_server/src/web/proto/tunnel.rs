use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
