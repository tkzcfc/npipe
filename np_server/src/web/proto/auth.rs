use serde::{Deserialize, Serialize};

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
    pub username: Option<String>,
}
