use crate::net::WriterMessage;
use crate::proxy::common::SessionCommonInfo;
use crate::proxy::inlet::{InletDataEx, InletProxyType};
use crate::proxy::ProxyMessage;
use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
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

    fn is_recv_proxy_message(&self) -> bool {
        false
    }

    async fn on_recv_proxy_message(&mut self, _proxy_message: ProxyMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_stop(&mut self, ctx_data: Arc<ProxyContextData>) -> anyhow::Result<()> {
        ctx_data
            .output
            .send(ProxyMessage::I2oDisconnect(ctx_data.get_session_id()))
            .await?;
        Ok(())
    }

    fn is_ready_for_read(&self) -> bool {
        true
    }
}

pub(crate) struct UniversalProxy {}

#[async_trait]
impl ProxyContext for UniversalProxy {}

impl UniversalProxy {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
