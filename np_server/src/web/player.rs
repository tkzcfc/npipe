use super::proto;
use super::support::{
    auth_context, bool_text, forbidden_response, player_name, player_online, record_operation,
    require_admin,
};
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::prelude::User;
use crate::orm_entity::traffic_hourly;
use actix_identity::Identity;
use actix_web::{error, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

pub(super) async fn player_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = match auth_context(identity).await {
        Ok(auth) => auth,
        Err(_) => {
            return Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 10086,
                msg: "Session expired, please log in again.".into(),
            }))
        }
    };

    let req = serde_json::from_str::<proto::PlayerListRequest>(&body)?;

    let page_number = req.page_number;
    let page_size = if req.page_size == 0 {
        20
    } else {
        req.page_size.min(100)
    };

    let db = GLOBAL_DB_POOL.get().unwrap();
    let (users, total_count) = if auth.role == "admin" {
        let paginator = User::find().paginate(db, page_size as u64);
        let users = paginator
            .fetch_page(page_number as u64)
            .await
            .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?;
        let total_count = User::find()
            .count(db)
            .await
            .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?;
        (users, total_count)
    } else if let Some(user_id) = auth.user_id {
        let users = User::find_by_id(user_id)
            .one(db)
            .await
            .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?
            .into_iter()
            .collect();
        (users, 1)
    } else {
        (vec![], 0)
    };

    let mut players: Vec<proto::PlayerListItem> = Vec::new();

    for data in users {
        let (online, ip_addr, online_time, bytes_in, bytes_out) =
            if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(data.id) {
                let player = p.read().await;
                let (rx, tx) = player.get_traffic();
                (
                    player.is_online(),
                    player.get_addr().to_string(),
                    player.get_online_time(),
                    rx as i64,
                    tx as i64,
                )
            } else {
                (false, String::new(), 0, 0, 0)
            };

        players.push(proto::PlayerListItem {
            id: data.id,
            username: data.username,
            enabled: data.enabled == 1,
            web_access: data.web_access == 1,
            online,
            ip_addr,
            online_time,
            bytes_in,
            bytes_out,
        })
    }

    Ok(HttpResponse::Ok().json(proto::PlayerListResponse {
        players,
        cur_page_number: req.page_number,
        total_count: total_count as usize,
    }))
}

pub(super) async fn remove_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerRemoveReq>(&body)?;
    let name = player_name(req.id).await;

    if let Err(err) = GLOBAL_MANAGER.player_manager.delete_player(req.id).await {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        record_operation(
            "remove_player",
            "player",
            req.id,
            &name,
            &format!("username: {}", name),
        )
        .await;
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

pub(super) async fn add_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerAddReq>(&body)?;

    return match GLOBAL_MANAGER
        .player_manager
        .add_player(&req.username, &req.password)
        .await
    {
        Ok((code, msg)) => {
            if code == 0 {
                let user_id = User::find()
                    .filter(crate::orm_entity::user::Column::Username.eq(&req.username))
                    .one(GLOBAL_DB_POOL.get().unwrap())
                    .await
                    .ok()
                    .flatten()
                    .map(|user| user.id)
                    .unwrap_or(0);
                record_operation(
                    "add_player",
                    "player",
                    user_id,
                    &req.username,
                    &format!("username: {}", req.username),
                )
                .await;
            }
            Ok(HttpResponse::Ok().json(proto::GeneralResponse { code, msg }))
        }
        Err(err) => Err(error::ErrorInternalServerError(err.to_string())),
    };
}

pub(super) async fn update_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerUpdateReq>(&body)?;
    let old_name = player_name(req.id).await;

    if let Err(err) = GLOBAL_MANAGER
        .player_manager
        .rename_player(req.id, &req.username)
        .await
    {
        return Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }));
    }
    record_operation(
        "rename_player",
        "player",
        req.id,
        &req.username,
        &format!("username: {} -> {}", old_name, req.username),
    )
    .await;

    if !req.password.is_empty() {
        if let Err(err) = GLOBAL_MANAGER
            .player_manager
            .reset_player_password(req.id, &req.password)
            .await
        {
            return Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: -1,
                msg: err.to_string(),
            }));
        }
        kick_player_session(req.id).await;
        record_operation(
            "reset_player_password",
            "player",
            req.id,
            &req.username,
            "password: changed",
        )
        .await;
    }

    Ok(HttpResponse::Ok().json(proto::GeneralResponse {
        code: 0,
        msg: "Success".into(),
    }))
}

