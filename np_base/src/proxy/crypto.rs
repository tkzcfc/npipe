use anyhow::anyhow;
use lz4_flex::block::{compress_prepend_size, decompress_size_prepended};
use rand::Rng;
use std::borrow::Cow;
use std::{fmt, io};

// Function to compress data using Brotli
pub fn compress_data(input: &[u8]) -> Result<Vec<u8>, io::Error> {
    let compressed = compress_prepend_size(input);
    Ok(compressed)
}

// Function to decompress data
pub fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, io::Error> {
    decompress_size_prepended(compressed)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

#[derive(Clone)]
pub enum EncryptionMethod {
    None,
    Aes128,
    Xor,
}

impl EncryptionMethod {
    pub fn is_none(&self) -> bool {
        matches!(self, EncryptionMethod::None)
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
            (0..32)
                .map(|_| {
                    let n: u8 = rng.gen_range(33..127);
                    n
                })
                .collect::<Vec<u8>>()
        }
        EncryptionMethod::Xor => {
            let mut rng = rand::thread_rng();
            let key_len = rng.gen_range(8..64);
            (0..key_len)
                .map(|_| {
                    let n: u8 = rng.gen_range(1..255);
                    n
                })
                .collect::<Vec<u8>>()
        }
    }
}

pub fn encrypt(
    method: &EncryptionMethod,
    key: &[u8],
    data: Cow<'_, [u8]>,
) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.into_owned()),
        EncryptionMethod::Aes128 => {
            // &*data = &[u8]，Cow::Borrowed 时零拷贝直接借用，Cow::Owned 时借用 Vec 内容
            simplestcrypt::encrypt_and_serialize(key, &*data)
                .map_err(|err| anyhow!("encrypt_and_serialize error: {:?}", err))
        }
        EncryptionMethod::Xor => {
            if key.is_empty() {
                return Ok(data.into_owned());
            }
            // into_owned(): Cow::Owned → O(0) 直接取出 Vec；Cow::Borrowed → O(n) 复制一次
            Ok(xor_encrypt_decrypt(data.into_owned(), key))
        }
    }
}

pub fn decrypt(
    method: &EncryptionMethod,
    key: &[u8],
    data: Cow<'_, [u8]>,
) -> anyhow::Result<Vec<u8>> {
    match method {
        EncryptionMethod::None => Ok(data.into_owned()),
        EncryptionMethod::Aes128 => {
            // 同 encrypt，直接借用，不需要调用方先 to_vec()
            simplestcrypt::deserialize_and_decrypt(key, &*data)
                .map_err(|err| anyhow!("deserialize_and_decrypt error: {:?}", err))
        }
        EncryptionMethod::Xor => {
            if key.is_empty() {
                return Ok(data.into_owned());
            }
            Ok(xor_encrypt_decrypt(data.into_owned(), key))
        }
    }
}

fn xor_encrypt_decrypt(mut data: Vec<u8>, key: &[u8]) -> Vec<u8> {
    for (data_byte, key_byte) in data.iter_mut().zip(key.iter().cycle()) {
        *data_byte ^= *key_byte;
    }
    data

    // data.iter()
    //     .zip(key.iter().cycle())
    //     .map(|(&data_byte, &key_byte)| data_byte ^ key_byte)
    //     .collect()
}
