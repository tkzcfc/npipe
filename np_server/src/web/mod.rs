mod proto;

use crate::global::GLOBAL_DB_POOL;
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
            .service(actix_files::Files::new("/", web_base_dir.as_str()).index_file("index.html"))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_name("auth-id".to_owned())
                    .cookie_secure(false)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::minutes(1)),
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
