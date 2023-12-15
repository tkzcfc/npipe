/// 登录请求
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginReq {
    ///   enum MsgId {None = 0; Id = 1001;}
    /// 用户名
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// 密码
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
