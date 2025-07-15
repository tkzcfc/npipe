#[derive(Debug)]
pub enum NetType {
    Tcp,
    Kcp,
    Ws,
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
