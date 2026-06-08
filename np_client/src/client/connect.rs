use super::transport::{Client, ClientTransport, ForwardConnector};
#[cfg(feature = "quic")]
use super::transport::{ClientTransportKind, QuicClientConnection, QuicTransportState};
use crate::client::now_secs;
use crate::CommonArgs;
use anyhow::anyhow;
use dashmap::DashMap;
use http::Uri;
use log::info;
#[cfg(feature = "ws")]
use np_base::net::ws_async_io::WebSocketAsyncIo;
#[cfg(feature = "quic")]
use s2n_quic::{
    client::Connect, stream::BidirectionalStream as QuicBidirectionalStream, Client as QUICClient,
};
#[cfg(feature = "tcp")]
use socket2::{SockRef, TcpKeepalive};
use std::collections::HashMap;
#[cfg(feature = "quic")]
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
#[cfg(feature = "ws")]
use std::str::FromStr;
#[cfg(feature = "quic")]
use std::sync::atomic::AtomicU32;
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
#[cfg(any(feature = "tcp", feature = "kcp", feature = "quic"))]
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
#[cfg(any(feature = "tcp", feature = "kcp"))]
use tokio_rustls::TlsConnector;
use webpki_roots::TLS_SERVER_ROOTS;

#[cfg(any(feature = "tcp", feature = "kcp"))]
const TIMEOUT_TLS: u64 = 30;

#[cfg(feature = "tcp")]
async fn connect_with_tcp(request: &Uri) -> anyhow::Result<TcpStream> {
    let host = request
        .host()
        .ok_or_else(|| anyhow!("Invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(80); // 默认端口为80

    info!("Attempting to connect to {}:{} with TCP", host, port);
    let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    info!("TCP successfully connected to {}:{}", host, port);

    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30));
    let sf = SockRef::from(&stream);
    sf.set_tcp_keepalive(&ka)?;
    Ok(stream)
}

#[cfg(feature = "kcp")]
async fn connect_with_kcp(request: &Uri) -> anyhow::Result<KcpStream> {
    let host = request
        .host()
        .ok_or_else(|| anyhow!("Invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(80); // 默认端口为80
    let addrs = tokio::net::lookup_host(format!("{}:{}", host, port)).await?;

    println!("Attempting to connect to {}:{} with KCP", host, port);

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
                info!("KCP successfully connected to {}:{}", host, port);
                return Ok(stream);
            }
            Err(e) => last_err = Some(e),
        }
    }

    if let Some(err) = last_err {
        Err(anyhow!(err))
    } else {
        Err(anyhow!("Unable to resolve domain name: {}", host))
    }
}

#[cfg(feature = "quic")]
async fn connect_with_quic(
    request: &Uri,
    server_name: ServerName<'_>,
    mut config: ClientConfig,
) -> anyhow::Result<QuicClientConnection> {
    info!("Initializing QUIC client with TLS configuration");

    config.alpn_protocols = vec![b"h3".to_vec()];

    let client = QUICClient::builder()
        .with_io("0.0.0.0:0")?
        .with_tls(s2n_quic_rustls::Client::from(config))?
        .with_congestion_controller(s2n_quic_core::recovery::bbr::Endpoint::default())?
        .start()?;

    let host = request
        .host()
        .ok_or_else(|| anyhow!("Invalid URI: missing host"))?;
    let port = request.port_u16().unwrap_or(4433); // 默认端口为4433

    info!("Attempting to connect to {}:{} with QUIC", host, port);

    let socket_addr = if let Ok(host) = host.parse::<IpAddr>() {
        SocketAddr::new(host, port)
    } else {
        // DNS解析域名
        format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("Unable to resolve domain name: {}", host))?
    };

    let name_str = server_name.to_str();
    let connect = Connect::new(socket_addr).with_server_name(&*name_str);

    let mut connection = client.connect(connect).await?;

    // 开启保活，避免连接因长时间无业务数据被判定为空闲超时。
    connection.keep_alive(true)?;
    let handle = connection.handle();

    // 打开一条新的双向流，后续由通用读写逻辑拆分读半边和写半边。
    let control_stream = connection.open_bidirectional_stream().await?;

    info!("QUIC successfully connected to {}:{}", host, port);

    Ok(QuicClientConnection {
        handle,
        control_stream,
    })
}

