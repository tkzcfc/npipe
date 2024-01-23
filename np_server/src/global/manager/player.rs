use crate::player::{Player, PlayerId};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PlayerManager {
    players: Vec<Arc<RwLock<Player>>>,
    player_map: HashMap<PlayerId, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    pub(crate) fn new() -> PlayerManager {
        PlayerManager {
            players: Vec::new(),
            player_map: HashMap::new(),
        }
    }

    pub async fn load_all_player(&mut self) -> anyhow::Result<()> {
        let query = "SELECT id, type FROM user";
        let rows = sqlx::query(query)
            .fetch_all(crate::global::GLOBAL_DB_POOL.get().unwrap())
            .await?;

        for row in rows {
            let id: u32 = row.get("id");
            let r#type: u8 = row.get("type");
            self.create_player(id, r#type).await;
        }

        Ok(())
    }

    pub fn contain(&self, player_id: PlayerId) -> bool {
        self.player_map.get(&player_id).is_some()
    }

    pub fn get_player(&self, player_id: PlayerId) -> Option<Arc<RwLock<Player>>> {
        if let Some(player) = self.player_map.get(&player_id) {
            Some(player.clone())
        } else {
            None
        }
    }

    pub async fn create_player(
        &mut self,
        player_id: PlayerId,
        player_type: u8,
    ) -> Arc<RwLock<Player>> {
        let player = Player::new(player_id, player_type);
        self.players.push(player.clone());
        self.player_map
            .insert(player.read().await.get_player_id(), player.clone());
        player
    }
}
