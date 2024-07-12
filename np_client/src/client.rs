use crate::Opts;
use tokio::net::{TcpStream};

pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&self, opts: &Opts) -> anyhow::Result<()> {
        let stream = TcpStream::connect(&opts.server_addr).await?;
        println!("connect succc");
        Ok(())
    }
}
