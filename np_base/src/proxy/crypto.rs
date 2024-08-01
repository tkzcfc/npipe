use anyhow::anyhow;
use rand::Rng;
use std::{fmt, io};
use lz4_flex::block::{compress_prepend_size, decompress_size_prepended};


// Function to compress data using Brotli
pub fn compress_data(input: &[u8]) -> Result<Vec<u8>, io::Error> {
    let compressed = compress_prepend_size(input);
    Ok(compressed)
}

// Function to decompress data using Brotli
pub fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, io::Error> {
    let uncompressed = decompress_size_prepended(&compressed).unwrap();
    Ok(uncompressed)
}

#[derive(Clone)]
pub enum EncryptionMethod {
    None,
    Aes128,
    Xor,
}

impl EncryptionMethod {
    pub fn is_none(&self) -> bool {
        match self {
            EncryptionMethod::None => true,
            _ => false,
        }
    }
}

pub fn get_method(method: &str) -> EncryptionMethod {
    match method {
        "Aes128" => EncryptionMethod::Aes128,
        "None" => EncryptionMethod::None,
        "Xor" => EncryptionMethod::Xor,
        _ => EncryptionMethod::None,
    }
}

impl fmt::Display for EncryptionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionMethod::None => {
                write!(f, "None")
            }
            EncryptionMethod::Aes128 => {
                write!(f, "Aes128")
            }
            EncryptionMethod::Xor => {
                write!(f, "Xor")
            }
        }
    }
}

pub fn generate_key(method: &EncryptionMethod) -> Vec<u8> {
    match method {
        EncryptionMethod::None => "None".into(),
        EncryptionMethod::Aes128 => {
            let mut rng = rand::thread_rng();
            (0..32).map(|_| {
                let n: u8 = rng.gen_range(33..127);
                n
            }).collect::<Vec<u8>>()
        }
        EncryptionMethod::Xor => {
            let mut rng = rand::thread_rng();
            let key_len = rng.gen_range(1..32);
            (0..key_len).map(|_| {
                let n: u8 = rng.gen_range(1..255);
                n
            }).collect::<Vec<u8>>()
        }
    }
}

pub fn encrypt(method: &EncryptionMethod, key: &[u8], data: &[u8]) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.to_vec()),
        EncryptionMethod::Aes128 => {
            let ciphertext = simplestcrypt::encrypt_and_serialize(key, data);
            match ciphertext {
                Ok(data) => Ok(data),
                Err(err) => Err(anyhow!("encrypt_and_serialize error: {:?}", err)),
            }
        }
        EncryptionMethod::Xor => {
            if key.is_empty() {
                return Ok(data.to_vec());
            }
            Ok(xor_encrypt_decrypt(data, key))
        }
    }
}

pub fn decrypt(method: &EncryptionMethod, key: &[u8], data: &[u8]) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.to_vec()),
        EncryptionMethod::Aes128 => {
            let plaintext = simplestcrypt::deserialize_and_decrypt(key, data);
            match plaintext {
                Ok(data) => Ok(data),
                Err(err) => Err(anyhow!("deserialize_and_decrypt error: {:?}", err)),
            }
        }
        EncryptionMethod::Xor => {
            if key.is_empty() {
                return Ok(data.to_vec());
            }
            Ok(xor_encrypt_decrypt(data, key))
        }
    }
}

fn xor_encrypt_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .zip(key.iter().cycle())
        .map(|(&data_byte, &key_byte)| data_byte ^ key_byte)
        .collect()
}
