//! 客户端模块：连接建立、会话管理、传输层与帧读写。

mod connect;
mod io;
mod session;
mod tls;
mod transport;

use crate::CommonArgs;
use anyhow::anyhow;
pub use connect::run;
use http::Uri;
use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 Unix 时间戳（秒）。
#[inline(always)]
pub(super) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 从命令行参数或请求 URI 解析 TLS 服务端名称。
#[cfg(any(feature = "tcp", feature = "kcp", feature = "quic"))]
pub(super) fn tls_server_name(
    common_args: &CommonArgs,
    request: &Uri,
) -> anyhow::Result<tokio_rustls::rustls::pki_types::ServerName<'static>> {
    use tokio_rustls::rustls::pki_types::ServerName;

    if common_args.tls_server_name.is_empty() {
        let host = request
            .host()
            .ok_or_else(|| anyhow!("invalid URI: missing host"))?;
        Ok(ServerName::try_from(host)?.to_owned())
    } else {
        Ok(ServerName::try_from(common_args.tls_server_name.as_str())?.to_owned())
    }
}
