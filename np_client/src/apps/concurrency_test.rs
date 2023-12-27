use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use log::error;
use std::sync::mpsc::{channel, Receiver, Sender};
use byteorder::BigEndian;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::time::Instant;
use np_proto::generic;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use crate::apps::rpc_client::RpcClient;
use crate::tokio_runtime;

struct TestResult {
    qps: u32,
}


#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ConcurrencyTest {
    #[serde(skip)]
    results :Vec<TestResult>,
    #[serde(skip)]
    rx: Option<Receiver<(u32)>>,

    host: String,
    port: u16,
    concurrent_quantity: u32,
    use_raw: bool,
}

impl Default for ConcurrencyTest {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            rx: None,
            host: "127.0.0.1".into(),
            port: 8118,
            concurrent_quantity: 100,
            use_raw: true,
        }
    }
}

async fn do_test(tx: Sender<u32>, addr: SocketAddr) {
    let mut rpc = RpcClient::new(addr);

    let start = Instant::now();
    if let Err(error) = rpc.connect().await {
        error!("connect error: {}", error.to_string());
        tx.send(0).unwrap();
        return;
    }

    let mut qps = 0u32;
    // 10超时等待时间
    while Instant::now().duration_since(start) < Duration::from_secs(1) {
        let result = rpc.send_request(MessageType::GenericPing(generic::Ping{
            ticks: 0,
        })).await;

        match result {
            Err(error) => {
                println!("{}", error.to_string());
            },
            _ => {
                qps += 1;
            }
        }
    }
    rpc.disconnect();

    // println!("qps:{}", qps);
    tx.send(qps).unwrap();
}


async fn do_test_raw(tx: Sender<u32>, addr: SocketAddr) -> io::Result<()> {

    let message = MessageType::GenericPing(generic::Ping{
        ticks: 0,
    });
    let message_id = get_message_id(&message).unwrap();
    let message_size = get_message_size(&message);
    let mut buf = Vec::with_capacity(message_size + 12);

    byteorder::WriteBytesExt::write_u32::<BigEndian>(
        &mut buf,
        (8 + message_size) as u32,
    )?;
    byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, -1)?;
    byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
    encode_raw_message(&message, &mut buf);


    let mut buffer = BytesMut::with_capacity(1024);

    let mut qps = 0u32;
    let start = Instant::now();

    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    let mut stream = socket.connect(addr).await?;
    let duration = Duration::from_secs(1);
    while Instant::now().duration_since(start) < duration {
        stream.write_all(&buf).await?;
        stream.flush().await?;

        buffer.clear();
        loop {
            stream.read_buf(&mut buffer).await?;
            if buffer.len() >= 11 || Instant::now().duration_since(start) < duration {
                break;
            }
        }
        qps += 1;
        break;
    }
    stream.shutdown().await?;
    tx.send(qps).unwrap();
    Ok(())
}

impl ConcurrencyTest {
    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(egui::Checkbox::new(&mut self.use_raw, "use raw"));
            ui.add(egui::Slider::new(&mut self.port, 0..=65535).text("port"));
            if self.rx.is_none() {
                ui.add(egui::Slider::new(&mut self.concurrent_quantity, 1..=100000).text("concurrent quantity"));
            }

            ui.horizontal(|ui| {
                ui.label("hots:");
                ui.text_edit_singleline(&mut self.host);

                if self.rx.is_none() && ui.button("test").clicked() {
                    match format!("{}:{}", self.host, self.port).parse::<SocketAddr>() {
                        Ok(addr) => {
                            let (tx, rx) = channel();
                            self.rx = Some(rx);
                            self.results.clear();
                            let use_raw = self.use_raw;
                            for _ in 0..self.concurrent_quantity {
                                let tx_cloned = tx.clone();
                                tokio_runtime::instance().spawn(async move {
                                    if use_raw {
                                        let _ = do_test_raw(tx_cloned, addr).await;
                                    }
                                    else {
                                        do_test(tx_cloned, addr).await;
                                    }
                                });
                            }
                        }
                        Err(error) => {
                            error!("{}", error.to_string());
                        }
                    }
                }
            });
            ui.separator();

            match self.rx {
                Some(ref mut rx) => {
                    loop {
                        if let Ok(qps) = rx.try_recv() {
                            self.results.push(TestResult {
                                qps
                            });
                        }
                        else {
                            break;
                        }
                    }

                    if self.results.len() == self.concurrent_quantity as usize {
                        self.rx.take();
                    }
                }
                None => {
                    ui.label(format!("{}", self.results.iter().map(|x| x.qps).sum::<u32>()));
                }
            }


            // if ui.button("connect").clicked() {
            //     for peer in &self.peers {
            //         if let Ok(p) = peer.read() {
            //             if let Some(rpc) = &p.rpc {
            //                 if !rpc.is_connect() {
            //                     let peer_cloned = peer.clone();
            //                     tokio::spawn(async move {
            //                         peer_cloned.write().unwrap().rpc.unwrap().connect().await;
            //                     });
            //                 }
            //             }
            //         }
            //     }
            // }
        });
    }
}
