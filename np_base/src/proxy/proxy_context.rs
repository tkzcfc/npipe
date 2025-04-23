use crate::net::{SendMessageFuncType, WriterMessage};
use crate::proxy::common::SessionCommonInfo;
use crate::proxy::inlet::{InletDataEx, InletProxyType};
use crate::proxy::ProxyMessage;
use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, UnboundedSender};

// 通用的父类结构体
pub(crate) struct ProxyContextData {
    session_id: AtomicU32,

    pub(crate) inlet_proxy_type: InletProxyType,
    pub(crate) output_addr: String,
    pub(crate) output: Sender<ProxyMessage>,
    pub(crate) common_data: SessionCommonInfo,
    pub(crate) data_ex: Arc<InletDataEx>,
}

impl ProxyContextData {
    pub fn new(
        inlet_proxy_type: InletProxyType,
        output_addr: String,
        output: Sender<ProxyMessage>,
        common_data: SessionCommonInfo,
        data_ex: Arc<InletDataEx>,
    ) -> Self {
        Self {
            inlet_proxy_type,
            output_addr,
            session_id: AtomicU32::new(0),
            output,
            common_data,
            data_ex,
        }
    }

    pub fn get_session_id(&self) -> u32 {
        self.session_id.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set_session_id(&self, session_id: u32) {
        self.session_id
            .store(session_id, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait]
pub trait ProxyContext
where
    Self: Sync + Send,
{
    async fn on_start(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        peer_addr: SocketAddr,
        _write_to_peer_tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()>;

    async fn on_recv_peer_data(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        data: Vec<u8>,
    ) -> anyhow::Result<()>;

    async fn on_recv_proxy_message(&mut self, _proxy_message: ProxyMessage) -> anyhow::Result<()>;

    async fn on_stop(&mut self, ctx_data: Arc<ProxyContextData>) -> anyhow::Result<()> {
        ctx_data
            .output
            .send(ProxyMessage::I2oDisconnect(ctx_data.get_session_id()))
            .await?;
        Ok(())
    }

    fn is_ready_for_read(&self) -> bool;
}

pub(crate) struct UniversalProxy {
    is_connected: AtomicBool,
    write_to_peer_tx: Option<UnboundedSender<WriterMessage>>,
    ctx_data: Option<Arc<ProxyContextData>>,
}

#[async_trait]
impl ProxyContext for UniversalProxy {
    async fn on_start(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        peer_addr: SocketAddr,
        write_to_peer_tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        ctx_data
            .output
            .send(ProxyMessage::I2oConnect(
                ctx_data.get_session_id(),
                ctx_data.inlet_proxy_type.to_u8(),
                ctx_data.inlet_proxy_type.is_tcp(),
                ctx_data.common_data.is_compressed,
                ctx_data.output_addr.clone(),
                ctx_data.common_data.encryption_method.to_string(),
                BASE64_STANDARD.encode(&ctx_data.common_data.encryption_key),
                peer_addr.to_string(),
            ))
            .await?;
        self.write_to_peer_tx = Some(write_to_peer_tx);
        self.ctx_data = Some(ctx_data);
        Ok(())
    }

    async fn on_recv_peer_data(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        mut data: Vec<u8>,
    ) -> anyhow::Result<()> {
        data = ctx_data.common_data.encode_data_and_limiting(data).await?;
        ctx_data
            .output
            .send(ProxyMessage::I2oSendData(ctx_data.get_session_id(), data))
            .await?;
        Ok(())
    }
    async fn on_recv_proxy_message(&mut self, proxy_message: ProxyMessage) -> anyhow::Result<()> {
        match proxy_message {
            ProxyMessage::O2iConnect(_session_id, success, error_msg) => {
                if success {
                    self.is_connected.store(true, Ordering::Relaxed);
                } else {
                    error!("connect error: {error_msg}");
                    self.write_to_peer_tx
                        .as_ref()
                        .unwrap()
                        .send(WriterMessage::Close)?;
                }
            }
            ProxyMessage::O2iRecvData(session_id, mut data) => {
                let data_len = data.len();
                data = self
                    .ctx_data
                    .as_ref()
                    .unwrap()
                    .common_data
                    .decode_data(data)?;

                // 写入完毕回调
                let output = self.ctx_data.as_ref().unwrap().output.clone();
                let callback: SendMessageFuncType = Box::new(move || {
                    let output = output.clone();
                    Box::pin(async move {
                        let _ = output
                            .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
                            .await;
                    })
                });

                self.write_to_peer_tx
                    .as_ref()
                    .unwrap()
                    .send(WriterMessage::SendAndThen(data, callback))?;
            }
            ProxyMessage::O2iDisconnect(_) => {
                self.write_to_peer_tx
                    .as_ref()
                    .unwrap()
                    .send(WriterMessage::Close)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn is_ready_for_read(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }
}

impl UniversalProxy {
    pub(crate) fn new() -> Self {
        Self {
            is_connected: AtomicBool::new(false),
            write_to_peer_tx: None,
            ctx_data: None,
        }
    }
}
