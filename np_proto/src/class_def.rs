/// 通道端点
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChannelPoint {
    /// 地址
    #[prost(string, tag = "1")]
    pub addr: ::prost::alloc::string::String,
}
/// 通道
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Channel {
    /// 起点（入口）
    #[prost(message, optional, tag = "1")]
    pub source: ::core::option::Option<ChannelPoint>,
    /// 终点（出口）
    #[prost(message, optional, tag = "2")]
    pub endpoint: ::core::option::Option<ChannelPoint>,
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
}