pub(super) async fn rename_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerRenameReq>(&body)?;
    let old_name = player_name(req.id).await;
    match GLOBAL_MANAGER
        .player_manager
        .rename_player(req.id, &req.username)
        .await
    {
        Ok(()) => {
            record_operation(
                "rename_player",
                "player",
                req.id,
                &req.username,
                &format!("username: {} -> {}", old_name, req.username),
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

pub(super) async fn reset_player_password(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::PlayerResetPasswordReq>(&body)?;
    if auth.role != "admin" && auth.user_id != Some(req.id) {
        return Ok(forbidden_response());
    }
    match GLOBAL_MANAGER
        .player_manager
        .reset_player_password(req.id, &req.password)
        .await
    {
        Ok(()) => {
            kick_player_session(req.id).await;
            let name = player_name(req.id).await;
            record_operation(
                "reset_player_password",
                "player",
                req.id,
                &name,
                "password: changed",
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

pub(super) async fn update_player_status(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerStatusUpdateReq>(&body)?;
    let old_enabled = User::find_by_id(req.id)
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|user| (user.username, user.enabled));
    match GLOBAL_MANAGER
        .player_manager
        .update_player_status(req.id, req.enabled)
        .await
    {
        Ok(()) => {
            let target_name = old_enabled
                .as_ref()
                .map(|(name, _)| name.clone())
                .unwrap_or_default();
            let detail = old_enabled
                .map(|(_, enabled)| {
                    format!(
                        "enabled: {} -> {}",
                        bool_text(enabled == 1),
                        bool_text(req.enabled == 1)
                    )
                })
                .unwrap_or_else(|| format!("enabled: unknown -> {}", bool_text(req.enabled == 1)));
            record_operation(
                "update_player_status",
                "player",
                req.id,
                &target_name,
                &detail,
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

pub(super) async fn update_player_web_access(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::PlayerWebAccessUpdateReq>(&body)?;
    let old_access = User::find_by_id(req.id)
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .map(|user| (user.username, user.web_access));
    match GLOBAL_MANAGER
        .player_manager
        .update_player_web_access(req.id, req.web_access)
        .await
    {
        Ok(()) => {
            let target_name = old_access
                .as_ref()
                .map(|(name, _)| name.clone())
                .unwrap_or_default();
            let detail = old_access
                .map(|(_, web_access)| {
                    format!(
                        "web_access: {} -> {}",
                        bool_text(web_access == 1),
                        bool_text(req.web_access == 1)
                    )
                })
                .unwrap_or_else(|| {
                    format!("web_access: unknown -> {}", bool_text(req.web_access == 1))
                });
            record_operation(
                "update_player_web_access",
                "player",
                req.id,
                &target_name,
                &detail,
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

async fn kick_player_session(player_id: u32) {
    if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(player_id) {
        let mut player = p.write().await;
        if player.is_online() {
            player.kick_offline();
        }
    }
}

pub(super) async fn kick_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let req = serde_json::from_str::<proto::KickPlayerReq>(&body)?;

    if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(req.id) {
        let mut player = p.write().await;
        if player.is_online() {
            player.kick_offline();
            let name = player_name(req.id).await;
            record_operation("kick_player", "player", req.id, &name, "kicked offline").await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Player kicked offline".into(),
            }))
        } else {
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: -1,
                msg: "Player is not online".into(),
            }))
        }
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -2,
            msg: "Player not found".into(),
        }))
    }
}

pub(super) async fn player_detail(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::PlayerDetailRequest>(&body)?;
    if auth.role != "admin" && auth.user_id != Some(req.id) {
        return Ok(forbidden_response());
    }
    let db = GLOBAL_DB_POOL.get().unwrap();
    let Some(user) = User::find_by_id(req.id)
        .one(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
    else {
        return Ok(HttpResponse::Ok().json(proto::PlayerDetailResponse { player: None }));
    };

    let (online, ip_addr, online_time, bytes_in, bytes_out) =
        if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(user.id) {
            let player = p.read().await;
            let (rx, tx) = player.get_traffic();
            (
                player.is_online(),
                player.get_addr().to_string(),
                player.get_online_time(),
                rx as i64,
                tx as i64,
            )
        } else {
            (false, String::new(), 0, 0, 0)
        };

    let tunnel_models = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await.clone();
    let mut tunnels = Vec::new();
    for item in tunnel_models
        .iter()
        .filter(|item| item.sender == user.id || item.receiver == user.id)
        .take(20)
    {
        let sender_online = player_online(item.sender).await;
        let receiver_online = player_online(item.receiver).await;
        let role = match (item.sender == user.id, item.receiver == user.id) {
            (true, true) => "both",
            (true, false) => "sender",
            (false, true) => "receiver",
            _ => "",
        };

        tunnels.push(proto::PlayerTunnelItem {
            id: item.id,
            source: item.source.clone(),
            endpoint: item.endpoint.clone(),
            enabled: item.enabled == 1,
            tunnel_type: item.tunnel_type,
            role: role.to_owned(),
            available: item.enabled == 1 && sender_online && receiver_online,
        });
    }

    let login_rows = login_history::Entity::find()
        .filter(login_history::Column::UserId.eq(user.id))
        .order_by_desc(login_history::Column::Id)
        .limit(5)
        .all(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;

    let recent_logins = login_rows
        .iter()
        .map(|r| proto::LoginHistoryItem {
            id: r.id,
            user_id: r.user_id,
            ip_addr: r.ip_addr.clone(),
            login_time: r.login_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            logout_time: r
                .logout_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
            duration_secs: r.duration_secs.unwrap_or(0),
        })
        .collect();

    let traffic_rows = traffic_hourly::Entity::find()
        .filter(traffic_hourly::Column::UserId.eq(user.id))
        .order_by_desc(traffic_hourly::Column::Hour)
        .limit(24)
        .all(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
    let mut traffic_24h_in: i64 = traffic_rows.iter().map(|item| item.bytes_in).sum();
    let mut traffic_24h_out: i64 = traffic_rows.iter().map(|item| item.bytes_out).sum();
    traffic_24h_in += bytes_in;
    traffic_24h_out += bytes_out;

    Ok(HttpResponse::Ok().json(proto::PlayerDetailResponse {
        player: Some(proto::PlayerDetailItem {
            id: user.id,
            username: user.username,
            enabled: user.enabled == 1,
            web_access: user.web_access == 1,
            create_time: user.create_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            online,
            ip_addr,
            online_time,
            bytes_in,
            bytes_out,
            traffic_24h_in,
            traffic_24h_out,
            tunnels,
            recent_logins,
        }),
    }))
}

pub(super) async fn traffic_stats(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TrafficStatsRequest>(&body)?;
    if auth.role != "admin" && auth.user_id != Some(req.user_id) {
        return Ok(forbidden_response());
    }
    let hours = req.hours.unwrap_or(24).min(720);

    let db = GLOBAL_DB_POOL.get().unwrap();
    let rows = traffic_hourly::Entity::find()
        .filter(traffic_hourly::Column::UserId.eq(req.user_id))
        .order_by_desc(traffic_hourly::Column::Hour)
        .limit(hours as u64)
        .all(db)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?;

    let mut items: Vec<proto::TrafficHourItem> = Vec::new();

    for row in &rows {
        items.push(proto::TrafficHourItem {
            hour: row.hour.clone(),
            bytes_in: row.bytes_in,
            bytes_out: row.bytes_out,
        });
    }

    if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(req.user_id) {
        let player = p.read().await;
        let (rx, tx) = player.get_traffic();
        if rx > 0 || tx > 0 {
            let current_hour = Utc::now().format("%Y-%m-%d %H").to_string();
            if let Some(item) = items.iter_mut().find(|item| item.hour == current_hour) {
                item.bytes_in += rx as i64;
                item.bytes_out += tx as i64;
            } else {
                items.insert(
                    0,
                    proto::TrafficHourItem {
                        hour: current_hour,
                        bytes_in: rx as i64,
                        bytes_out: tx as i64,
                    },
                );
                items.truncate(hours as usize);
            }
        }
    }

    let total_in = items.iter().map(|item| item.bytes_in).sum();
    let total_out = items.iter().map(|item| item.bytes_out).sum();

    Ok(HttpResponse::Ok().json(proto::TrafficStatsResponse {
        items,
        total_in,
        total_out,
    }))
}
