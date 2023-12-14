/// 登录请求
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginReq {
    /// 用户名
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// 密码
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
/// Nested message and enum types in `LoginReq`.
pub mod login_req {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum MsgId {
        None = 0,
        Id = 1001,
    }
    impl MsgId {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                MsgId::None => "None",
                MsgId::Id => "Id",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "None" => Some(Self::None),
                "Id" => Some(Self::Id),
                _ => None,
            }
        }
    }
}
/// 登录回复
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginAck {
    #[prost(int32, tag = "1")]
    pub code: i32,
}
/// Nested message and enum types in `LoginAck`.
pub mod login_ack {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum MsgId {
        None = 0,
        Id = 1002,
    }
    impl MsgId {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                MsgId::None => "None",
                MsgId::Id => "Id",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "None" => Some(Self::None),
                "Id" => Some(Self::Id),
                _ => None,
            }
        }
    }
}
