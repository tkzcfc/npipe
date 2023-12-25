mod player;
mod peer;

use std::{env, io};
use np_base::net::server::run_server;
use crate::peer::Peer;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    run_server("0.0.0.0:8118", || {
        Box::new(Peer::new())
    }).await
}
