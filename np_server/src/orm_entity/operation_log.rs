//! `SeaORM` Entity — 后台操作日志表

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "operation_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: u32,
    pub target_name: String,
    pub detail: String,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
