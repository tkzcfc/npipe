use std::fs::File;
use std::io::BufReader;
use tokio_rustls::rustls::{Certificate, PrivateKey};

pub fn load_certs(path: &str) -> anyhow::Result<Vec<Certificate>> {
    let cert_file = File::open(path)?;
    let mut reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();
    Ok(certs)
}

pub fn load_private_key(path: &str) -> anyhow::Result<PrivateKey> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?
        .into_iter()
        .map(PrivateKey)
        .collect::<Vec<_>>();

    if keys.is_empty() {
        let mut reader = BufReader::new(File::open(path)?);
        keys = rustls_pemfile::rsa_private_keys(&mut reader)?
            .into_iter()
            .map(PrivateKey)
            .collect::<Vec<_>>();
    }

    anyhow::ensure!(
        !keys.is_empty(),
        "The private key file ({path}) format is incorrect"
    );

    Ok(keys[0].clone())
}
