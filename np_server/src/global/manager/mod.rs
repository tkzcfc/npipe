use self::player::PlayerManager;
use self::proxy::ProxyManager;
use self::tunnel::TunnelManager;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

pub mod player;
pub mod proxy;
pub mod tunnel;

pub struct GlobalManager {
    pub player_manager: RwLock<PlayerManager>,
    pub tunnel_manager: RwLock<TunnelManager>,
    pub proxy_manager: RwLock<ProxyManager>,
}

impl GlobalManager {
    fn new() -> Self {
        Self {
            player_manager: RwLock::new(PlayerManager::new()),
            tunnel_manager: RwLock::new(TunnelManager::new()),
            proxy_manager: RwLock::new(ProxyManager::new()),
        }
    }
}

pub static GLOBAL_MANAGER: Lazy<GlobalManager> = Lazy::new(|| GlobalManager::new());
