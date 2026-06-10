//! 协议连接建立。
//!
//! 每种协议（TCP、KCP、WS、QUIC）提供一个 `ForwardConnector` 闭包，
//! 传输层连接池据此按需创建新的转发连接。

use super::session::ClientSession;
use super::transport::{ClientTransport, ForwardConnector};
use crate::client::now_secs;
use crate::CommonArgs;
use anyhow::anyhow;
use dashmap::DashMap;
use http::Uri;
use log::info;
#[cfg(feature = "ws")]
use np_base::net::ws_async_io::WebSocketAsyncIo;
#[cfg(feature = "quic")]
use s2n_quic::{client::Connect, Client as QUICClient};
#[cfg(feature = "tcp")]
use socket2::{SockRef, TcpKeepalive};
use std::collections::HashMap;
#[cfg(feature = "quic")]
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
#[cfg(feature = "ws")]
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(feature = "tcp")]
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
#[cfg(any(feature = "tcp", feature = "kcp"))]
use tokio::time::timeout;
#[cfg(feature = "kcp")]
use tokio_kcp::{KcpConfig, KcpNoDelayConfig, KcpStream};
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
#[cfg(any(feature = "tcp", feature = "kcp"))]
use tokio_rustls::TlsConnector;
use webpki_roots::TLS_SERVER_ROOTS;

/// TLS 握手超时时间（秒）。
#[cfg(any(feature = "tcp", feature = "kcp"))]
const TIMEOUT_TLS: u64 = 30;

// ─── 底层连接辅助函数 ─────────────────────────────────────────────────────────

