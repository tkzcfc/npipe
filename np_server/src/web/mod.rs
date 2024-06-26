mod proto;

use crate::global::manager::player::PlayerDbData;
use crate::global::manager::tunnel::Tunnel;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::utils::str::{is_valid_password, is_valid_username};
use actix_cors::Cors;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key},
    error, middleware, web, App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use log::info;
use std::net::SocketAddr;

/// http server
pub async fn run_http_server(addr: &SocketAddr, web_base_dir: String) -> anyhow::Result<()> {
    info!("HttpServer listening: {}", addr);

    let secret_key = Key::generate();

    HttpServer::new(move || {
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
            .service(actix_files::Files::new("/", web_base_dir.as_str()).index_file("index.html"))
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
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}

fn map_db_err(err: sqlx::Error) -> Error {
    error::ErrorInternalServerError(format!("sqlx error:{}", err.to_string()))
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

    struct Result {
        r#type: u8,
        id: u32,
    }

    let result: Option<Result> = sqlx::query_as!(
        Result,
        "SELECT type, id FROM user WHERE username = ? AND password = ?",
        req.username,
        req.password
    )
    .fetch_optional(GLOBAL_DB_POOL.get().unwrap())
    .await
    .map_err(map_db_err)?;

    if let Some(result) = result {
        if result.r#type == 1 {
            Identity::login(&request.extensions(), format!("{}", result.id))?;
            // 登录成功
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        } else {
            // 不是管理员
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: -1,
                msg: "Not an administrator account".into(),
            }))
        }
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
    let page_size = if req.page_size <= 0 || req.page_size > 100 {
        1
    } else {
        req.page_size
    };
    let offset = page_number * page_size;

    struct Data {
        id: u32,
        username: String,
        password: String,
    }

    // 分页查询玩家数据
    let data_list: Vec<Data> = sqlx::query_as!(
        Data,
        "SELECT id, username, password FROM user WHERE type = ? LIMIT ? OFFSET ?",
        0,
        page_size as u32,
        offset as u32
    )
    .fetch_all(GLOBAL_DB_POOL.get().unwrap())
    .await
    .map_err(map_db_err)?;

    // 查询玩家总条数
    let count_query = "SELECT COUNT(*) FROM user WHERE type = ?";
    let total_count: i64 = sqlx::query_scalar(count_query)
        .bind(0)
        .fetch_one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(map_db_err)?;

    let mut players: Vec<proto::PlayerListItem> = Vec::new();

    for data in data_list {
        let online = if let Some(p) = GLOBAL_MANAGER
            .player_manager
            .read()
            .await
            .get_player(data.id)
        {
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

    if let Err(err) = GLOBAL_MANAGER
        .player_manager
        .write()
        .await
        .delete_player(req.id)
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

async fn add_player(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::PlayerAddReq>(&body)?;

    return match GLOBAL_MANAGER
        .player_manager
        .write()
        .await
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
            msg: "Bad parameter".into(),
        }));
    }

    match GLOBAL_MANAGER
        .player_manager
        .read()
        .await
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
    let page_number = req.page_number;
    let page_size = if req.page_size <= 0 || req.page_size > 100 {
        1
    } else {
        req.page_size
    };
    let offset = page_number * page_size;

    struct Data {
        id: u32,
        source: String,
        endpoint: String,
        enabled: u8,
        sender: u32,
        receiver: u32,
        description: String,
    }
    // 分页查询数据
    let data_list: Vec<Data> = sqlx::query_as!(
        Data,
        "SELECT id, source, endpoint, enabled, sender, receiver, description FROM tunnel LIMIT ? OFFSET ?",
        page_size as u32,
        offset as u32
    )
    .fetch_all(GLOBAL_DB_POOL.get().unwrap())
    .await
    .map_err(map_db_err)?;

    // 查询总条数
    let count_query = "SELECT COUNT(*) FROM tunnel";
    let total_count: i64 = sqlx::query_scalar(count_query)
        .bind(0)
        .fetch_one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(map_db_err)?;

    let mut tunnels: Vec<proto::TunnelListItem> = Vec::new();

    for data in data_list {
        tunnels.push(proto::TunnelListItem {
            id: data.id,
            source: data.source,
            endpoint: data.endpoint,
            enabled: data.enabled != 0,
            sender: data.sender,
            receiver: data.receiver,
            description: data.description,
        })
    }

    Ok(HttpResponse::Ok().json(proto::TunnelListResponse {
        tunnels,
        cur_page_number: req.page_number,
        total_count: total_count as usize,
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
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .write()
        .await
        .delete_tunnel(req.id)
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

async fn add_tunnel(identity: Option<Identity>, body: String) -> actix_web::Result<impl Responder> {
    if let Some(result) = authentication(identity) {
        return result;
    }

    let req = serde_json::from_str::<proto::TunnelAddReq>(&body)?;
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .write()
        .await
        .add_tunnel(Tunnel {
            source: req.source,
            endpoint: req.endpoint,
            id: 0,
            enabled: req.enabled,
            sender: req.sender,
            receiver: req.receiver,
            description: req.description,
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
        .write()
        .await
        .update_tunnel(Tunnel {
            source: req.source,
            endpoint: req.endpoint,
            id: req.id,
            enabled: req.enabled,
            sender: req.sender,
            receiver: req.receiver,
            description: req.description,
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
