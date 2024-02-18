use self::player::PlayerManager;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use self::tunnel::TunnelManager;

pub mod tunnel;
pub mod player;

pub struct GlobalManager {
    pub player_manager: RwLock<PlayerManager>,
    pub tunnel_manager: RwLock<TunnelManager>,
}

impl GlobalManager {
    fn new() -> Self {
        Self {
            player_manager: RwLock::new(PlayerManager::new()),
            tunnel_manager: RwLock::new(TunnelManager::new()),
        }
    }
}

pub static GLOBAL_MANAGER: Lazy<GlobalManager> = Lazy::new(|| GlobalManager::new());
