use super::proto;
use super::support::{record_operation, require_admin};
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::operation_log;
use crate::orm_entity::traffic_hourly;
use actix_identity::Identity;
use actix_web::{error, HttpResponse, Responder};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

pub(super) async fn database_maintenance_info(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::CleanupDatabaseRequest>(&body)?;
    let db = GLOBAL_DB_POOL.get().unwrap();

    let login_days = req.login_history_keep_days.unwrap_or(90).max(1);
    let operation_days = req.operation_log_keep_days.unwrap_or(180).max(1);
    let traffic_days = req.traffic_hourly_keep_days.unwrap_or(90).max(1);

    let login_cutoff = (Utc::now() - ChronoDuration::days(login_days as i64)).naive_utc();
    let operation_cutoff = (Utc::now() - ChronoDuration::days(operation_days as i64)).naive_utc();
    let traffic_cutoff = (Utc::now() - ChronoDuration::days(traffic_days as i64))
        .format("%Y-%m-%d %H")
        .to_string();

    let login_history_total = login_history::Entity::find()
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let login_history_cleanup = login_history::Entity::find()
        .filter(login_history::Column::LoginTime.lt(login_cutoff))
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let login_history_oldest = login_history::Entity::find()
        .order_by_asc(login_history::Column::LoginTime)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.login_time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    let login_history_newest = login_history::Entity::find()
        .order_by_desc(login_history::Column::LoginTime)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.login_time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();

    let operation_log_total = operation_log::Entity::find()
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let operation_log_cleanup = operation_log::Entity::find()
        .filter(operation_log::Column::CreatedAt.lt(operation_cutoff))
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let operation_log_oldest = operation_log::Entity::find()
        .order_by_asc(operation_log::Column::CreatedAt)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.created_at.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    let operation_log_newest = operation_log::Entity::find()
        .order_by_desc(operation_log::Column::CreatedAt)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.created_at.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();

    let traffic_hourly_total = traffic_hourly::Entity::find()
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let traffic_hourly_cleanup = traffic_hourly::Entity::find()
        .filter(traffic_hourly::Column::Hour.lt(traffic_cutoff))
        .count(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let traffic_hourly_oldest = traffic_hourly::Entity::find()
        .order_by_asc(traffic_hourly::Column::Hour)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.hour)
        .unwrap_or_default();
    let traffic_hourly_newest = traffic_hourly::Entity::find()
        .order_by_desc(traffic_hourly::Column::Hour)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|item| item.hour)
        .unwrap_or_default();

    Ok(
        HttpResponse::Ok().json(proto::DatabaseMaintenanceInfoResponse {
            login_history: proto::DatabaseMaintenanceTableInfo {
                total_count: login_history_total,
                cleanup_count: login_history_cleanup,
                oldest: login_history_oldest,
                newest: login_history_newest,
            },
            operation_log: proto::DatabaseMaintenanceTableInfo {
                total_count: operation_log_total,
                cleanup_count: operation_log_cleanup,
                oldest: operation_log_oldest,
                newest: operation_log_newest,
            },
            traffic_hourly: proto::DatabaseMaintenanceTableInfo {
                total_count: traffic_hourly_total,
                cleanup_count: traffic_hourly_cleanup,
                oldest: traffic_hourly_oldest,
                newest: traffic_hourly_newest,
            },
        }),
    )
}

pub(super) async fn cleanup_database(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::CleanupDatabaseRequest>(&body)?;
    let db = GLOBAL_DB_POOL.get().unwrap();

    let login_days = req.login_history_keep_days.unwrap_or(90).max(1);
    let operation_days = req.operation_log_keep_days.unwrap_or(180).max(1);
    let traffic_days = req.traffic_hourly_keep_days.unwrap_or(90).max(1);

    let login_cutoff = (Utc::now() - ChronoDuration::days(login_days as i64)).naive_utc();
    let operation_cutoff = (Utc::now() - ChronoDuration::days(operation_days as i64)).naive_utc();
    let traffic_cutoff = (Utc::now() - ChronoDuration::days(traffic_days as i64))
        .format("%Y-%m-%d %H")
        .to_string();

    let login_history_deleted = login_history::Entity::delete_many()
        .filter(login_history::Column::LoginTime.lt(login_cutoff))
        .exec(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .rows_affected;

    let operation_log_deleted = operation_log::Entity::delete_many()
        .filter(operation_log::Column::CreatedAt.lt(operation_cutoff))
        .exec(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .rows_affected;

    let traffic_hourly_deleted = traffic_hourly::Entity::delete_many()
        .filter(traffic_hourly::Column::Hour.lt(traffic_cutoff))
        .exec(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .rows_affected;

    let detail = format!(
        "login_history: {}; operation_log: {}; traffic_hourly: {}; keep_days: login={}, operation={}, traffic={}",
        login_history_deleted,
        operation_log_deleted,
        traffic_hourly_deleted,
        login_days,
        operation_days,
        traffic_days
    );
    record_operation("cleanup_database", "system", 0, "database", &detail).await;

    Ok(HttpResponse::Ok().json(proto::CleanupDatabaseResponse {
        login_history_deleted,
        operation_log_deleted,
        traffic_hourly_deleted,
    }))
}
