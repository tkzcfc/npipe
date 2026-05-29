mod proto;

use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::operation_log;
use crate::orm_entity::prelude::User;
use crate::orm_entity::traffic_hourly;
use crate::orm_entity::tunnel;
use crate::utils::str::{
    get_tunnel_address_port, is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address,
};
use actix_cors::Cors;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key},
    error, middleware, web, App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::collections::HashMap;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

#[derive(Clone)]
struct AuthContext {
    role: String,
    user_id: Option<u32>,
}

/// http server
pub async fn run_http_server(addr: &str, web_base_dir: &str) -> anyhow::Result<()> {
    let secret_key = Key::generate();
    let web_base_dir = web_base_dir.to_string();

    let server = HttpServer::new(move || {
        App::new()
            // 添加 Cors 中间件，并允许所有跨域请求
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .service(web::resource("/api/login").route(web::post().to(login)))
            .service(web::resource("/api/logout").route(web::post().to(logout)))
            .service(web::resource("/api/test_auth").route(web::post().to(test_auth)))
            .service(web::resource("/api/player_list").route(web::post().to(player_list)))
            .service(web::resource("/api/remove_player").route(web::post().to(remove_player)))
            .service(web::resource("/api/add_player").route(web::post().to(add_player)))
            .service(web::resource("/api/update_player").route(web::post().to(update_player)))
            .service(web::resource("/api/rename_player").route(web::post().to(rename_player)))
            .service(
                web::resource("/api/reset_player_password")
                    .route(web::post().to(reset_player_password)),
            )
            .service(
                web::resource("/api/update_player_status")
                    .route(web::post().to(update_player_status)),
            )
            .service(
                web::resource("/api/update_player_web_access")
                    .route(web::post().to(update_player_web_access)),
            )
            .service(web::resource("/api/kick_player").route(web::post().to(kick_player)))
            .service(web::resource("/api/player_detail").route(web::post().to(player_detail)))
            .service(
                web::resource("/api/dashboard_overview").route(web::post().to(dashboard_overview)),
            )
            .service(web::resource("/api/traffic_stats").route(web::post().to(traffic_stats)))
            .service(web::resource("/api/login_history").route(web::post().to(login_history)))
            .service(web::resource("/api/operation_logs").route(web::post().to(operation_logs)))
            .service(
                web::resource("/api/database_maintenance_info")
                    .route(web::post().to(database_maintenance_info)),
            )
            .service(web::resource("/api/cleanup_database").route(web::post().to(cleanup_database)))
            .service(web::resource("/api/tunnel_list").route(web::post().to(tunnel_list)))
            .service(web::resource("/api/tunnel_detail").route(web::post().to(tunnel_detail)))
            .service(web::resource("/api/remove_tunnel").route(web::post().to(remove_tunnel)))
            .service(web::resource("/api/add_tunnel").route(web::post().to(add_tunnel)))
            .service(web::resource("/api/update_tunnel").route(web::post().to(update_tunnel)))
            .service(
                web::resource("/api/update_tunnel_status")
                    .route(web::post().to(update_tunnel_status)),
            )
            .service(web::resource("/api/tunnel_diagnose").route(web::post().to(tunnel_diagnose)))
            .service(actix_files::Files::new("/", &web_base_dir).index_file("index.html"))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_name("auth-id".to_owned())
                    .cookie_secure(false)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::minutes(60)),
                    )
                    .build(),
            )
            .wrap(middleware::NormalizePath::trim())
    })
    .workers(1)
    .bind(addr)?
    .run();
    server.await?;
    Ok(())
}

