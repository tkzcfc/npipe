use super::Peer;
use crate::global::manager::proxy::ProxyManager;
use crate::global::manager::GLOBAL_MANAGER;
use np_proto::message_map::{get_message_size, MessageType};
use np_proto::utils::message_bridge;
use std::sync::atomic::Ordering;

impl Peer {
    // 收到玩家向服务器推送消息
    pub(crate) async fn handle_push(&self, message: MessageType) -> anyhow::Result<()> {
        // 在 message 被 move 之前计算代理流量大小
        let proxy_bytes = get_message_size(&message) as u64 + 13;

        if let Some((msg, tunnel_id)) = message_bridge::pb_2_proxy_message(message) {
            // 统计入站代理流量
            if let Some(ref rx) = self.traffic_rx {
                rx.fetch_add(proxy_bytes, Ordering::Relaxed);
            }
            // 先提取需要的数据，立即 drop 读锁，再调用 send_proxy_message
            // 原代码在持有 tunnels.read() Guard 时调用 send_proxy_message，该函数内部
            // 有 player_manager 查找和 await，导致读锁被跨 await 持有。
            let found = {
                let guard = GLOBAL_MANAGER.tunnel_manager.tunnels.read().await;
                guard.iter().find(|x| x.id == tunnel_id).map(|tunnel| {
                    let (from, to) = if message_bridge::is_i2o_message(&msg) {
                        (tunnel.receiver, tunnel.sender)
                    } else {
                        (tunnel.sender, tunnel.receiver)
                    };
                    (from, to, tunnel.id)
                })
            }; // ← 读锁在此 drop

            if let Some((from_player_id, to_player_id, id)) = found {
                ProxyManager::send_proxy_message(from_player_id, to_player_id, id, msg).await;
            }
        }
        Ok(())
    }
}
