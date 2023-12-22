mod player;
mod player_manager;
mod server;
mod session;
mod session_logic;

use crate::server::Server;
use crate::session::Session;
use log::trace;
use std::{env, io};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;

async fn run_server() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8118").await?;
    loop {
        let (socket, addr) = listener.accept().await?;

        // const SEND_BUFFER_SIZE: usize = 262144;
        // const RECV_BUFFER_SIZE: usize = SEND_BUFFER_SIZE * 2;

        // // 新连接单独起一个异步任务处理
        tokio::spawn(async move {
            trace!("new connection: {}", addr);

            let (tx, rx) = unbounded_channel();
            let (reader, mut writer) = tokio::io::split(socket);

            let mut session = Session::new(tx.clone(), addr, Server::instance().new_id());
            session.run(rx, reader, writer).await;

            trace!("disconnect: {}", addr);
        });
    }
}

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    run_server().await
}
