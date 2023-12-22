mod player;
mod player_manager;
mod server;
mod session;
mod session_logic;
use std::{env, io};


#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    server::run("0.0.0.0:8118").await
}
