/// 通用成功
///
/// @build_automatically_generate_message_id@  enum MsgId { None=0; Id = 150001; }
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Success {}
/// 通用返回失败
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Fail {
    /// @build_automatically_generate_message_id@  enum MsgId {  None=0;Id = 150002; }
    #[prost(int32, tag = "1")]
    pub number: i32,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
}
/// 通用错误返回
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Error {
    /// @build_automatically_generate_message_id@  enum MsgId {  None=0;Id = 150003; }
    #[prost(int32, tag = "1")]
    pub number: i32,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
}
/// ping
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ping {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150004;}
    #[prost(int64, tag = "1")]
    pub ticks: i64,
}
/// pong
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pong {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150005;}
    #[prost(int64, tag = "1")]
    pub ticks: i64,
}
/// 向输出端请求发起连接
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct I2oConnect {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150006;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
    /// 是否是TCP连接
    #[prost(bool, tag = "3")]
    pub is_tcp: bool,
    /// 是否压缩数据
    #[prost(bool, tag = "4")]
    pub is_compressed: bool,
    /// 目标地址
    #[prost(string, tag = "5")]
    pub addr: ::prost::alloc::string::String,
    /// 加密方式
    #[prost(string, tag = "6")]
    pub encryption_method: ::prost::alloc::string::String,
    /// 加密key
    #[prost(string, tag = "7")]
    pub encryption_key: ::prost::alloc::string::String,
    /// 客户端地址
    #[prost(string, tag = "8")]
    pub client_addr: ::prost::alloc::string::String,
}
/// 连接结果
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct O2iConnect {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150007;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
    /// 是否是成功
    #[prost(bool, tag = "3")]
    pub success: bool,
    /// 错误信息
    #[prost(string, tag = "4")]
    pub error_info: ::prost::alloc::string::String,
}
/// 输出端收到数据返回给输入端
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct I2oSendData {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150008;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
    /// 数据
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// 输出端收到数据返回给输入端
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct O2iRecvData {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150009;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
    /// 数据
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// 断开连接
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct I2oDisconnect {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150010;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
}
/// 断开连接
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct O2iDisconnect {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 150011;}
    /// 通道id
    #[prost(uint32, tag = "1")]
    pub tunnel_id: u32,
    /// 会话id
    #[prost(uint32, tag = "2")]
    pub session_id: u32,
}
/// 通用错误码
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ErrorCode {
    None = 0,
    /// 服务器内部错误
    InternalError = -1000,
    /// 请求协议Id不存在
    InterfaceAbsent = -1001,
    /// 玩家未登录
    PlayerNotLogin = -1002,
}
impl ErrorCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ErrorCode::None => "None",
            ErrorCode::InternalError => "InternalError",
            ErrorCode::InterfaceAbsent => "InterfaceAbsent",
            ErrorCode::PlayerNotLogin => "PlayerNotLogin",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "None" => Some(Self::None),
            "InternalError" => Some(Self::InternalError),
            "InterfaceAbsent" => Some(Self::InterfaceAbsent),
            "PlayerNotLogin" => Some(Self::PlayerNotLogin),
            _ => None,
        }
    }
}
