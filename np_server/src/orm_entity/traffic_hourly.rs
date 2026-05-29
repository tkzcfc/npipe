//! `SeaORM` Entity — 流量按小时统计表

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "traffic_hourly")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: u32,
    pub user_id: u32,
    pub bytes_in: i64,
    pub bytes_out: i64,
    /// 小时标识，格式 "2026-05-28 14"
    pub hour: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
