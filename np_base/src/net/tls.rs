use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

pub struct TlsConfiguration {
    pub certificate: String,
    pub key: String,
}

impl TryFrom<TlsConfiguration> for TlsAcceptor {
    type Error = anyhow::Error;
    fn try_from(tls_configuration: TlsConfiguration) -> Result<Self, Self::Error> {
        let cert_path = std::path::PathBuf::from(&tls_configuration.certificate);
        let key_path = std::path::PathBuf::from(&tls_configuration.key);

        let certs = CertificateDer::pem_file_iter(cert_path)?.collect::<Result<Vec<_>, _>>()?;
        let key = PrivateKeyDer::from_pem_file(key_path)?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(TlsAcceptor::from(Arc::new(config)))
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
