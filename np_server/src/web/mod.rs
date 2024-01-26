mod proto;

use crate::global::GLOBAL_DB_POOL;
use actix_web::{error, get, web, App, Error, HttpResponse, HttpServer, Responder};
use log::info;
use std::net::SocketAddr;

fn map_db_err(err: sqlx::Error) -> Error {
    error::ErrorInternalServerError(format!("sqlx error:{}", err.to_string()))
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", name)
}

async fn index_login(body: web::Bytes) -> anyhow::Result<HttpResponse, Error> {
    let req = serde_json::from_slice::<proto::LoginReq>(&body)?;

    struct Result {
        r#type: u8,
    }

    let result: Option<Result> = sqlx::query_as!(
        Result,
        "SELECT type FROM user WHERE username = ? AND password = ?",
        req.username,
        req.password
    )
    .fetch_optional(GLOBAL_DB_POOL.get().unwrap())
    .await
    .map_err(map_db_err)?;

    if let Some(result) = result {
        if result.r#type == 1 {
            // 生成token
            Ok(HttpResponse::Ok().json(proto::LoginAck {
                token: "".into(),
                msg: "Not an administrator account".into(),
            }))
        } else {
            // 不是管理员
            Ok(HttpResponse::Ok().json(proto::LoginAck {
                token: "".into(),
                msg: "Not an administrator account".into(),
            }))
        }
    } else {
        // 账号或密码错误
        Ok(HttpResponse::Ok().json(proto::LoginAck {
            token: "".into(),
            msg: "Incorrect username or password".into(),
        }))
    }
}

pub async fn run_http_server(addr: &SocketAddr) -> anyhow::Result<()> {
    info!("HttpServer listening: {}", addr);
    HttpServer::new(|| {
        App::new()
            .service(greet)
            .service(web::resource("/api/login").route(web::post().to(index_login)))
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
