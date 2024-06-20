use crate::net::tcp_session::WriterMessage;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

// 输出函数类型
pub type OutputFuncType =
    Arc<dyn Fn(WriterMessage) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;
// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;
// 发送通道集合
pub(crate) type SenderMap = Arc<Mutex<HashMap<u32, InputSenderType>>>;
