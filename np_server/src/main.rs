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
use np_base::net::kcp_server;
use np_base::net::session_delegate::SessionDelegate;
use np_base::net::tcp_server;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::{select, signal};
use tokio_kcp::{KcpConfig, KcpNoDelayConfig};

async fn run_tcp_server() -> anyhow::Result<()> {
    let mut builder = tcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
        Box::new(Peer::new())
    }));

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder
        .build(GLOBAL_CONFIG.listen_addr.as_str(), signal::ctrl_c())
        .await
}

async fn run_kcp_server() -> anyhow::Result<()> {
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

    builder
        .build(GLOBAL_CONFIG.listen_addr.as_str(), signal::ctrl_c())
        .await
}

async fn run_logic_server() -> anyhow::Result<()> {
    match (
        !GLOBAL_CONFIG.kcp_listen_addr.is_empty(),
        !GLOBAL_CONFIG.listen_addr.is_empty(),
    ) {
        (true, true) => {
            select! {
                res = run_tcp_server() => {
                    if let Err(e) = res {
                        return Err(anyhow!("TCP server error: {}", e));
                    }
                },
                res = run_kcp_server() => {
                    if let Err(e) = res {
                        return Err(anyhow!("KCP server error: {}", e));
                    }
                },
            }
            Ok(())
        }
        (false, true) => run_tcp_server().await,
        (true, false) => run_kcp_server().await,
        (false, false) => Err(anyhow!("No listening address configured")),
    }
}

async fn run_web_server() -> anyhow::Result<()> {
    info!("HttpServer listening: {}", GLOBAL_CONFIG.web_addr);
    let addr = GLOBAL_CONFIG.web_addr.parse::<SocketAddr>();
    match addr {
        Ok(addr) => web::run_http_server(&addr, GLOBAL_CONFIG.web_base_dir.clone()).await,
        Err(parse_error) => Err(anyhow!(parse_error.to_string())),
    }
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
        run_logic_server().await
    } else {
        let result: anyhow::Result<()>;

        select! {
            r1 = run_logic_server() => { result = r1 },
            r2 = run_web_server() => { result = r2 },
        }

        result
    }
}
