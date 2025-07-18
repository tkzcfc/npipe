use std::fmt::Display;

#[derive(Debug)]
pub enum NetType {
    Tcp,
    Kcp,
    Ws,
}

impl Display for NetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetType::Tcp => write!(f, "tcp"),
            NetType::Kcp => write!(f, "kcp"),
            NetType::Ws => write!(f, "ws"),
        }
    }
}
pub fn parse(addrs: &str) -> Vec<(NetType, String)> {
    addrs
        .split(',')
        .map(|addr| {
            let trimmed_addr = addr.trim();
            if let Some(stripped) = trimmed_addr.strip_prefix("tcp://") {
                (NetType::Tcp, stripped.to_string())
            } else if let Some(stripped) = trimmed_addr.strip_prefix("kcp://") {
                (NetType::Kcp, stripped.to_string())
            } else if let Some(stripped) = trimmed_addr.strip_prefix("ws://") {
                (NetType::Ws, stripped.to_string())
            } else {
                panic!("Unsupported URL scheme: {}", trimmed_addr);
            }
        })
        .collect()
}
