/// 登录回复
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginAck {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1002;}
    /// 错误码 0成功
    #[prost(int32, tag = "1")]
    pub code: i32,
    /// 自己的玩家id
    #[prost(uint32, tag = "2")]
    pub player_id: u32,
    /// 通道列表
    #[prost(message, repeated, tag = "3")]
    pub channel_list: ::prost::alloc::vec::Vec<super::class_def::Channel>,
}
/// 管理员登录回复
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ManagementLoginAck {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1006;}
    /// 错误码 0成功
    #[prost(int32, tag = "1")]
    pub code: i32,
}
/// 修改通道通知
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModifyChannelNtf {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1008;}
    /// 是否是删除通道，如果不是则表示新增或更新通道信息
    #[prost(bool, tag = "1")]
    pub is_delete: bool,
    /// 通道信息
    #[prost(message, optional, tag = "2")]
    pub channel: ::core::option::Option<super::class_def::Channel>,
}
/// 向通道发送消息
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendMessageToChannel {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1010;}
    /// 通道id
    #[prost(int32, tag = "1")]
    pub id: i32,
    /// 数据
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// 从通道收到消息
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RecvMessageFromChannel {
    /// @build_automatically_generate_message_id@  enum MsgId {None = 0; Id = 1012;}
    /// 通道id
    #[prost(int32, tag = "1")]
    pub id: i32,
    /// 数据
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
