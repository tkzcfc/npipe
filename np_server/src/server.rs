use crate::player_manager::PlayerManager;
use std::{io, mem};
use std::net::SocketAddr;
use log::{info, trace};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::mpsc::unbounded_channel;
use crate::session::Session;

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
                Option::Some(ref mut server) => *server,
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


async fn run_server(addr: &str) -> io::Result<()> {
    let addr = addr.parse::<SocketAddr>();
    match addr {
        Err(parse_error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                parse_error.to_string(),
            ));
        }
        _ => {}
    }

    let addr = addr.unwrap();
    info!("Start listening: {}", addr);

    let listener = TcpListener::bind(addr).await?;
    loop {
        let (socket, addr) = listener.accept().await?;

        // const SEND_BUFFER_SIZE: usize = 262144;
        // const RECV_BUFFER_SIZE: usize = SEND_BUFFER_SIZE * 2;

        // // 新连接单独起一个异步任务处理
        tokio::spawn(async move {
            trace!("new connection: {}", addr);

            let (tx, rx) = unbounded_channel();
            let (reader, writer) = tokio::io::split(socket);

            let mut session = Session::new(tx.clone(), addr, Server::instance().new_id());
            session.run(rx, reader, writer).await;

            trace!("disconnect: {}", addr);
        });
    }
}

pub async fn run(addr: &str) -> io::Result<()> {
    Server::instance();
    run_server(addr).await?;
    Server::destroy();
    Ok(())
}