use super::Peer;
use crate::global::manager::proxy::ProxyManager;
use crate::global::manager::GLOBAL_MANAGER;
use np_base::proxy::ProxyMessage;
use np_proto::generic::{
    I2oConnect, I2oDisconnect, I2oSendData, O2iConnect, O2iDisconnect, O2iRecvData,
};
use np_proto::message_map::MessageType;

impl Peer {
    // 收到玩家向服务器推送消息
    pub(crate) async fn handle_push(&self, message: MessageType) -> anyhow::Result<()> {
        match message {
            MessageType::GenericI2oConnect(msg) => self.on_generic_i2o_connect(msg).await,
            MessageType::GenericO2iConnect(msg) => self.on_generic_o2i_connect(msg).await,
            MessageType::GenericI2oSendData(msg) => self.on_generic_i2o_send_data(msg).await,
            MessageType::GenericO2iRecvData(msg) => self.on_generic_o2i_recv_data(msg).await,
            MessageType::GenericI2oDisconnect(msg) => self.on_generic_i2o_disconnect(msg).await,
            MessageType::GenericO2iDisconnect(msg) => self.on_generic_o2i_disconnect(msg).await,
            _ => {}
        }

        Ok(())
    }

    async fn on_generic_i2o_connect(&self, msg: I2oConnect) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.receiver,
                tunnel.sender,
                tunnel.id,
                ProxyMessage::I2oConnect(
                    msg.session_id,
                    msg.is_tcp,
                    msg.is_compressed,
                    msg.addr,
                    msg.encryption_method,
                    msg.encryption_key,
                    msg.client_addr
                ),
            )
            .await;
        }
    }

    async fn on_generic_o2i_connect(&self, msg: O2iConnect) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.sender,
                tunnel.receiver,
                tunnel.id,
                ProxyMessage::O2iConnect(msg.session_id, msg.success, msg.error_info),
            )
            .await;
        }
    }

    async fn on_generic_i2o_send_data(&self, msg: I2oSendData) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.receiver,
                tunnel.sender,
                tunnel.id,
                ProxyMessage::I2oSendData(msg.session_id, msg.data),
            )
            .await;
        }
    }

    async fn on_generic_o2i_recv_data(&self, msg: O2iRecvData) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.sender,
                tunnel.receiver,
                tunnel.id,
                ProxyMessage::O2iRecvData(msg.session_id, msg.data),
            )
            .await;
        }
    }

    async fn on_generic_i2o_disconnect(&self, msg: I2oDisconnect) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.receiver,
                tunnel.sender,
                tunnel.id,
                ProxyMessage::I2oDisconnect(msg.session_id),
            )
            .await;
        }
    }

    async fn on_generic_o2i_disconnect(&self, msg: O2iDisconnect) {
        if let Some(tunnel) = GLOBAL_MANAGER
            .tunnel_manager
            .read()
            .await
            .get_tunnel(msg.tunnel_id)
        {
            ProxyManager::send_proxy_message(
                tunnel.sender,
                tunnel.receiver,
                tunnel.id,
                ProxyMessage::O2iDisconnect(msg.session_id),
            )
            .await;
        }
    }
}