pub async fn run(common_args: &CommonArgs, request: Uri) -> anyhow::Result<()> {
    info!("Start connecting to server {}", request);

    // 升级为TLS连接
    if common_args.enable_tls {
        let mut root_cert_store = RootCertStore::empty();
        // 加载自定义的根证书
        if !common_args.ca_cert.is_empty() {
            for cert in CertificateDer::pem_file_iter(&common_args.ca_cert)? {
                root_cert_store.add(cert?)?;
            }
        }
        // 加载系统默认的根证书
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());

        // 创建TLS配置
        let mut config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth(); // 客户端不使用证书认证。

        if common_args.insecure {
            config.dangerous().set_certificate_verifier(Arc::new(
                super::tls_danger::NoCertificateVerification::default(),
            ));
        }

        match request.scheme_str() {
            #[cfg(feature = "tcp")]
            Some("tcp") => {
                info!("Connecting to server {} with TCP&TLS", request);
                let domain = super::tls_server_name(common_args, &request)?;
                let config = Arc::new(config);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    let domain = domain.clone();
                    let config = config.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        let domain = domain.clone();
                        let config = config.clone();
                        Box::pin(async move {
                            let connector = TlsConnector::from(config);
                            match timeout(
                                Duration::from_secs(TIMEOUT_TLS),
                                connector.connect(domain, connect_with_tcp(&request).await?),
                            )
                            .await
                            {
                                Ok(result) => Ok(result?),
                                Err(err) => Err(anyhow!("tls connect timeout, error: {}", err)),
                            }
                        })
                    })
                };

                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "kcp")]
            Some("kcp") => {
                info!("Connecting to server {} with KCP&TLS", request);
                let domain = super::tls_server_name(common_args, &request)?;
                let config = Arc::new(config);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    let domain = domain.clone();
                    let config = config.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        let domain = domain.clone();
                        let config = config.clone();
                        Box::pin(async move {
                            let connector = TlsConnector::from(config);
                            match timeout(
                                Duration::from_secs(TIMEOUT_TLS),
                                connector.connect(domain, connect_with_kcp(&request).await?),
                            )
                            .await
                            {
                                Ok(result) => Ok(result?),
                                Err(err) => Err(anyhow!("tls connect timeout, error: {}", err)),
                            }
                        })
                    })
                };

                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "ws")]
            Some("ws") => {
                let request = Uri::from_str(&request.to_string().replace("ws://", "wss://"))?;
                info!("Connecting to server {} with WSS", request);
                let config = Arc::new(config);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    let config = config.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        let config = config.clone();
                        Box::pin(async move {
                            let connector = tokio_tungstenite::Connector::Rustls(config);
                            let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
                                request,
                                None,
                                false,
                                Some(connector),
                            )
                            .await?;
                            Ok(WebSocketAsyncIo::new(stream))
                        })
                    })
                };
                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "quic")]
            Some("quic") => {
                info!("Connecting to server {} with QUIC", request);
                let domain = super::tls_server_name(common_args, &request)?;
                let connection = connect_with_quic(&request, domain, config).await?;
                run_quic_client(common_args, connection).await
            }
            _ => Err(anyhow!("Unsupported URL scheme: {}", request)),
        }
    } else {
        match request.scheme_str() {
            #[cfg(feature = "tcp")]
            Some("tcp") => {
                info!("Connecting to server {} with TCP", request);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        Box::pin(async move { connect_with_tcp(&request).await })
                    })
                };
                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "kcp")]
            Some("kcp") => {
                info!("Connecting to server {} with KCP", request);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        Box::pin(async move { connect_with_kcp(&request).await })
                    })
                };
                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "ws")]
            Some("ws") => {
                info!("Connecting to server {} with WS", request);
                let forward_connector: ForwardConnector<_> = {
                    let request = request.clone();
                    Arc::new(move || {
                        let request = request.clone();
                        Box::pin(async move {
                            let (stream, _) = tokio_tungstenite::connect_async(request).await?;
                            Ok(WebSocketAsyncIo::new(stream))
                        })
                    })
                };
                let stream = forward_connector().await?;
                run_client(common_args, stream, Some(forward_connector)).await
            }
            #[cfg(feature = "quic")]
            Some("quic") => {
                Err(anyhow!("QUIC protocol requires Transport Layer Security (TLS) to be enabled for secure communication."))
            }
            _ => Err(anyhow!("Unsupported URL scheme: {}", request)),
        }
    }
}

