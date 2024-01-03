use crate::player::{Player, PlayerId};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref PLAYER_MANAGER: RwLock<PlayerManager> = RwLock::new(PlayerManager::new());
}

pub struct PlayerManager {
    players: Vec<Arc<RwLock<Player>>>,
    player_map: HashMap<PlayerId, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    fn new() -> PlayerManager {
        PlayerManager {
            players: Vec::new(),
            player_map: HashMap::new(),
        }
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
    ) -> anyhow::Result<Arc<RwLock<Player>>> {
        let player = Player::new(player_id);
        self.players.push(player.clone());
        self.player_map
            .insert(player.read().await.get_player_id(), player.clone());
        Ok(player)
    }
}