async fn auth_context(identity: Option<Identity>) -> actix_web::Result<AuthContext, Error> {
    let id = match identity.map(|id| id.id()) {
        None => "anonymous".to_owned(),
        Some(Ok(id)) => id,
        Some(Err(err)) => return Err(error::ErrorInternalServerError(err)),
    };

    if id == "anonymous" {
        return Err(error::ErrorUnauthorized("Session expired"));
    }

    if id == "admin" {
        return Ok(AuthContext {
            role: "admin".to_owned(),
            user_id: None,
        });
    }

    if let Some(user_id) = id
        .strip_prefix("user:")
        .and_then(|it| it.parse::<u32>().ok())
    {
        let user = User::find_by_id(user_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await
            .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?;
        if !user.is_some_and(|user| user.enabled == 1 && user.web_access == 1) {
            return Err(error::ErrorUnauthorized("Session expired"));
        }

        return Ok(AuthContext {
            role: "user".to_owned(),
            user_id: Some(user_id),
        });
    }

    Err(error::ErrorUnauthorized("Session expired"))
}

fn forbidden_response() -> HttpResponse {
    HttpResponse::Ok().json(proto::GeneralResponse {
        code: 403,
        msg: "Forbidden".into(),
    })
}

async fn require_admin(
    identity: Option<Identity>,
) -> actix_web::Result<Result<AuthContext, HttpResponse>, Error> {
    let auth = auth_context(identity).await?;
    if auth.role != "admin" {
        return Ok(Err(forbidden_response()));
    }
    Ok(Ok(auth))
}

async fn record_operation(
    action: &str,
    target_type: &str,
    target_id: u32,
    target_name: &str,
    detail: &str,
) {
    let db = match GLOBAL_DB_POOL.get() {
        Some(db) => db,
        None => return,
    };

    let model = operation_log::ActiveModel {
        id: NotSet,
        actor: Set("admin".to_owned()),
        action: Set(action.to_owned()),
        target_type: Set(target_type.to_owned()),
        target_id: Set(target_id),
        target_name: Set(target_name.to_owned()),
        detail: Set(detail.to_owned()),
        created_at: Set(Utc::now().naive_utc()),
    };

    if let Err(err) = model.insert(db).await {
        log::error!("operation log insert error: {}", err);
    }
}

async fn player_name(player_id: u32) -> String {
    match User::find_by_id(player_id)
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
    {
        Ok(Some(user)) => user.username,
        _ => String::new(),
    }
}

async fn test_auth(identity: Option<Identity>) -> actix_web::Result<impl Responder> {
    match auth_context(identity).await {
        Ok(auth) => Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some(auth.role),
            user_id: auth.user_id,
        })),
        Err(_) => Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 10086,
            msg: "Session expired, please log in again.".into(),
            role: None,
            user_id: None,
        })),
    }
}

async fn logout(id: Identity) -> actix_web::Result<HttpResponse, Error> {
    id.logout();
    Ok(HttpResponse::Ok().json(proto::GeneralResponse {
        code: 10086,
        msg: "Session expired, please log in again.".into(),
    }))
}

async fn login(request: HttpRequest, body: String) -> actix_web::Result<HttpResponse, Error> {
    let req = serde_json::from_str::<proto::LoginReq>(&body)?;

    // 管理员登录（配置文件中的账号）
    if !GLOBAL_CONFIG.web_username.is_empty()
        && GLOBAL_CONFIG.web_username == req.username
        && GLOBAL_CONFIG.web_password == req.password
    {
        Identity::login(&request.extensions(), "admin".to_owned())?;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("admin".into()),
            user_id: None,
        }));
    }

    if let Some(user) = User::find()
        .filter(crate::orm_entity::user::Column::Username.eq(&req.username))
        .filter(crate::orm_entity::user::Column::Password.eq(&req.password))
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
    {
        if user.enabled != 1 {
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -3,
                msg: "User has been disabled".into(),
                role: None,
                user_id: None,
            }));
        }
        if user.web_access != 1 {
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -4,
                msg: "Console access has not been approved".into(),
                role: None,
                user_id: None,
            }));
        }

        Identity::login(&request.extensions(), format!("user:{}", user.id))?;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("user".into()),
            user_id: Some(user.id),
        }));
    }

    Ok(HttpResponse::Ok().json(proto::LoginResponse {
        code: -2,
        msg: "Incorrect username or password".into(),
        role: None,
        user_id: None,
    }))
}