#[cfg(feature = "quic")]
async fn run_quic_client(
    common_args: &CommonArgs,
    connection: QuicClientConnection,
) -> anyhow::Result<()> {
    let (reader, writer) = tokio::io::split(connection.control_stream);
    let writer = Arc::new(Mutex::new(writer));
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let last_active_secs = Arc::new(AtomicU64::new(now_secs()));
    let last_read_secs = Arc::new(AtomicU64::new(now_secs()));

    let transport_state = Arc::new(QuicTransportState {
        handle: Mutex::new(connection.handle),
        control_writer: writer,
        event_tx: event_tx.clone(),
        last_active_secs: last_active_secs.clone(),
        last_read_secs: last_read_secs.clone(),
        token: Mutex::new(String::new()),
        max_forward_paths: AtomicU32::new(0),
        idle_timeout_secs: AtomicU32::new(0),
        forward_path_create_lock: Mutex::new(()),
        next_connection_id: AtomicU64::new(1),
        session_paths: DashMap::new(),
        forward_paths: DashMap::new(),
    });
    QuicTransportState::start_idle_cleanup(transport_state.clone());
    let transport = ClientTransport {
        kind: ClientTransportKind::Quic(transport_state),
    };

    let mut client = Client::<QuicBidirectionalStream> {
        transport: transport.clone(),
        username: common_args.username.clone(),
        password: common_args.password.clone(),
        transport_max_connections: common_args.transport_max_connections,
        transport_idle_timeout_secs: common_args.transport_idle_timeout_secs,
        player_id: 0u32,
        outlets: Arc::new(DashMap::new()),
        inlets: Arc::new(DashMap::new()),
        tunnels: HashMap::new(),
    };

    client
        .run(reader, event_tx, event_rx, last_active_secs, last_read_secs)
        .await
}

async fn run_client<S>(
    common_args: &CommonArgs,
    stream: S,
    connector: Option<ForwardConnector<S>>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);

    let writer = Arc::new(Mutex::new(writer));
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let last_active_secs = Arc::new(AtomicU64::new(now_secs()));
    let last_read_secs = Arc::new(AtomicU64::new(now_secs()));
    let transport = if common_args.transport_max_connections > 0 {
        if let Some(connector) = connector {
            ClientTransport::pool(
                writer,
                connector,
                event_tx.clone(),
                last_active_secs.clone(),
                last_read_secs.clone(),
            )
        } else {
            ClientTransport::legacy(
                writer,
                common_args.transport_max_connections,
                common_args.transport_idle_timeout_secs,
            )
        }
    } else {
        ClientTransport::legacy(
            writer,
            common_args.transport_max_connections,
            common_args.transport_idle_timeout_secs,
        )
    };

    let mut client = Client::<S> {
        transport: transport.clone(),
        username: common_args.username.clone(),
        password: common_args.password.clone(),
        transport_max_connections: common_args.transport_max_connections,
        transport_idle_timeout_secs: common_args.transport_idle_timeout_secs,
        player_id: 0u32,
        outlets: Arc::new(DashMap::new()),
        inlets: Arc::new(DashMap::new()),
        tunnels: HashMap::new(),
    };

    client
        .run(reader, event_tx, event_rx, last_active_secs, last_read_secs)
        .await
}