#[cfg(feature = "tcp")]
async fn connect_with_tcp(request: &Uri) -> anyhow::Result<TcpStream> {
    let host = request
        .host()
        .ok_or_else(|| anyhow!("invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(80);

    info!("connecting via TCP to {}:{}", host, port);
    let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    info!("TCP connected to {}:{}", host, port);

    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;
    Ok(stream)
}

#[cfg(feature = "kcp")]
async fn connect_with_kcp(request: &Uri) -> anyhow::Result<KcpStream> {
    let host = request
        .host()
        .ok_or_else(|| anyhow!("invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(80);
    let addrs = tokio::net::lookup_host(format!("{}:{}", host, port)).await?;

    info!("connecting via KCP to {}:{}", host, port);

    let mut last_err = None;
    for addr in addrs {
        match KcpStream::connect(
            &KcpConfig {
                mtu: 1400,
                nodelay: KcpNoDelayConfig::fastest(),
                wnd_size: (1024, 1024),
                session_expire: Some(Duration::from_secs(60)),
                flush_write: false,
                flush_acks_input: false,
                stream: true,
                allow_recv_empty_packet: false,
            },
            addr,
        )
        .await
        {
            Ok(stream) => {
                info!("KCP connected to {}:{}", host, port);
                return Ok(stream);
            }
            Err(e) => last_err = Some(e),
        }
    }

    if let Some(err) = last_err {
        Err(anyhow!(err))
    } else {
        Err(anyhow!("cannot resolve hostname: {}", host))
    }
}

// ─── 公共入口 ──────────────────────────────────────────────────────────────────

/// 连接到服务端并运行客户端会话。
///
/// 根据 URI scheme 选择协议，构建 `ForwardConnector`，然后启动会话。
pub async fn run(common_args: &CommonArgs, request: Uri) -> anyhow::Result<()> {
    info!("connecting to server: {}", request);

    if common_args.enable_tls {
        run_with_tls(common_args, request).await
    } else {
        run_without_tls(common_args, request).await
    }
}

// ─── TLS 路径 ──────────────────────────────────────────────────────────────────

async fn run_with_tls(common_args: &CommonArgs, request: Uri) -> anyhow::Result<()> {
    let mut root_cert_store = RootCertStore::empty();
    if !common_args.ca_cert.is_empty() {
        // 加载自定义 CA 证书
        for cert in CertificateDer::pem_file_iter(&common_args.ca_cert)? {
            root_cert_store.add(cert?)?;
        }
    }
    // 加载默认根证书
    root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());

    // 创建TLS客户端配置
    let mut config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    // TLS 验证禁用
    if common_args.insecure {
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(super::tls::NoCertificateVerification::default()));
    }

    match request.scheme_str() {
        #[cfg(feature = "tcp")]
        Some("tcp") => {
            info!("using TCP+TLS");
            let domain = super::tls_server_name(common_args, &request)?;
            let config = Arc::new(config);
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                let domain = domain.clone();
                let config = config.clone();
                Arc::new(move || {
                    let request = request.clone();
                    let domain = domain.clone();
                    let config = config.clone();
                    Box::pin(async move {
                        let tls = TlsConnector::from(config);
                        match timeout(
                            Duration::from_secs(TIMEOUT_TLS),
                            tls.connect(domain, connect_with_tcp(&request).await?),
                        )
                        .await
                        {
                            Ok(result) => Ok(result?),
                            Err(_) => Err(anyhow!("TLS 握手超时 ({}秒)", TIMEOUT_TLS)),
                        }
                    })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "kcp")]
        Some("kcp") => {
            info!("using KCP+TLS");
            let domain = super::tls_server_name(common_args, &request)?;
            let config = Arc::new(config);
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                let domain = domain.clone();
                let config = config.clone();
                Arc::new(move || {
                    let request = request.clone();
                    let domain = domain.clone();
                    let config = config.clone();
                    Box::pin(async move {
                        let tls = TlsConnector::from(config);
                        match timeout(
                            Duration::from_secs(TIMEOUT_TLS),
                            tls.connect(domain, connect_with_kcp(&request).await?),
                        )
                        .await
                        {
                            Ok(result) => Ok(result?),
                            Err(_) => Err(anyhow!("TLS handshake timeout ({}s)", TIMEOUT_TLS)),
                        }
                    })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "ws")]
        Some("ws") => {
            let request = Uri::from_str(&request.to_string().replace("ws://", "wss://"))?;
            info!("using WSS");
            let config = Arc::new(config);
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                let config = config.clone();
                Arc::new(move || {
                    let request = request.clone();
                    let config = config.clone();
                    Box::pin(async move {
                        let tls_connector = tokio_tungstenite::Connector::Rustls(config);
                        let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
                            request,
                            None,
                            false,
                            Some(tls_connector),
                        )
                        .await?;
                        Ok(WebSocketAsyncIo::new(stream))
                    })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "quic")]
        Some("quic") => {
            info!("using QUIC");
            run_quic_client(common_args, &request, config).await
        }
        _ => Err(anyhow!("unsupported URL scheme: {}", request)),
    }
}

// ─── 非 TLS 路径 ──────────────────────────────────────────────────────────────

async fn run_without_tls(common_args: &CommonArgs, request: Uri) -> anyhow::Result<()> {
    match request.scheme_str() {
        #[cfg(feature = "tcp")]
        Some("tcp") => {
            info!("using TCP (no TLS)");
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                Arc::new(move || {
                    let request = request.clone();
                    Box::pin(async move { connect_with_tcp(&request).await })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "kcp")]
        Some("kcp") => {
            info!("using KCP (no TLS)");
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                Arc::new(move || {
                    let request = request.clone();
                    Box::pin(async move { connect_with_kcp(&request).await })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "ws")]
        Some("ws") => {
            info!("using WS (no TLS)");
            let connector: ForwardConnector<_> = {
                let request = request.clone();
                Arc::new(move || {
                    let request = request.clone();
                    Box::pin(async move {
                        let (stream, _) = tokio_tungstenite::connect_async(request).await?;
                        Ok(WebSocketAsyncIo::new(stream))
                    })
                })
            };
            run_client(common_args, connector).await
        }
        #[cfg(feature = "quic")]
        Some("quic") => Err(anyhow!("QUIC requires TLS (--enable-tls)")),
        _ => Err(anyhow!("unsupported URL scheme: {}", request)),
    }
}

// ─── QUIC 客户端 ──────────────────────────────────────────────────────────────

#[cfg(feature = "quic")]
async fn run_quic_client(
    common_args: &CommonArgs,
    request: &Uri,
    mut config: ClientConfig,
) -> anyhow::Result<()> {
    config.alpn_protocols = vec![b"h3".to_vec()];

    let client = QUICClient::builder()
        .with_io("0.0.0.0:0")?
        .with_tls(s2n_quic_rustls::Client::from(config))?
        .with_congestion_controller(s2n_quic_core::recovery::bbr::Endpoint::default())?
        .start()?;

    let host = request
        .host()
        .ok_or_else(|| anyhow!("invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(4433);

    info!("connecting via QUIC to {}:{}", host, port);

    let socket_addr = if let Ok(ip) = host.parse::<IpAddr>() {
        SocketAddr::new(ip, port)
    } else {
        format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("cannot resolve hostname: {}", host))?
    };

    let server_name = super::tls_server_name(common_args, request)?;
    let name_str = server_name.to_str();
    let connect = Connect::new(socket_addr).with_server_name(&*name_str);

    let mut connection = client.connect(connect).await?;
    // 开启保活，避免连接因长时间无业务数据被判定为空闲超时
    connection.keep_alive(true)?;

    info!("QUIC connected to {}:{}", host, port);

    // 构建 connector 闭包：在同一 QUIC 连接上打开新的双向流
    let handle = Arc::new(Mutex::new(connection.handle()));
    let connector: ForwardConnector<_> = Arc::new(move || {
        let handle = handle.clone();
        Box::pin(async move {
            let mut h = handle.lock().await;
            let stream = h.open_bidirectional_stream().await?;
            Ok(stream)
        })
    });

    run_client(common_args, connector).await
}

// ─── 通用客户端启动 ──────────────────────────────────────────────────────────

/// 创建 `ClientTransport` 并启动会话。
///
/// TCP/KCP/WS（有无 TLS）的统一入口。
/// 通过 `connector` 创建第一条连接用作控制流，后续按需创建转发连接。
async fn run_client<S>(
    common_args: &CommonArgs,
    connector: ForwardConnector<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let stream = connector().await?;
    let (reader, writer) = tokio::io::split(stream);
    let writer = Arc::new(Mutex::new(writer));

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let last_active_secs = Arc::new(AtomicU64::new(now_secs()));
    let last_read_secs = Arc::new(AtomicU64::new(now_secs()));

    let transport = ClientTransport::new(
        writer,
        connector,
        event_tx.clone(),
        last_active_secs.clone(),
        last_read_secs.clone(),
        common_args.transport_min_connections,
    );

    let mut session = ClientSession {
        transport,
        username: common_args.username.clone(),
        password: common_args.password.clone(),
        transport_max_connections: common_args.transport_max_connections,
        transport_idle_timeout_secs: common_args.transport_idle_timeout_secs,
        player_id: 0,
        outlets: Arc::new(DashMap::new()),
        inlets: Arc::new(DashMap::new()),
        tunnels: HashMap::new(),
    };

    session
        .run(reader, event_tx, event_rx, last_active_secs, last_read_secs)
        .await
}
