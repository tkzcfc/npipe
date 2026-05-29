mod auth;
mod dashboard;
mod logs;
mod maintenance;
mod player;
mod proto;
mod support;
mod tunnel;

use actix_cors::Cors;
use actix_identity::IdentityMiddleware;
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key},
    middleware, web, App, HttpServer,
};

use crate::global::config::GLOBAL_CONFIG;
use log::warn;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;

fn load_web_tls_config() -> anyhow::Result<rustls::ServerConfig> {
    if GLOBAL_CONFIG.web_tls_cert.is_empty() || GLOBAL_CONFIG.web_tls_key.is_empty() {
        if !GLOBAL_CONFIG.web_tls_auto_self_signed {
            anyhow::bail!("web TLS is enabled, but web_tls_cert or web_tls_key is empty");
        }
        return generate_self_signed_web_tls_config();
    }

    let cert_file = File::open(&GLOBAL_CONFIG.web_tls_cert)?;
    let key_file = File::open(&GLOBAL_CONFIG.web_tls_key)?;
    let mut cert_reader = BufReader::new(cert_file);
    let mut key_reader = BufReader::new(key_file);

    let cert_chain = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;
    let private_key = rustls_pemfile::private_key(&mut key_reader)?
        .ok_or_else(|| anyhow::anyhow!("web TLS private key not found"))?;

    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;

    config.alpn_protocols.push(b"h2".to_vec());
    config.alpn_protocols.push(b"http/1.1".to_vec());
    Ok(config)
}

fn generate_self_signed_web_tls_config() -> anyhow::Result<rustls::ServerConfig> {
    warn!("web TLS certificate is not configured; generated a temporary self-signed certificate");

    let subject_alt_names = BTreeSet::from([
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "0.0.0.0".to_owned(),
        web_tls_subject_name(),
    ])
    .into_iter()
    .collect::<Vec<_>>();
    let certified_key = rcgen::generate_simple_self_signed(subject_alt_names)?;
    let cert_der = CertificateDer::from(certified_key.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified_key.key_pair.serialize_der(),
    ));

    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    config.alpn_protocols.push(b"h2".to_vec());
    config.alpn_protocols.push(b"http/1.1".to_vec());
    Ok(config)
}

fn web_tls_subject_name() -> String {
    GLOBAL_CONFIG
        .web_addr
        .rsplit_once(':')
        .map(|(host, _)| host.trim_matches(['[', ']']).to_owned())
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| "localhost".to_owned())
}

/// http server
pub async fn run_http_server(addr: &str, web_base_dir: &str) -> anyhow::Result<()> {
    let secret_key = Key::generate();
    let web_base_dir = web_base_dir.to_string();

    let cookie_secure = GLOBAL_CONFIG.web_enable_tls || GLOBAL_CONFIG.web_cookie_secure;
    let server = HttpServer::new(move || {
        App::new()
            // 添加 Cors 中间件，并允许所有跨域请求
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .service(web::resource("/api/login").route(web::post().to(auth::login)))
            .service(web::resource("/api/logout").route(web::post().to(auth::logout)))
            .service(web::resource("/api/test_auth").route(web::post().to(auth::test_auth)))
            .service(web::resource("/api/player_list").route(web::post().to(player::player_list)))
            .service(
                web::resource("/api/remove_player").route(web::post().to(player::remove_player)),
            )
            .service(web::resource("/api/add_player").route(web::post().to(player::add_player)))
            .service(
                web::resource("/api/update_player").route(web::post().to(player::update_player)),
            )
            .service(
                web::resource("/api/rename_player").route(web::post().to(player::rename_player)),
            )
            .service(
                web::resource("/api/reset_player_password")
                    .route(web::post().to(player::reset_player_password)),
            )
            .service(
                web::resource("/api/update_player_status")
                    .route(web::post().to(player::update_player_status)),
            )
            .service(
                web::resource("/api/update_player_web_access")
                    .route(web::post().to(player::update_player_web_access)),
            )
            .service(web::resource("/api/kick_player").route(web::post().to(player::kick_player)))
            .service(
                web::resource("/api/player_detail").route(web::post().to(player::player_detail)),
            )
            .service(
                web::resource("/api/dashboard_overview")
                    .route(web::post().to(dashboard::dashboard_overview)),
            )
            .service(
                web::resource("/api/traffic_stats").route(web::post().to(player::traffic_stats)),
            )
            .service(web::resource("/api/login_history").route(web::post().to(logs::login_history)))
            .service(
                web::resource("/api/operation_logs").route(web::post().to(logs::operation_logs)),
            )
            .service(
                web::resource("/api/database_maintenance_info")
                    .route(web::post().to(maintenance::database_maintenance_info)),
            )
            .service(
                web::resource("/api/cleanup_database")
                    .route(web::post().to(maintenance::cleanup_database)),
            )
            .service(web::resource("/api/tunnel_list").route(web::post().to(tunnel::tunnel_list)))
            .service(
                web::resource("/api/tunnel_detail").route(web::post().to(tunnel::tunnel_detail)),
            )
            .service(
                web::resource("/api/remove_tunnel").route(web::post().to(tunnel::remove_tunnel)),
            )
            .service(web::resource("/api/add_tunnel").route(web::post().to(tunnel::add_tunnel)))
            .service(
                web::resource("/api/update_tunnel").route(web::post().to(tunnel::update_tunnel)),
            )
            .service(
                web::resource("/api/update_tunnel_status")
                    .route(web::post().to(tunnel::update_tunnel_status)),
            )
            .service(
                web::resource("/api/tunnel_diagnose")
                    .route(web::post().to(tunnel::tunnel_diagnose)),
            )
            .service(actix_files::Files::new("/", &web_base_dir).index_file("index.html"))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_name("auth-id".to_owned())
                    .cookie_secure(cookie_secure)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::minutes(60)),
                    )
                    .build(),
            )
            .wrap(middleware::NormalizePath::trim())
    });

    let server = if GLOBAL_CONFIG.web_enable_tls {
        server.bind_rustls_0_23(addr, load_web_tls_config()?)?
    } else {
        server.bind(addr)?
    }
    .run();

    server.await?;
    Ok(())
}
