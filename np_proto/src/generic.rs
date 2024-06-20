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
