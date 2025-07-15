mod global;
mod orm_entity;
mod peer;
mod player;
mod utils;
mod web;

use crate::global::config::GLOBAL_CONFIG;
use crate::global::opts::GLOBAL_OPTS;
use crate::peer::Peer;
use log::{error, info};
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::tcp_server;
use np_base::net::{kcp_server, net_type};
use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::signal;
use tokio::task::JoinSet;
use tokio_kcp::{KcpConfig, KcpNoDelayConfig};

async fn run_tcp_server(addr: &str) -> anyhow::Result<()> {
    info!("TCP Server listening: {}", addr);
    let mut builder = tcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
        Box::new(Peer::new())
    }));

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder.build(addr, signal::ctrl_c()).await
}

async fn run_kcp_server(addr: &str) -> anyhow::Result<()> {
    info!("KCP Server listening: {}", addr);
    let mut builder = kcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
        Box::new(Peer::new())
    }))
    .set_kcp_config(KcpConfig {
        mtu: 1400,
        nodelay: KcpNoDelayConfig::fastest(),
        wnd_size: (1024, 1024),
        session_expire: Some(Duration::from_secs(15)),
        flush_write: false,
        flush_acks_input: false,
        stream: true,
        allow_recv_empty_packet: false,
    });

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder.build(addr, signal::ctrl_c()).await
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    Lazy::force(&GLOBAL_OPTS);
    Lazy::force(&GLOBAL_CONFIG);
    global::init_global().await?;

    let mut set = JoinSet::new();

    if !GLOBAL_CONFIG.web_password.is_empty()
        && !GLOBAL_CONFIG.web_username.is_empty()
        && !GLOBAL_CONFIG.web_addr.is_empty()
    {
        set.spawn(async move {
            info!("HttpServer listening: {}", GLOBAL_CONFIG.web_addr);
            web::run_http_server(&GLOBAL_CONFIG.web_addr, &GLOBAL_CONFIG.web_base_dir).await
        });
    }

    net_type::parse(&GLOBAL_CONFIG.listen_addr)
        .iter()
        .for_each(|(net_type, addr)| {
            let addr = addr.clone();
            match net_type {
                net_type::NetType::Tcp => {
                    set.spawn(async move { run_tcp_server(&addr).await });
                }
                net_type::NetType::Kcp => {
                    set.spawn(async move { run_kcp_server(&addr).await });
                }
                _ => {
                    panic!("Unsupported network type: {:?}", net_type);
                }
            }
        });

    if let Some(res) = set.join_next().await {
        set.abort_all();
        if let Err(err) = res? {
            error!("The server unexpectedly shutdown, error: {}", err);
        } else {
            info!("Server gracefully shutdown");
        }
    } else {
        error!("No listening address configured");
    }

    Ok(())
}
