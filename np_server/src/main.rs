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
use http::Uri;
use log::{error, info};
use np_base::net::session_delegate::SessionDelegate;
use once_cell::sync::Lazy;
use std::future::Future;
use std::str::FromStr;
use tokio::signal;
use tokio::task::JoinSet;

struct ServerExit {
    name: String,
    addr: String,
    result: anyhow::Result<()>,
}

fn spawn_server<F>(
    set: &mut JoinSet<ServerExit>,
    name: impl Into<String>,
    addr: impl Into<String>,
    fut: F,
) where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let name = name.into();
    let addr = addr.into();
    info!("Starting {} server on {}", name, addr);
    set.spawn(async move {
        let result = fut.await;
        ServerExit { name, addr, result }
    });
}

fn uri_to_socket_addr(uri: &Uri) -> anyhow::Result<String> {
    let host = uri
        .host()
        .ok_or_else(|| anyhow!("Invalid URI: missing host"))?;
    let port = uri
        .port_u16()
        .ok_or_else(|| anyhow!("Invalid URI: missing port"))?;
    Ok(format!("{}:{}", host, port))
}

async fn run_tcp_server(addr: String) -> anyhow::Result<()> {
    info!("TCP Server listening: {}", addr);
    let mut builder =
        np_base::net::tcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
            Box::new(Peer::new())
        }));

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder.build(addr, signal::ctrl_c()).await
}

#[cfg(feature = "kcp")]
async fn run_kcp_server(addr: String) -> anyhow::Result<()> {
    info!("KCP Server listening: {}", addr);
    let mut builder =
        np_base::net::kcp_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
            Box::new(Peer::new())
        }))
        .set_kcp_config(tokio_kcp::KcpConfig {
            mtu: 1400,
            nodelay: tokio_kcp::KcpNoDelayConfig::fastest(),
            wnd_size: (1024, 1024),
            session_expire: Some(std::time::Duration::from_secs(15)),
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

#[cfg(feature = "ws")]
async fn run_ws_server(addr: String) -> anyhow::Result<()> {
    info!("Websocket Server listening: {}", addr);
    let mut builder =
        np_base::net::ws_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
            Box::new(Peer::new())
        }));

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder.build(addr, signal::ctrl_c()).await
}

#[cfg(feature = "quic")]
async fn run_quic_server(addr: String) -> anyhow::Result<()> {
    info!("QUIC Server listening: {}", addr);
    let mut builder =
        np_base::net::quic_server::Builder::new(Box::new(|| -> Box<dyn SessionDelegate> {
            Box::new(Peer::new())
        }));

    if GLOBAL_CONFIG.enable_tls {
        builder = builder.set_tls_configuration(&GLOBAL_CONFIG.tls_cert, &GLOBAL_CONFIG.tls_key);
    }

    builder.build(&addr, signal::ctrl_c()).await
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
        let name = if GLOBAL_CONFIG.web_enable_tls {
            "HTTPS"
        } else {
            "HTTP"
        };
        spawn_server(
            &mut set,
            name,
            GLOBAL_CONFIG.web_addr.clone(),
            web::run_http_server(&GLOBAL_CONFIG.web_addr, &GLOBAL_CONFIG.web_base_dir),
        );
    }

    GLOBAL_CONFIG
        .listen_addr
        .split(",")
        .filter_map(|s| {
            let raw = s.trim();
            if raw.is_empty() {
                return None;
            }
            Uri::from_str(raw)
                .map_err(|e| {
                    error!("Failed to parse listen_addr item '{}': {}", raw, e);
                    e
                })
                .ok() // 丢弃错误，保留成功的 Uri
        })
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|request| match request.scheme_str() {
            Some("tcp") => match uri_to_socket_addr(&request) {
                Ok(addr) => spawn_server(&mut set, "TCP", addr.clone(), run_tcp_server(addr)),
                Err(err) => error!("Invalid TCP listen address '{}': {}", request, err),
            },
            #[cfg(feature = "kcp")]
            Some("kcp") => match uri_to_socket_addr(&request) {
                Ok(addr) => spawn_server(&mut set, "KCP", addr.clone(), run_kcp_server(addr)),
                Err(err) => error!("Invalid KCP listen address '{}': {}", request, err),
            },
            #[cfg(feature = "ws")]
            Some("ws") => match uri_to_socket_addr(&request) {
                Ok(addr) => spawn_server(&mut set, "WebSocket", addr.clone(), run_ws_server(addr)),
                Err(err) => error!("Invalid WebSocket listen address '{}': {}", request, err),
            },
            #[cfg(feature = "quic")]
            Some("quic") => match uri_to_socket_addr(&request) {
                Ok(addr) => spawn_server(&mut set, "QUIC", addr.clone(), run_quic_server(addr)),
                Err(err) => error!("Invalid QUIC listen address '{}': {}", request, err),
            },
            _ => error!("Unsupported URL scheme: {}", request),
        });

    if set.is_empty() {
        error!("No listening address configured");
        return Ok(());
    }

    while let Some(res) = set.join_next().await {
        match res {
            Ok(ServerExit {
                name,
                addr,
                result: Ok(()),
            }) => info!("{} server on {} exited normally", name, addr),
            Ok(ServerExit {
                name,
                addr,
                result: Err(err),
            }) => error!("{} server on {} exited with error: {:?}", name, addr, err),
            Err(err) => error!("A service task panicked or was cancelled: {}", err),
        }
    }

    info!("All server services have exited.");

    Ok(())
}
