use anyhow::anyhow;
use brotli::CompressorWriter;
use brotli::DecompressorWriter;
use crypto_secretbox::aead::{generic_array::GenericArray, Aead, KeyInit, OsRng};
use crypto_secretbox::XSalsa20Poly1305;
use hex_literal::hex;
use std::io::{self, Write};

// Function to compress data using Brotli
pub fn compress_data(input: &[u8]) -> Result<Vec<u8>, io::Error> {
    let mut compressed_data = Vec::new();
    {
        let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
        compressor.write_all(input)?;
    }
    Ok(compressed_data)
}

// Function to decompress data using Brotli
pub fn decompress_data(input: &[u8]) -> Result<Vec<u8>, io::Error> {
    let mut decompressed_data = Vec::new();
    {
        let mut decompressor = DecompressorWriter::new(&mut decompressed_data, 4096);
        decompressor.write_all(input)?;
    }
    Ok(decompressed_data)
}

#[derive(Clone)]
pub enum EncryptionMethod {
    None,
    XSalsa20Poly1305,
}

pub fn get_method(method: &str) -> EncryptionMethod {
    match method {
        "XSalsa20Poly1305" => EncryptionMethod::XSalsa20Poly1305,
        "None" => EncryptionMethod::None,
        _ => EncryptionMethod::None,
    }
}

pub fn get_method_name(method: &EncryptionMethod) -> String {
    match method {
        EncryptionMethod::XSalsa20Poly1305 => "XSalsa20Poly1305".to_string(),
        _ => "None".to_string(),
    }
}

pub fn generate_key(method: &EncryptionMethod) -> Vec<u8> {
    match method {
        EncryptionMethod::None => "None".into(),
        EncryptionMethod::XSalsa20Poly1305 => {
            let key = XSalsa20Poly1305::generate_key(&mut OsRng);
            key.to_vec()
        }
    }
}

pub fn encrypt(method: &EncryptionMethod, key: &[u8], data: &[u8]) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.to_vec()),
        EncryptionMethod::XSalsa20Poly1305 => {
            const NONCE: &[u8; 24] = &hex!("69696ee955b62b73cd622da855fc73d68219e0036b7a0b37");

            let key = GenericArray::from_slice(key);
            let nonce = GenericArray::from_slice(NONCE); // 24-bytes; unique
            let cipher = XSalsa20Poly1305::new(key);
            let ciphertext = cipher.encrypt(nonce, data);
            match ciphertext {
                Ok(data) => Ok(data),
                Err(err) => Err(anyhow!(err.to_string())),
            }
        }
    }
}

pub fn decrypt(method: &EncryptionMethod, key: &[u8], data: &[u8]) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.to_vec()),
        EncryptionMethod::XSalsa20Poly1305 => {
            const NONCE: &[u8; 24] = &hex!("69696ee955b62b73cd622da855fc73d68219e0036b7a0b37");

            let key = GenericArray::from_slice(key);
            let nonce = GenericArray::from_slice(NONCE); // 24-bytes; unique
            let cipher = XSalsa20Poly1305::new(key);
            let ciphertext = cipher.decrypt(nonce, data);
            match ciphertext {
                Ok(data) => Ok(data),
                Err(err) => Err(anyhow!(err.to_string())),
            }
        }
    }
}
