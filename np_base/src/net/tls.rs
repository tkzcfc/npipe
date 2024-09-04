use anyhow::anyhow;
use std::fs::File;
use std::io::BufReader;
use tokio_rustls::rustls::{Certificate, PrivateKey};

pub fn load_certs(path: &str) -> anyhow::Result<Vec<Certificate>> {
    let cert_file = File::open(path)?;
    let mut reader = BufReader::new(cert_file);

    let certs = rustls_pemfile::certs(&mut reader)
        .filter_map(|x| x.ok())
        .map(|x| Certificate(x.to_vec()))
        .collect::<Vec<_>>();

    Ok(certs)
}

pub fn load_private_key(path: &str) -> anyhow::Result<PrivateKey> {
    let mut reader = BufReader::new(File::open(path)?);
    let private_key_der = rustls_pemfile::private_key(&mut reader)?;

    return if let Some(private_key_der) = private_key_der {
        Ok(PrivateKey(private_key_der.secret_der().into()))
    } else {
        Err(anyhow!("The private key file ({path}) format is incorrect"))
    };
}
