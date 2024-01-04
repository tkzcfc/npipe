mod log;
mod opts;
mod peer;
mod player;

use crate::log::init_log;
use crate::opts::SERVER_OPT;
use crate::peer::Peer;
use np_base::net::server;
use std::{env, io};
use tokio::net::TcpStream;
use tokio::signal;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", SERVER_OPT.log_level.clone());
    if SERVER_OPT.backtrace {
        env::set_var("RUST_BACKTRACE", "1");
    }

    init_log().expect("");

    let listener = server::bind("0.0.0.0:8118").await?;
    server::run_server(
        listener,
        || Box::new(Peer::new()),
        |stream: TcpStream| async move { Ok(stream) },
        signal::ctrl_c(),
    )
    .await;
    Ok(())
}
