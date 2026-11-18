use serde::{Deserialize, Serialize};

/// 列表查询能力版本。1 表示 player_list / tunnel_list 在分页前应用服务端搜索与筛选。
/// 响应中始终返回该常量，供前端识别能力，避免与“是否带了过滤条件”混淆。
pub const LIST_QUERY_VERSION: u32 = 1;

/// 通用回复
#[derive(Serialize, Deserialize)]
pub struct GeneralResponse {
    pub msg: String,
    pub code: i32,
}
