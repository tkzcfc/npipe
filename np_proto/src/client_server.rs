/// 登录请求
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginReq {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1001;}
    /// 用户名
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// 密码
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
/// 注册请求
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterReq {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1003;}
    /// 用户名
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// 密码
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
/// 管理员登录
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ManagementLoginReq {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1005;}
    /// 用户名
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// 密码
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
