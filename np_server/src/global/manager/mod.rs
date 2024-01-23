
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use self::player::PlayerManager;


pub mod player;

pub struct GlobalManager {
    pub player_manager: RwLock<PlayerManager>,
}

impl GlobalManager {
    fn new() -> Self {
        Self {
            player_manager: RwLock::new(PlayerManager::new())
        }
    }
}


pub static GLOBAL_MANAGER: Lazy<GlobalManager> =
    Lazy::new(|| GlobalManager::new());