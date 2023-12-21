use std::mem;
use tokio::sync::{Mutex, RwLock};
use crate::player_manager::PlayerManager;

pub struct Server {
    pub id_seed: Mutex<u32>,
    pub player_manager: RwLock<PlayerManager>,
}

static mut SERVER_INSTANCE: Option<&'static mut Server> = Option::None;

impl Drop for Server {
    fn drop(&mut self) {}
}

impl Server {
    pub fn instance() -> &'static mut Server {
        unsafe {
            match SERVER_INSTANCE {
                Option::Some(ref mut Server) => *Server,
                Option::None => {
                    // 如果不存在，先创建新的实例，然后返回
                    let server_box = Box::new(Server {
                        id_seed: Mutex::new(0u32),
                        player_manager: PlayerManager::new(),
                    });
                    let server_ptr = Box::into_raw(server_box);
                    SERVER_INSTANCE = Some(&mut *server_ptr);
                    &mut *server_ptr
                }
            }
        }
    }

    pub fn destroy() {
        unsafe {
            if let Some(raw) = mem::replace(&mut SERVER_INSTANCE, None) {
                let server = Box::from_raw(raw);
                drop(server);
            }
        }
    }

    pub fn new_id(&mut self) -> u32 {
        let seed = self.id_seed.get_mut();
        if *seed >= u32::MAX {
            *seed = 0;
        }
        *seed = *seed + 1;
        *seed
    }
}
