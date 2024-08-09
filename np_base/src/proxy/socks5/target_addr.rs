use crate::proxy::socks5::{
    SOCKS5_ADDR_TYPE_DOMAIN_NAME, SOCKS5_ADDR_TYPE_IPV4, SOCKS5_ADDR_TYPE_IPV6,
};
use anyhow::anyhow;
use log::debug;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::vec::IntoIter;
use std::{fmt, io};

/// A description of a connection target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetAddr {
    /// Connect to an IP address.
    Ip(SocketAddr),
    /// Connect to a fully qualified domain name.
    ///
    /// The domain name will be passed along to the proxy server and DNS lookup
    /// will happen there.
    Domain(String, u16),
}

impl TargetAddr {
    pub fn is_ip(&self) -> bool {
        match self {
            TargetAddr::Ip(_) => true,
            _ => false,
        }
    }

    pub fn is_domain(&self) -> bool {
        !self.is_ip()
    }

    pub fn to_be_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        match self {
            TargetAddr::Ip(SocketAddr::V4(addr)) => {
                debug!("TargetAddr::IpV4");

                buf.extend_from_slice(&[SOCKS5_ADDR_TYPE_IPV4]);

                debug!("addr ip {:?}", (*addr.ip()).octets());
                buf.extend_from_slice(&(addr.ip()).octets()); // ip
                buf.extend_from_slice(&addr.port().to_be_bytes()); // port
            }
            TargetAddr::Ip(SocketAddr::V6(addr)) => {
                debug!("TargetAddr::IpV6");
                buf.extend_from_slice(&[SOCKS5_ADDR_TYPE_IPV6]);

                debug!("addr ip {:?}", (*addr.ip()).octets());
                buf.extend_from_slice(&(addr.ip()).octets()); // ip
                buf.extend_from_slice(&addr.port().to_be_bytes()); // port
            }
            TargetAddr::Domain(ref domain, port) => {
                debug!("TargetAddr::Domain");
                if domain.len() > u8::MAX as usize {
                    return Err(anyhow!("Maximum field length exceeded"));
                }
                buf.extend_from_slice(&[SOCKS5_ADDR_TYPE_DOMAIN_NAME, domain.len() as u8]);
                buf.extend_from_slice(domain.as_bytes()); // domain content
                buf.extend_from_slice(&port.to_be_bytes());
                // port content (.to_be_bytes() convert from u16 to u8 type)
            }
        }
        Ok(buf)
    }
}

// async-std ToSocketAddrs doesn't supports external trait implementation
// @see https://github.com/async-rs/async-std/issues/539
impl std::net::ToSocketAddrs for TargetAddr {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
        match *self {
            TargetAddr::Ip(addr) => Ok(vec![addr].into_iter()),
            TargetAddr::Domain(_, _) => Err(io::Error::new(
                io::ErrorKind::Other,
                "Domain name has to be explicitly resolved, please use TargetAddr::resolve_dns().",
            )),
        }
    }
}

impl fmt::Display for TargetAddr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TargetAddr::Ip(ref addr) => write!(f, "{}", addr),
            TargetAddr::Domain(ref addr, ref port) => write!(f, "{}:{}", addr, port),
        }
    }
}

/// A trait for objects that can be converted to `TargetAddr`.
pub trait ToTargetAddr {
    /// Converts the value of `self` to a `TargetAddr`.
    fn to_target_addr(&self) -> io::Result<TargetAddr>;
}

impl<'a> ToTargetAddr for (&'a str, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        // try to parse as an IP first
        if let Ok(addr) = self.0.parse::<Ipv4Addr>() {
            return (addr, self.1).to_target_addr();
        }

        if let Ok(addr) = self.0.parse::<Ipv6Addr>() {
            return (addr, self.1).to_target_addr();
        }

        Ok(TargetAddr::Domain(self.0.to_owned(), self.1))
    }
}

impl ToTargetAddr for SocketAddr {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        Ok(TargetAddr::Ip(*self))
    }
}

impl ToTargetAddr for SocketAddrV4 {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddr::V4(*self).to_target_addr()
    }
}

impl ToTargetAddr for SocketAddrV6 {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddr::V6(*self).to_target_addr()
    }
}

impl ToTargetAddr for (Ipv4Addr, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddrV4::new(self.0, self.1).to_target_addr()
    }
}

impl ToTargetAddr for (Ipv6Addr, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddrV6::new(self.0, self.1, 0, 0).to_target_addr()
    }
}

#[derive(Debug)]
pub enum Addr {
    V4([u8; 4]),
    V6([u8; 16]),
    Domain(String), // Vec<[u8]> or Box<[u8]> or String ?
    Unknown,
}

/// This function is used by the client & the server
pub fn read_address(data: &[u8], atyp: u8) -> anyhow::Result<Option<(TargetAddr, usize)>> {
    let addr_data_len: usize;
    let addr = match atyp {
        SOCKS5_ADDR_TYPE_IPV4 => {
            addr_data_len = 6;
            if data.len() < addr_data_len {
                Addr::Unknown
            } else {
                debug!("Address type `IPv4`");
                let array: [u8; 4] = data[0..4].try_into().expect("Slice with incorrect length");
                Addr::V4(array)
            }
        }
        SOCKS5_ADDR_TYPE_IPV6 => {
            addr_data_len = 18;
            if data.len() < addr_data_len {
                Addr::Unknown
            } else {
                debug!("Address type `IPv6`");
                let array: [u8; 16] = data[0..16].try_into().expect("Slice with incorrect length");
                Addr::V6(array)
            }
        }
        SOCKS5_ADDR_TYPE_DOMAIN_NAME => {
            debug!("Address type `domain`");
            let len = data[0] as usize + 1;
            addr_data_len = 2 + len;

            if data.len() < addr_data_len {
                Addr::Unknown
            } else {
                let domain = data[1..len]
                    .try_into()
                    .expect("Slice with incorrect length");
                // make sure the bytes are correct utf8 string
                let domain = String::from_utf8(domain)?;
                Addr::Domain(domain)
            }
        }
        _ => return Err(anyhow!("incorrect address type")),
    };

    match addr {
        Addr::Unknown => {
            return Ok(None);
        }
        _ => {}
    }

    // Convert (u8 * 2) into u16
    let port = (data[addr_data_len - 2] as u16) << 8 | data[addr_data_len - 1] as u16;

    // Merge ADDRESS + PORT into a TargetAddr
    let addr = match addr {
        Addr::V4([a, b, c, d]) => (Ipv4Addr::new(a, b, c, d), port).to_target_addr()?,
        Addr::V6(x) => (Ipv6Addr::from(x), port).to_target_addr()?,
        Addr::Domain(domain) => TargetAddr::Domain(domain, port),
        _ => panic!("Unknown"),
    };

    Ok(Some((addr, addr_data_len)))
}
