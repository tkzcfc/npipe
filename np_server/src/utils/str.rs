use std::net::SocketAddr;

/// 是否只包含ASCII码并且不包含空格
pub fn is_ascii_nospace(s: &str) -> bool {
    s.is_ascii() && !s.contains(' ')
}

/// 是否是有效的用户名
pub fn is_valid_username(s: &str) -> bool {
    s.len() >= 5 && s.len() <= 30 && is_ascii_nospace(s)
}

/// 是否是有效的密码
pub fn is_valid_password(s: &str) -> bool {
    s.len() >= 5 && s.len() <= 15 && is_ascii_nospace(s)
}

/// 是否是有效的域名
pub fn is_valid_domain(domain: &str) -> bool {
    let parts: Vec<&str> = domain.split('.').collect();

    // 域名应至少有两部分（二级域名和顶级域名）
    if parts.len() < 2 {
        return false;
    }

    for part in &parts {
        // 每一部分必须是非空的
        if part.is_empty() {
            return false;
        }

        // 每一部分只能包含字母、数字和连字符
        if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }

        // 连字符不能在部分的开头或结尾
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }
    }

    true
}

/// 是否是有效的隧道入口地址
pub fn is_valid_tunnel_source_address(addr: &str) -> bool {
    addr.parse::<SocketAddr>().is_ok()
}

/// 是否是有效的隧道出口地址
pub fn is_valid_tunnel_endpoint_address(addr: &str) -> bool {
    if addr.parse::<SocketAddr>().is_ok() {
        true
    } else {
        let s: Vec<&str> = addr.split(":").collect();
        if s.len() == 2 && is_valid_domain(s[0]) && s[1].parse::<u16>().is_ok() {
            return true;
        }
        false
    }
}

/// 获取隧道端口
pub fn get_tunnel_address_port(addr: &str) -> Option<u16> {
    let s: Vec<&str> = addr.split(":").collect();
    if s.len() == 2 {
        return if let Ok(value) = s[1].parse::<u16>() {
            Some(value)
        } else {
            None
        }
    }
    None
}
