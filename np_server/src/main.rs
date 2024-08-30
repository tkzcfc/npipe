mod global;
mod orm_entity;
mod peer;
mod player;
mod utils;
mod web;

use crate::global::config::GLOBAL_CONFIG;
use crate::global::opts::GLOBAL_OPTS;
use crate::peer::Peer;
use anyhow::anyhow;
use log::info;
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::tcp_server;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use tokio::{select, signal};

pub async fn run_tcp_server() -> anyhow::Result<()> {
    tcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
        Box::new(Peer::new())
    }))
    .build(GLOBAL_CONFIG.listen_addr.as_str(), signal::ctrl_c())
    .await
}

pub async fn run_web_server() -> anyhow::Result<()> {
    info!("HttpServer listening: {}", GLOBAL_CONFIG.web_addr);
    let addr = GLOBAL_CONFIG.web_addr.parse::<SocketAddr>();
    return match addr {
        Ok(addr) => web::run_http_server(&addr, GLOBAL_CONFIG.web_base_dir.clone()).await,
        Err(parse_error) => Err(anyhow!(parse_error.to_string())),
    };
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    Lazy::force(&GLOBAL_OPTS);
    Lazy::force(&GLOBAL_CONFIG);
    global::init_global().await?;

    if GLOBAL_CONFIG.web_username.is_empty()
        || GLOBAL_CONFIG.web_password.is_empty()
        || GLOBAL_CONFIG.web_addr.is_empty()
    {
        run_tcp_server().await
    } else {
        let result: anyhow::Result<()>;

        select! {
            r1 = run_tcp_server() => { result = r1 },
            r2 = run_web_server() => { result = r2 },
        }

        result
    }
}
