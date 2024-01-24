use self::player::PlayerManager;
use crate::global::manager::channel::ChannelManager;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

pub mod channel;
pub mod player;

pub struct GlobalManager {
    pub player_manager: RwLock<PlayerManager>,
    pub channel_manager: RwLock<ChannelManager>,
}

impl GlobalManager {
    fn new() -> Self {
        Self {
            player_manager: RwLock::new(PlayerManager::new()),
            channel_manager: RwLock::new(ChannelManager::new()),
        }
    }
}

pub static GLOBAL_MANAGER: Lazy<GlobalManager> = Lazy::new(|| GlobalManager::new());
