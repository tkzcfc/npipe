use crate::net::session::Session;
use crate::net::session_logic::SessionLogic;
use log::{info, trace};
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;

pub type CreateSessionLogicCallback = fn() -> Box<dyn SessionLogic>;

pub async fn run_server(
    addr: &str,
    on_create_session_logic_callback: CreateSessionLogicCallback,
) -> io::Result<()> {
    let addr = addr.parse::<SocketAddr>();
    match addr {
        Err(parse_error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                parse_error.to_string(),
            ));
        }
        _ => {}
    }

    let addr = addr.unwrap();
    info!("Start listening: {}", addr);

    let listener = TcpListener::bind(addr).await?;
    let mut session_id_seed = 0;
    loop {
        let (socket, addr) = listener.accept().await?;

        if session_id_seed >= u32::MAX {
            session_id_seed = 0;
        }
        session_id_seed += 1;

        // const SEND_BUFFER_SIZE: usize = 262144;
        // const RECV_BUFFER_SIZE: usize = SEND_BUFFER_SIZE * 2;

        let session_id = session_id_seed;
        let logic = on_create_session_logic_callback();

        // 新连接单独起一个异步任务处理
        tokio::spawn(async move {
            trace!("new connection: {}", addr);

            let (tx, rx) = unbounded_channel();
            let (reader, writer) = tokio::io::split(socket);

            let mut session = Session::new(tx.clone(), addr, session_id, logic);
            session.run(rx, reader, writer).await;

            trace!("disconnect: {}", addr);
        });
    }
}
