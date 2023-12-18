use crate::player::{Player, PlayerId};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PlayerManager {
    players: Vec<Arc<RwLock<Player>>>,
    player_map: HashMap<PlayerId, Arc<RwLock<Player>>>,
}

lazy_static! {
    pub static ref PLAYERNMANAGER: Arc<RwLock<PlayerManager>> = PlayerManager::new();
}

impl PlayerManager {
    fn new() -> Arc<RwLock<PlayerManager>> {
        Arc::new(RwLock::new(PlayerManager {
            players: Vec::new(),
            player_map: HashMap::new(),
        }))
    }

    async fn get_player(&self, player_id: u32) -> Option<Arc<RwLock<Player>>> {
        if let Some(player) = self.player_map.get(&player_id) {
            Some(player.clone())
        } else {
            None
        }
    }

    async fn create_player(&mut self) -> Result<Arc<RwLock<Player>>, std::io::Error> {
        let player = Player::new(0);
        self.players.push(player.clone());
        self.player_map
            .insert(player.read().await.get_player_id(), player.clone());
        Ok(player)
    }
}
