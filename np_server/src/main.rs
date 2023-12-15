mod player;
mod player_manager;
mod session;
mod session_logic;
mod session_manager;

use crate::session_manager::SESSIONMANAGER;
use log::debug;
use std::{env, io};
use tokio::net::TcpListener;

async fn run_server() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2000").await?;
    loop {
        let (socket, addr) = listener.accept().await?;

        debug!("new connection: {}", addr);
        let session = SESSIONMANAGER.write().await.new_session(socket, addr).await;
        // 新连接单独起一个异步任务处理
        tokio::spawn(async move {
            session.write().await.read_poll().await;
        });
    }
}

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    run_server().await
}
