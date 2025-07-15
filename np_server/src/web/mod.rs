mod proto;

use crate::global::config::GLOBAL_CONFIG;
use crate::global::manager::player::PlayerDbData;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::User;
use crate::orm_entity::tunnel;
use crate::utils::str::{is_valid_password, is_valid_username};
use actix_cors::Cors;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key},
    error, middleware, web, App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use sea_orm::{EntityTrait, PaginatorTrait};
use std::collections::HashMap;

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
            .service(web::resource("/api/tunnel_list").route(web::post().to(tunnel_list)))
            .service(web::resource("/api/remove_tunnel").route(web::post().to(remove_tunnel)))
            .service(web::resource("/api/add_tunnel").route(web::post().to(add_tunnel)))
            .service(web::resource("/api/update_tunnel").route(web::post().to(update_tunnel)))
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

        // 登录成功
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    } else {
        // 账号或密码错误
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -2,
            msg: "Incorrect username or password".into(),
        }))
    }
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
    let page_size = if req.page_size == 0 || req.page_size > 100 {
        1
    } else {
        req.page_size
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
        let online = if let Some(p) = GLOBAL_MANAGER.player_manager.get_player(data.id).await {
            p.read().await.is_online()
        } else {
            false
        };

        players.push(proto::PlayerListItem {
            id: data.id,
            username: data.username,
            password: data.password,
            online,
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

    // 参数长度越界检查
    if !is_valid_username(&req.username) || !is_valid_password(&req.password) {
        return Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: "usernames may not exceed 30 characters, and passwords may not exceed 15 characters.".into(),
        }));
    }

    match GLOBAL_MANAGER
        .player_manager
        .update_player(PlayerDbData {
            username: req.username,
            password: req.password,
            id: req.id,
        })
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        })),
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -2,
            msg: err.to_string(),
        })),
    }
}

async fn tunnel_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerListRequest>(&body)?;
    let tunnel_list = GLOBAL_MANAGER
        .tunnel_manager
        .query(req.page_number, req.page_size)
        .await;

    // 查询总条数
    let total_count = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await.len();

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
            password: data.password,
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
        .update_tunnel(tunnel::Model {
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
