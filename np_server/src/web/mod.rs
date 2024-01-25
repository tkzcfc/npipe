use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", name)
}

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

async fn index_manual(body: web::Bytes) -> Result<HttpResponse, Error> {
    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<MyObj>(&body)?;
    Ok(HttpResponse::Ok().json(obj)) // <- send response
}

pub async fn run_http_server(addr: &SocketAddr) -> anyhow::Result<()> {
    info!("HttpServer listening: {}", addr);
    HttpServer::new(|| {
        App::new()
            .service(greet)
            .service(web::resource("/manual").route(web::post().to(index_manual)))
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
