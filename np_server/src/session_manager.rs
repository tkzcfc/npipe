use std::net::SocketAddr;
use std::sync::{Arc};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::net::TcpStream;
use crate::session::{Session, SessionStatus};
use lazy_static::lazy_static;
use tokio::time::sleep;

pub struct SessionManager {
    sessions : RwLock<Vec<Arc<RwLock<Session>>>>,
}

lazy_static! {
    pub static ref SESSIONMANAGER: Arc<RwLock<SessionManager>> = SessionManager::new();
}

impl SessionManager {

    fn new() -> Arc<RwLock<SessionManager>> {
        let instance = Arc::new(RwLock::new(SessionManager{
            sessions: RwLock::new(Vec::new())
        }));

        let instance_cloned = Arc::clone(&instance);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                instance_cloned.write().await.collect_garbage_session().await;
            }
        });

        instance
    }

    pub async fn new_session(&mut self, socket: TcpStream, addr: SocketAddr) -> Arc<RwLock<Session>> {
        let session = Session::new(socket, addr);
        self.sessions.write().await.push(Arc::clone(&session));
        session
    }

    pub async fn collect_garbage_session(&mut self) {
        let mut sessions = self.sessions.write().await;

        let mut i = 0;
        while i != sessions.len() {
            let item_cloned = sessions[i].clone();
            let v = item_cloned.read().await;
            match v.status() {
                SessionStatus::Disconnected => {
                    sessions.remove(i);
                }
                _=> {
                    i += 1;
                }
            }
        }
    }
}


