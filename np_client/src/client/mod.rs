mod connect;
mod core;
mod io;
mod tls_danger;
mod transport;

use crate::CommonArgs;
use anyhow::anyhow;
pub use connect::run;
use http::Uri;
use s2n_quic_rustls::rustls::pki_types::ServerName;
use std::time::{SystemTime, UNIX_EPOCH};

/// 返回当前 Unix 时间戳（秒）
#[inline(always)]
pub(super) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(any(feature = "tcp", feature = "kcp", feature = "quic"))]
pub(super) fn tls_server_name(
    common_args: &CommonArgs,
    request: &Uri,
) -> anyhow::Result<ServerName<'static>> {
    if common_args.tls_server_name.is_empty() {
        let host = request
            .host()
            .ok_or_else(|| anyhow!("Invalid URI: missing host"))?;
        Ok(ServerName::try_from(host)?.to_owned())
    } else {
        Ok(ServerName::try_from(common_args.tls_server_name.as_str())?.to_owned())
    }
}
