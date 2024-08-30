use rustls_pemfile::{certs, rsa_private_keys};
use std::io;
use std::path::Path;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

pub fn load_config(certificate: &Path, key: &String) -> io::Result<ServerConfig> {
    let certs = certs(&mut std::io::BufReader::new(std::fs::File::open(
        certificate,
    )?))
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
    .map(|mut certs| certs.drain(..).map(Certificate).collect())?;

    let mut keys: Vec<PrivateKey> =
        rsa_private_keys(&mut std::io::BufReader::new(std::fs::File::open(key)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
            .map(|mut keys| keys.drain(..).map(PrivateKey).collect())?;

    let server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.swap_remove(0))
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Problem when setting cert to config",
            )
        })?;

    Ok(server_config)
}
