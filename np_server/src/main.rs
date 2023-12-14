mod session;
mod session_manager;

use std::{env, io};
use log::{debug};
use tokio::net::TcpListener;
use crate::session_manager::SESSIONMANAGER;

async fn run_server()-> io::Result<()> {
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
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    run_server().await
}
