use crate::net::WriterMessage;
use crate::proxy::crypto::EncryptionMethod;
use crate::proxy::{crypto, OutputFuncType, ProxyMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::yield_now;

const READ_BUF_MAX_LEN: usize = 1024 * 1024 * 2;

// 输入通道发送端类型
pub type InputSenderType = UnboundedSender<WriterMessage>;

#[derive(Clone)]
pub struct SessionCommonInfo {
    // 是否压缩数据
    pub is_compressed: bool,
    // 加密方法
    pub encryption_method: EncryptionMethod,
    // 加密key
    pub encryption_key: Vec<u8>,
    // 读缓存大小
    pub read_buf_len: Arc<RwLock<usize>>,
}

pub struct SessionInfo {
    pub sender: InputSenderType,
    pub common_info: SessionCommonInfo,
}

pub type SessionInfoMap = Arc<RwLock<HashMap<u32, SessionInfo>>>;

impl SessionCommonInfo {
    pub fn new(
        is_compressed: bool,
        encryption_method: EncryptionMethod,
        encryption_key: Vec<u8>,
    ) -> Self {
        Self {
            is_compressed,
            encryption_method,
            encryption_key,
            read_buf_len: Arc::new(RwLock::new(0)),
        }
    }

    pub fn from_method_name(is_compressed: bool, encryption_method: String) -> Self {
        let encryption_method = crypto::get_method(encryption_method.as_str());
        let encryption_key = crypto::generate_key(&encryption_method);
        Self::new(is_compressed, encryption_method, encryption_key)
    }

    pub async fn encode_data_and_limiting(&self, mut data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        if self.is_compressed {
            data = crypto::compress_data(data.as_slice())?;
        }

        if !self.encryption_method.is_none() {
            data = crypto::encrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }

        while *self.read_buf_len.read().await > READ_BUF_MAX_LEN {
            yield_now().await;
        }

        let mut read_buf_len_rw = self.read_buf_len.write().await;
        *read_buf_len_rw = *read_buf_len_rw + data.len();
        drop(read_buf_len_rw);

        Ok(data)
    }

    pub fn decode_data(&self, mut data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        if !self.encryption_method.is_none() {
            data = crypto::decrypt(
                &self.encryption_method,
                self.encryption_key.as_slice(),
                data.as_slice(),
            )?;
        }
        if self.is_compressed {
            data = crypto::decompress_data(data.as_slice())?;
        }

        Ok(data)
    }
}

pub async fn async_receive_output(
    mut output_rx: Receiver<ProxyMessage>,
    on_output_callback: OutputFuncType,
) {
    loop {
        if let Some(message) = output_rx.recv().await {
            on_output_callback(message).await;
        }
    }
}

// async fn async_receive_input(
//     mut input: UnboundedReceiver<ProxyMessage>,
//     output: Sender<ProxyMessage>,
//     session_info_map: SessionInfoMap,
// ) {
//     while let Some(message) = input.recv().await {
//         if let Err(err) = input_internal(message, &output, &session_info_map).await {
//             error!("inlet async_receive_input error: {}", err.to_string());
//         }
//     }
// }
//
//
// async fn input_internal(
//     message: ProxyMessage,
//     output: &Sender<ProxyMessage>,
//     session_info_map: &SessionInfoMap,
// ) -> anyhow::Result<()> {
//     match message {
//         //////////////////////////////////////////////////////// O2i ////////////////////////////////////////////////////////
//         ProxyMessage::O2iConnect(session_id, success, error_msg) => {
//             trace!(
//                     "O2iConnect: session_id:{session_id}, success:{success}, error_msg:{error_msg}"
//                 );
//             if !success {
//                 error!("connect error: {error_msg}");
//                 if let Some(session) = session_info_map.read().await.get(&session_id) {
//                     session.sender.send(WriterMessage::Close)?;
//                 }
//             }
//         }
//         ProxyMessage::O2iDisconnect(session_id) => {
//             trace!("O2iDisconnect: session_id:{session_id}");
//             if let Some(session) = session_info_map.read().await.get(&session_id) {
//                 session.sender.send(WriterMessage::Close)?;
//             }
//         }
//         ProxyMessage::O2iSendDataResult(session_id, data_len) => {
//             // trace!("O2iSendDataResult: session_id:{session_id}, data_len:{data_len}");
//             if let Some(session) = session_info_map.read().await.get(&session_id) {
//                 let mut read_buf_len = session.common_info.read_buf_len.write().await;
//                 if *read_buf_len <= data_len {
//                     *read_buf_len = 0;
//                 } else {
//                     *read_buf_len = *read_buf_len - data_len;
//                 }
//                 // trace!("O2iSendDataResult: session_id:{session_id}, data_len:{data_len}, read_buf_len:{}", *read_buf_len);
//                 drop(read_buf_len);
//             }
//         }
//         ProxyMessage::O2iRecvData(session_id, mut data) => {
//             // trace!("O2iRecvData: session_id:{session_id}");
//             let data_len = data.len();
//
//             if let Some(session) = session_info_map.read().await.get(&session_id) {
//                 data = session.common_info.decode_data(data)?;
//
//                 // 写入完毕回调
//                 let output = output.clone();
//                 let callback: SendMessageFuncType = Box::new(move || {
//                     let output = output.clone();
//                     Box::pin(async move {
//                         let _ = output
//                             .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
//                             .await;
//                     })
//                 });
//
//                 session
//                     .sender
//                     .send(WriterMessage::SendAndThen(data, callback))?;
//             } else {
//                 trace!("O2iRecvData: unknown session:{session_id}");
//             }
//         }
//
//
//         //////////////////////////////////////////////////////// I2o ////////////////////////////////////////////////////////
//         ProxyMessage::I2oConnect(
//             session_id,
//             is_tcp,
//             is_compressed,
//             addr,
//             encryption_method,
//             encryption_key,
//             client_addr,
//         ) => {
//             trace!("I2oConnect: session_id:{session_id}, addr:{addr}, is_tcp:{is_tcp}");
//             if let Err(err) = self
//                 .on_i2o_connect(
//                     session_id,
//                     is_tcp,
//                     is_compressed,
//                     addr.clone(),
//                     encryption_method,
//                     encryption_key,
//                 )
//                 .await
//             {
//                 error!(
//                         "Failed to connect to {}, error: {}, remote client addr {}",
//                         addr,
//                         err.to_string(),
//                         client_addr
//                     );
//
//                 self.output
//                     .send(ProxyMessage::O2iConnect(session_id, false, err.to_string()))
//                     .await?;
//             } else {
//                 info!(
//                         "Successfully connected to {}, remote client addr {}",
//                         addr, client_addr
//                     );
//                 self.output
//                     .send(ProxyMessage::O2iConnect(session_id, true, "".into()))
//                     .await?;
//             }
//         }
//         ProxyMessage::I2oSendData(session_id, data) => {
//             // trace!("I2oSendData: session_id:{session_id}");
//             self.on_i2o_send_data(session_id, data).await?;
//         }
//         ProxyMessage::I2oDisconnect(session_id) => {
//             trace!("I2oDisconnect: session_id:{session_id}");
//             self.on_i2o_disconnect(session_id).await?;
//         }
//         ProxyMessage::I2oRecvDataResult(session_id, data_len) => {
//             // trace!("I2oRecvDataResult: session_id:{session_id}, data_len:{data_len}");
//             self.on_i2o_recv_data_result(session_id, data_len).await?;
//         }
//     }
//
//     Ok(())
// }
