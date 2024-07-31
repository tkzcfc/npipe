/// 通道端点
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TunnelPoint {
    /// 地址
    #[prost(string, tag = "1")]
    pub addr: ::prost::alloc::string::String,
}
/// 通道
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Tunnel {
    /// 起点（入口）
    #[prost(message, optional, tag = "1")]
    pub source: ::core::option::Option<TunnelPoint>,
    /// 终点（出口）
    #[prost(message, optional, tag = "2")]
    pub endpoint: ::core::option::Option<TunnelPoint>,
    /// 通道id
    #[prost(uint32, tag = "3")]
    pub id: u32,
    /// 是否启用
    #[prost(bool, tag = "4")]
    pub enabled: bool,
    /// 发送方id
    #[prost(uint32, tag = "5")]
    pub sender: u32,
    /// 接收方id
    #[prost(uint32, tag = "6")]
    pub receiver: u32,
    /// 通道类型
    #[prost(enumeration = "TunnelType", tag = "7")]
    pub tunnel_type: i32,
    /// 密码
    #[prost(string, tag = "8")]
    pub password: ::prost::alloc::string::String,
    /// 用户名
    #[prost(string, tag = "9")]
    pub username: ::prost::alloc::string::String,
    /// 是否压缩数据
    #[prost(bool, tag = "10")]
    pub is_compressed: bool,
    /// 加密算法
    #[prost(string, tag = "11")]
    pub encryption_method: ::prost::alloc::string::String,
    /// 自定义域名映射关系
    #[prost(map = "string, string", tag = "12")]
    pub custom_mapping: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
/// 通道类型
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TunnelType {
    Tcp = 0,
    Udp = 1,
    Socks5 = 2,
    Unknown = 3,
}
impl TunnelType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TunnelType::Tcp => "TCP",
            TunnelType::Udp => "UDP",
            TunnelType::Socks5 => "SOCKS5",
            TunnelType::Unknown => "UNKNOWN",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "TCP" => Some(Self::Tcp),
            "UDP" => Some(Self::Udp),
            "SOCKS5" => Some(Self::Socks5),
            "UNKNOWN" => Some(Self::Unknown),
            _ => None,
        }
    }
}
