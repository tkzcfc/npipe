/// 登录回复
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginAck {
    ///   enum MsgId {None = 0; Id = 1002;}
    #[prost(int32, tag = "1")]
    pub code: i32,
}
