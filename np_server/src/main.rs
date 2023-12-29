mod peer;
mod player;

use crate::peer::Peer;
use np_base::net::server;
use std::time::Duration;
use std::{env, io};
use tokio::signal;
use tokio::time::sleep;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let listener = server::bind("0.0.0.0:8118").await?;
    server::run_server(listener, || Box::new(Peer::new()), signal::ctrl_c()).await;
    loop {
        sleep(Duration::from_secs(1)).await;
        println!("wait...");
    }
}
