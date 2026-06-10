use super::proto;
use super::support::{auth_context, require_admin};
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::operation_log;
use actix_identity::Identity;
use actix_web::{error, HttpResponse, Responder};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

pub(super) async fn login_history(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let mut req = serde_json::from_str::<proto::LoginHistoryRequest>(&body)?;
    if auth.role != "admin" {
        req.user_id = auth.user_id;
    }
    let page_size = req.page_size.unwrap_or(20).min(100);
    let page_number = req.page_number.unwrap_or(0);

    let db = GLOBAL_DB_POOL.get().unwrap();

    // 构建查询
    let mut query = login_history::Entity::find().order_by_desc(login_history::Column::Id);

    if let Some(uid) = req.user_id {
        query = query.filter(login_history::Column::UserId.eq(uid));
    }

    let total = query
        .clone()
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;

    let rows = query
        .offset((page_number * page_size) as u64)
        .limit(page_size as u64)
        .all(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;

    let items: Vec<proto::LoginHistoryItem> = rows
        .iter()
        .map(|r| proto::LoginHistoryItem {
            id: r.id,
            user_id: r.user_id,
            ip_addr: r.ip_addr.clone(),
            login_time: r.login_time.and_utc().timestamp(),
            logout_time: r.logout_time.map(|t| t.and_utc().timestamp()).unwrap_or(0),
            duration_secs: r.duration_secs.unwrap_or(0),
            login_source: r.login_source.clone(),
            success: r.success == 1,
        })
        .collect();

    Ok(HttpResponse::Ok().json(proto::LoginHistoryResponse {
        items,
        total_count: total as usize,
    }))
}

pub(super) async fn operation_logs(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::OperationLogRequest>(&body)?;
    let page_size = req.page_size.unwrap_or(20).min(100);
    let page_number = req.page_number.unwrap_or(0);
    let db = GLOBAL_DB_POOL.get().unwrap();

    let query = operation_log::Entity::find().order_by_desc(operation_log::Column::Id);
    let total = query
        .clone()
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let rows = query
        .offset((page_number * page_size) as u64)
        .limit(page_size as u64)
        .all(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;

    let items = rows
        .iter()
        .map(|item| proto::OperationLogItem {
            id: item.id,
            actor: item.actor.clone(),
            action: item.action.clone(),
            target_type: item.target_type.clone(),
            target_id: item.target_id,
            target_name: item.target_name.clone(),
            detail: item.detail.clone(),
            created_at: item.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    Ok(HttpResponse::Ok().json(proto::OperationLogResponse {
        items,
        total_count: total as usize,
    }))
}
