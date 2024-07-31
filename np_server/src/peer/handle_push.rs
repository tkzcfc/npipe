use super::Peer;
use crate::global::manager::proxy::ProxyManager;
use crate::global::manager::GLOBAL_MANAGER;
use np_proto::message_map::MessageType;
use np_proto::utils::message_bridge;

impl Peer {
    // 收到玩家向服务器推送消息
    pub(crate) async fn handle_push(&self, message: MessageType) -> anyhow::Result<()> {
        if let Some((msg, tunnel_id)) = message_bridge::pb_2_proxy_message(message) {
            if let Some(tunnel) = GLOBAL_MANAGER
                .tunnel_manager
                .read()
                .await
                .get_tunnel(tunnel_id)
            {
                let (from_player_id, to_player_id) = if message_bridge::is_i2o_message(&msg) {
                    (tunnel.receiver, tunnel.sender)
                } else {
                    (tunnel.sender, tunnel.receiver)
                };

                ProxyManager::send_proxy_message(from_player_id, to_player_id, tunnel.id, msg)
                    .await;
            }
        }
        Ok(())
    }
}
