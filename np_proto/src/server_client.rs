/// 登录回复
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginAck {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1002;}
    /// 错误码 0成功
    #[prost(int32, tag = "1")]
    pub code: i32,
}
/// 注册回复
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterAck {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1004;}
    /// 错误码 0成功
    #[prost(int32, tag = "1")]
    pub code: i32,
}
