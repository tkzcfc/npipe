use std::net::SocketAddr;
use std::sync::{Arc};
use tokio::sync::RwLock;
use tokio::net::TcpStream;
use crate::session::Session;
use lazy_static::lazy_static;

pub struct SessionManager {
    sessions : Vec<Arc<RwLock<Session>>>,
}

lazy_static! {
    pub static ref SESSIONMANAGER: Arc<RwLock<SessionManager>> = Arc::new(RwLock::new(SessionManager::new()));
}

impl SessionManager {

    fn new() -> SessionManager {
        SessionManager{
            sessions: Vec::new()
        }
    }

    pub fn new_session(&mut self, socket: TcpStream, addr: SocketAddr) -> Arc<RwLock<Session>> {
        let session = Session::new(socket, addr);
        self.sessions.push(Arc::clone(&session));
        session
    }
}


