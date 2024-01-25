use actix_web::{get, web, App, HttpServer, Responder};
use log::info;
use std::net::SocketAddr;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", name)
}

pub async fn run_http_server(addr: &SocketAddr) -> anyhow::Result<()> {
    info!("HttpServer listening: {}", addr);
    HttpServer::new(|| App::new().service(greet))
        .bind(addr)?
        .run()
        .await?;
    Ok(())
}
