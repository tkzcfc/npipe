mod proto;

use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::prelude::User;
use crate::orm_entity::traffic_hourly;
use crate::orm_entity::tunnel;
use actix_cors::Cors;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key},
    error, middleware, web, App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

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
            .service(web::resource("/api/kick_player").route(web::post().to(kick_player)))
            .service(
                web::resource("/api/dashboard_overview").route(web::post().to(dashboard_overview)),
            )
            .service(web::resource("/api/traffic_stats").route(web::post().to(traffic_stats)))
            .service(web::resource("/api/login_history").route(web::post().to(login_history)))
            .service(web::resource("/api/tunnel_list").route(web::post().to(tunnel_list)))
            .service(web::resource("/api/tunnel_detail").route(web::post().to(tunnel_detail)))
            .service(web::resource("/api/remove_tunnel").route(web::post().to(remove_tunnel)))
            .service(web::resource("/api/add_tunnel").route(web::post().to(add_tunnel)))
            .service(web::resource("/api/update_tunnel").route(web::post().to(update_tunnel)))
            .service(
                web::resource("/api/update_tunnel_status")
                    .route(web::post().to(update_tunnel_status)),
            )
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

fn authentication(identity: Option<Identity>) -> Option<actix_web::Result<HttpResponse, Error>> {
    let id = match identity.map(|id| id.id()) {
        None => "anonymous".to_owned(),
        Some(Ok(id)) => id,
        Some(Err(err)) => return Some(Err(error::ErrorInternalServerError(err))),
    };

    if id == "anonymous" {
        Some(Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 10086,
            msg: "Session expired, please log in again.".into(),
        })))
    } else {
        None
    }
}

async fn test_auth(identity: Option<Identity>) -> actix_web::Result<impl Responder> {
    let id = match identity.map(|id| id.id()) {
        None => "anonymous".to_owned(),
        Some(Ok(id)) => id,
        Some(Err(err)) => return Err(error::ErrorInternalServerError(err)),
    };

    if id == "anonymous" {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 10086,
            msg: "Session expired, please log in again.".into(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: format!("hello {}", id),
        }))
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
        let content = format!(
            "{}-{}",
            GLOBAL_CONFIG.web_username, GLOBAL_CONFIG.web_password
        );
        let digest = md5::compute(content.as_bytes());
        let md5 = format!("{:x}", digest);
        Identity::login(&request.extensions(), md5)?;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("admin".into()),
        }));
    }

    Ok(HttpResponse::Ok().json(proto::LoginResponse {
        code: -2,
        msg: "Incorrect username or password".into(),
        role: None,
    }))
}

async fn player_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerListRequest>(&body)?;

    let page_number = req.page_number;
    let page_size = if req.page_size == 0 {
        20
    } else {
        req.page_size.min(100)
    };

    // 分页查询玩家信息
    let paginator = User::find().paginate(GLOBAL_DB_POOL.get().unwrap(), page_size as u64);
    let users = paginator
        .fetch_page(page_number as u64)
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?;

    // 查询玩家总条数
    let total_count = User::find()
        .count(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sqlx error:{}", err)))?;

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
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerRemoveReq>(&body)?;

    if let Err(err) = GLOBAL_MANAGER.player_manager.delete_player(req.id).await {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

async fn add_player(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerAddReq>(&body)?;

    return match GLOBAL_MANAGER
        .player_manager
        .add_player(&req.username, &req.password)
        .await
    {
        Ok((code, msg)) => Ok(HttpResponse::Ok().json(proto::GeneralResponse { code, msg })),
        Err(err) => Err(error::ErrorInternalServerError(err.to_string())),
    };
}

async fn update_player(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerUpdateReq>(&body)?;

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
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerRenameReq>(&body)?;
    match GLOBAL_MANAGER
        .player_manager
        .rename_player(req.id, &req.username)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        })),
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
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerResetPasswordReq>(&body)?;
    match GLOBAL_MANAGER
        .player_manager
        .reset_player_password(req.id, &req.password)
        .await
    {
        Ok(()) => {
            kick_player_session(req.id).await;
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
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::KickPlayerReq>(&body)?;

    if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(req.id) {
        let mut player = p.write().await;
        if player.is_online() {
            player.kick_offline();
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

async fn traffic_stats(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TrafficStatsRequest>(&body)?;
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
    if let Some(result) = authentication(identity) {
        return result;
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
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::LoginHistoryRequest>(&body)?;
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

async fn tunnel_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelListRequest>(&body)?;

    // 一次读锁同时获取分页数据和总条数，避免两次加锁
    let (tunnel_list, total_count) = GLOBAL_MANAGER
        .tunnel_manager
        .query_with_total(req.page_number, req.page_size)
        .await;

    let mut tunnels: Vec<proto::TunnelListItem> = Vec::new();

    for data in tunnel_list {
        let custom_mapping: HashMap<String, String> =
            serde_json::from_str(&data.custom_mapping).map_or(HashMap::new(), |x| x);

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
        })
    }

    Ok(HttpResponse::Ok().json(proto::TunnelListResponse {
        tunnels,
        cur_page_number: req.page_number,
        total_count,
    }))
}

async fn tunnel_detail(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelDetailRequest>(&body)?;
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
            }
        });

    Ok(HttpResponse::Ok().json(proto::TunnelDetailResponse { tunnel }))
}

async fn remove_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelRemoveReq>(&body)?;
    if let Err(err) = GLOBAL_MANAGER.tunnel_manager.delete_tunnel(req.id).await {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

async fn add_tunnel(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelAddReq>(&body)?;
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .add_tunnel(tunnel::Model {
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
            custom_mapping: serde_json::to_string(&req.custom_mapping)
                .map_or("".to_string(), |x| x),
            encryption_method: req.encryption_method,
        })
        .await
    {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

async fn update_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelUpdateReq>(&body)?;
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .update_tunnel(
            tunnel::Model {
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
                custom_mapping: serde_json::to_string(&req.custom_mapping)
                    .map_or("".to_string(), |x| x),
                encryption_method: req.encryption_method,
            },
            req.preserve_password.unwrap_or(false),
        )
        .await
    {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

async fn update_tunnel_status(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelStatusUpdateReq>(&body)?;
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
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}
