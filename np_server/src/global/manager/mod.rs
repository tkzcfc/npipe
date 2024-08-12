use self::player::PlayerManager;
use self::proxy::ProxyManager;
use self::tunnel::TunnelManager;
use once_cell::sync::Lazy;

pub mod player;
pub mod proxy;
pub mod tunnel;

pub struct GlobalManager {
    pub player_manager: PlayerManager,
    pub tunnel_manager: TunnelManager,
    pub proxy_manager: ProxyManager,
}

impl GlobalManager {
    fn new() -> Self {
        Self {
            player_manager: PlayerManager::new(),
            tunnel_manager: TunnelManager::new(),
            proxy_manager: ProxyManager::new(),
        }
    }
}

pub static GLOBAL_MANAGER: Lazy<GlobalManager> = Lazy::new(|| GlobalManager::new());
