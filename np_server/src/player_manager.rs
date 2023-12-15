use crate::player::Player;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PlayerManager {
    players: RwLock<Vec<Arc<RwLock<Player>>>>,
}

lazy_static! {
    pub static ref PLAYERNMANAGER: Arc<RwLock<PlayerManager>> = PlayerManager::new();
}

impl PlayerManager {
    fn new() -> Arc<RwLock<PlayerManager>> {
        Arc::new(RwLock::new(PlayerManager {
            players: RwLock::new(Vec::new()),
        }))
    }

    async fn get_or_create_player(player_id: u32) -> Arc<RwLock<Player>> {
        let player = Arc::new(RwLock::new(Player::new()));

        player
    }
}
