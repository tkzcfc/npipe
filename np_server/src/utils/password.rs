use anyhow::anyhow;
use pbkdf2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Params, Pbkdf2,
};
use subtle::ConstantTimeEq;

/// PBKDF2-HMAC-SHA256 迭代次数，越大越抗离线爆破，但每次登录 CPU 开销也越高
const PBKDF2_ROUNDS: u32 = 100_000;

/// 已哈希密码的 PHC 串前缀，用于区分明文与哈希（迁移时判断是否需要升级）
pub const PBKDF2_PHC_PREFIX: &str = "$pbkdf2-";

/// 用带随机盐的 PBKDF2-HMAC-SHA256 生成 PHC 密码哈希串
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|e| anyhow!("salt rng failed: {e}"))?;
    let salt =
        SaltString::encode_b64(&salt_bytes).map_err(|e| anyhow!("salt encode failed: {e}"))?;
    let params = Params {
        rounds: PBKDF2_ROUNDS,
        output_length: 32,
    };
    let hash = Pbkdf2
        .hash_password_customized(password.as_bytes(), None, None, params, &salt)
        .map_err(|e| anyhow!("hash_password failed: {e}"))?
        .to_string();
    Ok(hash)
}

/// 校验明文密码与 PHC 哈希串是否匹配
pub fn verify_password(password: &str, phc_hash: &str) -> bool {
    match PasswordHash::new(phc_hash) {
        Ok(parsed) => Pbkdf2.verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// 一条已存储的密码是否已是 PBKDF2 哈希
pub fn is_hashed(stored: &str) -> bool {
    stored.starts_with(PBKDF2_PHC_PREFIX)
}

/// 常量时间比较，用于配置文件中的明文管理员口令，避免计时侧信道
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let hash = hash_password("s3cret!").unwrap();
        assert!(is_hashed(&hash));
        assert!(verify_password("s3cret!", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn plaintext_is_not_hashed() {
        assert!(!is_hashed("plaintextpw"));
    }

    #[test]
    fn constant_time_eq_matches() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "abcd"));
    }
}
