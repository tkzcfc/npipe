use crate::player::{Player, PlayerId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PlayerManager {
    players: Vec<Arc<RwLock<Player>>>,
    player_map: HashMap<PlayerId, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    pub fn new() -> RwLock<PlayerManager> {
        RwLock::new(PlayerManager {
            players: Vec::new(),
            player_map: HashMap::new(),
        })
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
    ) -> Result<Arc<RwLock<Player>>, std::io::Error> {
        let player = Player::new(player_id);
        self.players.push(player.clone());
        self.player_map
            .insert(player.read().await.get_player_id(), player.clone());
        Ok(player)
    }
}
