use anyhow::{anyhow, Context};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;

pub struct TlsConfiguration {
    pub certificate: String,
    pub key: String,
}

impl TryFrom<TlsConfiguration> for TlsAcceptor {
    type Error = anyhow::Error;
    fn try_from(tls_configuration: TlsConfiguration) -> Result<Self, Self::Error> {
        let certs = load_certs(&tls_configuration.certificate)?;
        let keys = load_private_key(&tls_configuration.key)?;

        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys)?;

        Ok(TlsAcceptor::from(Arc::new(server_config)))
    }
}

pub fn load_certs(path: &str) -> anyhow::Result<Vec<Certificate>> {
    let cert_file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let mut reader = BufReader::new(cert_file);

    let certs = rustls_pemfile::certs(&mut reader)
        .filter_map(|x| x.ok())
        .map(|x| Certificate(x.to_vec()))
        .collect::<Vec<_>>();

    Ok(certs)
}

pub fn load_private_key(path: &str) -> anyhow::Result<PrivateKey> {
    let key_file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let mut reader = BufReader::new(key_file);
    let private_key_der = rustls_pemfile::private_key(&mut reader)?;

    if let Some(private_key_der) = private_key_der {
        Ok(PrivateKey(private_key_der.secret_der().into()))
    } else {
        Err(anyhow!("The private key file ({path}) format is incorrect"))
    }
}

const TIMEOUT_TLS: u64 = 15;

// ref https://github.com/netskillzgh/rollo/blob/master/rollo/src/server/world_socket_mgr.rs#L183
pub async fn try_tls<IO>(
    stream: IO,
    tls_acceptor: TlsAcceptor,
) -> anyhow::Result<tokio_rustls::TlsStream<IO>>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let stream = tokio::time::timeout(
        Duration::from_secs(TIMEOUT_TLS),
        tls_acceptor.accept(stream),
    )
    .await??;
    Ok(tokio_rustls::TlsStream::Server(stream))
}
