//! `SeaORM` Entity — 登录历史记录表

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "login_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub user_id: u32,
    pub ip_addr: String,
    /// 登录时间
    pub login_time: DateTime,
    /// 登出时间（在线时为空）
    pub logout_time: Option<DateTime>,
    /// 在线时长（秒）
    pub duration_secs: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