async fn player_list(
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

async fn remove_player(
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

async fn add_player(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
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

async fn update_player(
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

async fn rename_player(
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

async fn reset_player_password(
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

async fn update_player_status(
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

async fn update_player_web_access(
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

async fn kick_player(
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

async fn player_detail(
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

async fn traffic_stats(
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

async fn dashboard_overview(identity: Option<Identity>) -> actix_web::Result<impl Responder> {
    if let Err(result) = require_admin(identity).await? {
        return Ok(result);
    }

    let total_players = GLOBAL_MANAGER.player_manager.player_map.len();
    let mut online_players = 0;
    for entry in GLOBAL_MANAGER.player_manager.player_map.iter() {
        if entry.value().read().await.is_online() {
            online_players += 1;
        }
    }

    let tunnels = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await;
    let total_tunnels = tunnels.len();
    let enabled_tunnels = tunnels.iter().filter(|tunnel| tunnel.enabled == 1).count();
    drop(tunnels);

    Ok(HttpResponse::Ok().json(proto::DashboardOverviewResponse {
        online_players,
        total_players,
        enabled_tunnels,
        total_tunnels,
        config: proto::DashboardConfigInfo {
            listen_addr: GLOBAL_CONFIG.listen_addr.clone(),
            web_addr: GLOBAL_CONFIG.web_addr.clone(),
            enable_tls: GLOBAL_CONFIG.enable_tls,
            tls_cert: GLOBAL_CONFIG.tls_cert.clone(),
            web_base_dir: GLOBAL_CONFIG.web_base_dir.clone(),
            illegal_traffic_forward: GLOBAL_CONFIG.illegal_traffic_forward.clone(),
            quiet: GLOBAL_CONFIG.quiet,
            log_dir: GLOBAL_CONFIG.log_dir.clone(),
            database: database_kind(&GLOBAL_CONFIG.database_url).to_string(),
        },
        system: collect_system_info().await,
    }))
}

async fn collect_system_info() -> proto::DashboardSystemInfo {
    let mut system = System::new_all();
    tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
    system.refresh_cpu_usage();
    system.refresh_memory();

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let memory_usage = if total_memory > 0 {
        used_memory as f32 * 100.0 / total_memory as f32
    } else {
        0.0
    };

    proto::DashboardSystemInfo {
        host_name: System::host_name().unwrap_or_default(),
        os_name: System::name().unwrap_or_default(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        uptime_secs: System::uptime(),
        cpu_usage: system.global_cpu_usage(),
        cpu_cores: system.cpus().len(),
        total_memory,
        used_memory,
        memory_usage,
    }
}

fn database_kind(database_url: &str) -> &'static str {
    if database_url.starts_with("sqlite:") {
        "SQLite"
    } else if database_url.starts_with("mysql:") {
        "MySQL"
    } else if database_url.starts_with("postgres:") || database_url.starts_with("postgresql:") {
        "PostgreSQL"
    } else {
        "Unknown"
    }
}

async fn login_history(
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
            login_time: r.login_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            logout_time: r
                .logout_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
            duration_secs: r.duration_secs.unwrap_or(0),
        })
        .collect();

    Ok(HttpResponse::Ok().json(proto::LoginHistoryResponse {
        items,
        total_count: total as usize,
    }))
}

async fn operation_logs(
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

async fn database_maintenance_info(
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

async fn cleanup_database(
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

async fn tunnel_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelListRequest>(&body)?;

    let (tunnel_list, total_count) = if auth.role == "admin" {
        GLOBAL_MANAGER
            .tunnel_manager
            .query_with_total(req.page_number, req.page_size)
            .await
    } else if let Some(user_id) = auth.user_id {
        let page_size = if req.page_size == 0 {
            20
        } else {
            req.page_size.min(100)
        };
        let tunnels: Vec<_> = GLOBAL_MANAGER
            .tunnel_manager
            .tunnels
            .read()
            .await
            .iter()
            .filter(|data| data.sender == user_id || data.receiver == user_id)
            .cloned()
            .collect();
        let total_count = tunnels.len();
        let start = req.page_number * page_size;
        let end = (start + page_size).min(total_count);
        let page = if start <= end {
            tunnels[start..end].to_vec()
        } else {
            vec![]
        };
        (page, total_count)
    } else {
        (vec![], 0)
    };

    let mut tunnels: Vec<proto::TunnelListItem> = Vec::new();

    for data in tunnel_list {
        let custom_mapping: HashMap<String, String> =
            serde_json::from_str(&data.custom_mapping).map_or(HashMap::new(), |x| x);
        let sender_online = player_online(data.sender).await;
        let receiver_online = player_online(data.receiver).await;
        let available = data.enabled == 1 && sender_online && receiver_online;

        tunnels.push(proto::TunnelListItem {
            id: data.id,
            source: data.source,
            endpoint: data.endpoint,
            enabled: data.enabled == 1,
            sender: data.sender,
            receiver: data.receiver,
            description: data.description,
            tunnel_type: data.tunnel_type,
            username: data.username,
            is_compressed: data.is_compressed == 1,
            encryption_method: data.encryption_method,
            custom_mapping,
            sender_online,
            receiver_online,
            available,
        })
    }

    Ok(HttpResponse::Ok().json(proto::TunnelListResponse {
        tunnels,
        cur_page_number: req.page_number,
        total_count,
    }))
}

async fn player_online(player_id: u32) -> bool {
    if player_id == 0 {
        return true;
    }

    if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id) {
        return player.read().await.is_online();
    }

    false
}

fn user_tunnel_allowed(auth: &AuthContext, sender: u32, receiver: u32) -> bool {
    auth.role == "admin"
        || auth
            .user_id
            .is_some_and(|user_id| sender == user_id && (receiver == 0 || receiver == user_id))
}

async fn user_can_manage_tunnel(auth: &AuthContext, tunnel_id: u32) -> bool {
    if auth.role == "admin" {
        return true;
    }

    let Some(user_id) = auth.user_id else {
        return false;
    };

    GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .any(|it| {
            it.id == tunnel_id
                && it.sender == user_id
                && (it.receiver == 0 || it.receiver == user_id)
        })
}

async fn tunnel_detail(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelDetailRequest>(&body)?;
    if auth.role != "admin" && !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|it| it.id == req.id)
        .map(|data| {
            let custom_mapping: HashMap<String, String> =
                serde_json::from_str(&data.custom_mapping).map_or(HashMap::new(), |x| x);

            proto::TunnelDetailItem {
                id: data.id,
                source: data.source.clone(),
                endpoint: data.endpoint.clone(),
                enabled: data.enabled == 1,
                sender: data.sender,
                receiver: data.receiver,
                description: data.description.clone(),
                tunnel_type: data.tunnel_type,
                password: data.password.clone(),
                username: data.username.clone(),
                is_compressed: data.is_compressed == 1,
                encryption_method: data.encryption_method.clone(),
                custom_mapping,
                sender_online: false,
                receiver_online: false,
                available: false,
            }
        });
    let tunnel = if let Some(mut tunnel) = tunnel {
        tunnel.sender_online = player_online(tunnel.sender).await;
        tunnel.receiver_online = player_online(tunnel.receiver).await;
        tunnel.available = tunnel.enabled && tunnel.sender_online && tunnel.receiver_online;
        Some(tunnel)
    } else {
        None
    };

    Ok(HttpResponse::Ok().json(proto::TunnelDetailResponse { tunnel }))
}

async fn remove_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelRemoveReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let old_tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .cloned();
    match GLOBAL_MANAGER.tunnel_manager.delete_tunnel(req.id).await {
        Ok(()) => {
            record_operation(
                "remove_tunnel",
                "tunnel",
                req.id,
                &old_tunnel
                    .as_ref()
                    .map(|item| format!("#{} {}", item.id, item.source))
                    .unwrap_or_else(|| format!("#{}", req.id)),
                &old_tunnel
                    .as_ref()
                    .map(build_tunnel_snapshot)
                    .unwrap_or_else(|| "old tunnel not found".to_owned()),
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

async fn add_tunnel(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelAddReq>(&body)?;
    if !user_tunnel_allowed(&auth, req.sender, req.receiver) {
        return Ok(forbidden_response());
    }
    let mut new_tunnel = tunnel::Model {
        source: req.source,
        endpoint: req.endpoint,
        id: 0,
        enabled: req.enabled,
        sender: req.sender,
        receiver: req.receiver,
        description: req.description,
        tunnel_type: req.tunnel_type,
        password: req.password,
        username: req.username,
        is_compressed: req.is_compressed,
        custom_mapping: serde_json::to_string(&req.custom_mapping).map_or("".to_string(), |x| x),
        encryption_method: req.encryption_method,
    };
    let source = new_tunnel.source.clone();
    match GLOBAL_MANAGER
        .tunnel_manager
        .add_tunnel(new_tunnel.clone())
        .await
    {
        Ok(tunnel_id) => {
            new_tunnel.id = tunnel_id;
            record_operation(
                "add_tunnel",
                "tunnel",
                tunnel_id,
                &format!("#{} {}", tunnel_id, source),
                &build_tunnel_snapshot(&new_tunnel),
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

fn tunnel_type_name(tunnel_type: u32) -> &'static str {
    match tunnel_type {
        0 => "TCP",
        1 => "UDP",
        2 => "SOCKS5",
        3 => "HTTP",
        _ => "Unknown",
    }
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "enabled"
    } else {
        "disabled"
    }
}

fn push_change<T: std::fmt::Display + PartialEq>(
    changes: &mut Vec<String>,
    label: &str,
    old: T,
    new: T,
) {
    if old != new {
        changes.push(format!("{}: {} -> {}", label, old, new));
    }
}

fn build_tunnel_update_detail(old: &tunnel::Model, new: &tunnel::Model) -> String {
    let mut changes = Vec::new();

    push_change(&mut changes, "source", &old.source, &new.source);
    push_change(&mut changes, "endpoint", &old.endpoint, &new.endpoint);
    push_change(
        &mut changes,
        "enabled",
        bool_text(old.enabled == 1),
        bool_text(new.enabled == 1),
    );
    push_change(&mut changes, "sender", old.sender, new.sender);
    push_change(&mut changes, "receiver", old.receiver, new.receiver);
    push_change(
        &mut changes,
        "type",
        tunnel_type_name(old.tunnel_type),
        tunnel_type_name(new.tunnel_type),
    );
    push_change(&mut changes, "username", &old.username, &new.username);
    if old.password != new.password {
        changes.push("password: changed".to_owned());
    }
    push_change(
        &mut changes,
        "compression",
        bool_text(old.is_compressed == 1),
        bool_text(new.is_compressed == 1),
    );
    push_change(
        &mut changes,
        "encryption",
        &old.encryption_method,
        &new.encryption_method,
    );
    push_change(
        &mut changes,
        "mapping",
        &old.custom_mapping,
        &new.custom_mapping,
    );
    push_change(
        &mut changes,
        "description",
        &old.description,
        &new.description,
    );

    if changes.is_empty() {
        "no changes".to_owned()
    } else {
        changes.join("; ")
    }
}

fn build_tunnel_snapshot(tunnel: &tunnel::Model) -> String {
    let mut parts = vec![
        format!("source: {}", tunnel.source),
        format!("endpoint: {}", tunnel.endpoint),
        format!("enabled: {}", bool_text(tunnel.enabled == 1)),
        format!("sender: {}", tunnel.sender),
        format!("receiver: {}", tunnel.receiver),
        format!("type: {}", tunnel_type_name(tunnel.tunnel_type)),
        format!("username: {}", tunnel.username),
        format!(
            "password: {}",
            if tunnel.password.is_empty() {
                "empty"
            } else {
                "set"
            }
        ),
        format!("compression: {}", bool_text(tunnel.is_compressed == 1)),
        format!("encryption: {}", tunnel.encryption_method),
    ];

    if !tunnel.custom_mapping.is_empty() && tunnel.custom_mapping != "{}" {
        parts.push(format!("mapping: {}", tunnel.custom_mapping));
    }
    if !tunnel.description.is_empty() {
        parts.push(format!("description: {}", tunnel.description));
    }

    parts.join("; ")
}

async fn update_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelUpdateReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await
        || !user_tunnel_allowed(&auth, req.sender, req.receiver)
    {
        return Ok(forbidden_response());
    }
    let source = req.source.clone();
    let old_tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .cloned();
    let new_tunnel = tunnel::Model {
        source: req.source,
        endpoint: req.endpoint,
        id: req.id,
        enabled: req.enabled,
        sender: req.sender,
        receiver: req.receiver,
        description: req.description,
        tunnel_type: req.tunnel_type,
        password: req.password,
        username: req.username,
        is_compressed: req.is_compressed,
        custom_mapping: serde_json::to_string(&req.custom_mapping).map_or("".to_string(), |x| x),
        encryption_method: req.encryption_method,
    };
    let mut log_tunnel = new_tunnel.clone();
    if req.preserve_password.unwrap_or(false) && log_tunnel.password.is_empty() {
        if let Some(old) = &old_tunnel {
            log_tunnel.password = old.password.clone();
        }
    }
    let detail = old_tunnel
        .as_ref()
        .map(|old| build_tunnel_update_detail(old, &log_tunnel))
        .unwrap_or_else(|| "old tunnel not found".to_owned());

    match GLOBAL_MANAGER
        .tunnel_manager
        .update_tunnel(new_tunnel, req.preserve_password.unwrap_or(false))
        .await
    {
        Ok(()) => {
            record_operation(
                "update_tunnel",
                "tunnel",
                req.id,
                &format!("#{} {}", req.id, source),
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

async fn update_tunnel_status(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelStatusUpdateReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let old_enabled = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .map(|item| item.enabled);
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .update_tunnel_status(req.id, req.enabled)
        .await
    {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        let detail = old_enabled
            .map(|old| {
                format!(
                    "enabled: {} -> {}",
                    bool_text(old == 1),
                    bool_text(req.enabled == 1)
                )
            })
            .unwrap_or_else(|| format!("enabled: unknown -> {}", bool_text(req.enabled == 1)));
        record_operation(
            "update_tunnel_status",
            "tunnel",
            req.id,
            &format!("#{}", req.id),
            &detail,
        )
        .await;
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

async fn tunnel_diagnose(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelDiagnoseRequest>(&body)?;
    if !user_tunnel_allowed(&auth, req.sender, req.receiver) {
        return Ok(forbidden_response());
    }
    if let Some(id) = req.id {
        if !user_can_manage_tunnel(&auth, id).await {
            return Ok(forbidden_response());
        }
    }
    let mut items = Vec::new();

    push_diagnose(
        &mut items,
        "source",
        is_valid_tunnel_source_address(&req.source),
        "Source address is valid",
        "Source address format error",
    );

    let needs_endpoint = matches!(req.tunnel_type, 0 | 1);
    if needs_endpoint {
        push_diagnose(
            &mut items,
            "endpoint",
            is_valid_tunnel_endpoint_address(&req.endpoint),
            "Endpoint address is valid",
            "Endpoint address format error",
        );
    } else {
        items.push(proto::TunnelDiagnoseItem {
            key: "endpoint".to_owned(),
            level: "ok".to_owned(),
            message: "Proxy tunnel does not require endpoint".to_owned(),
        });
    }

    let sender_exists = req.sender == 0 || GLOBAL_MANAGER.player_manager.contain(req.sender);
    let receiver_exists = req.receiver == 0 || GLOBAL_MANAGER.player_manager.contain(req.receiver);
    push_diagnose(
        &mut items,
        "sender",
        sender_exists,
        "Sender exists",
        "Sender player does not exist",
    );
    push_diagnose(
        &mut items,
        "receiver",
        receiver_exists,
        "Receiver exists",
        "Receiver player does not exist",
    );

    let sender_online = player_online(req.sender).await;
    let receiver_online = player_online(req.receiver).await;
    push_runtime_diagnose(&mut items, "sender_online", sender_online, req.sender);
    push_runtime_diagnose(&mut items, "receiver_online", receiver_online, req.receiver);

    let port_conflict = GLOBAL_MANAGER
        .tunnel_manager
        .has_port_conflict(
            req.receiver,
            get_tunnel_address_port(&req.source),
            req.id,
            req.tunnel_type == 1,
        )
        .await;
    push_diagnose(
        &mut items,
        "port",
        !port_conflict,
        "Listen port is available",
        "Listen port already in use",
    );

    let ok = items.iter().all(|item| item.level != "error");
    Ok(HttpResponse::Ok().json(proto::TunnelDiagnoseResponse { ok, items }))
}

fn push_diagnose(
    items: &mut Vec<proto::TunnelDiagnoseItem>,
    key: &str,
    ok: bool,
    ok_message: &str,
    error_message: &str,
) {
    items.push(proto::TunnelDiagnoseItem {
        key: key.to_owned(),
        level: if ok { "ok" } else { "error" }.to_owned(),
        message: if ok { ok_message } else { error_message }.to_owned(),
    });
}

fn push_runtime_diagnose(
    items: &mut Vec<proto::TunnelDiagnoseItem>,
    key: &str,
    online: bool,
    player_id: u32,
) {
    let (level, message) = if player_id == 0 {
        ("ok", "Server endpoint is available")
    } else if online {
        ("ok", "Player is online")
    } else {
        ("warn", "Player is offline now")
    };

    items.push(proto::TunnelDiagnoseItem {
        key: key.to_owned(),
        level: level.to_owned(),
        message: message.to_owned(),
    });
}
