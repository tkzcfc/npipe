use crate::apps::rpc_client::RpcClient;
use crate::tokio_runtime;
use byteorder::BigEndian;
use bytes::BytesMut;
use egui_extras::{Column, TableBuilder};
use log::error;
use np_proto::generic;
use np_proto::message_map::{encode_raw_message, get_message_id, get_message_size, MessageType};
use std::cell::RefCell;
use std::fmt::Pointer;
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::select;
use tokio::time::{interval, sleep, Instant};

struct TestResult {
    qps: u32,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ConcurrencyTest {
    #[serde(skip)]
    results: Vec<TestResult>,
    #[serde(skip)]
    rx: Option<Receiver<u32>>,

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

    let qps = Arc::new(Mutex::new(0u32));
    let isback = Arc::new(RwLock::new(false));

    while Instant::now().duration_since(start) < Duration::from_secs(1) {
        *isback.write().unwrap() = false;

        let isback_cloned = isback.clone();
        let qps_cloned = qps.clone();

        let message = MessageType::GenericPing(generic::Ping { ticks: 0 });

        rpc.send_request(message, move |result: io::Result<&MessageType>| {
            *isback_cloned.write().unwrap() = true;
            match result {
                Err(error) => {
                    println!("{}", error.to_string());
                }
                _ => {
                    *qps_cloned.lock().unwrap() += 1;
                }
            }
        });

        while !*isback.read().unwrap() {
            rpc.update();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    rpc.disconnect();

    // println!("qps:{}", qps);
    tx.send(*qps.lock().unwrap()).unwrap();
}

async fn do_test_raw_impl(tx: &Sender<u32>, addr: SocketAddr) -> io::Result<()> {
    let message = MessageType::GenericPing(generic::Ping { ticks: 0 });
    let message_id = get_message_id(&message).unwrap();
    let message_size = get_message_size(&message);
    let mut buf = Vec::with_capacity(message_size + 12);

    byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
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
    let duration = Duration::from_secs(100);

    let mut result = Ok(());

    select! {
        _= async {
            loop {
                sleep(Duration::from_millis(100)).await;
                result = stream.write_all(&buf).await;
                if result.is_err() {
                    break;
                }
                result = stream.flush().await;
                if result.is_err() {
                    break;
                }

                buffer.clear();
                loop {
                    if let Err(err) = stream.read_buf(&mut buffer).await {
                        result = Err(err);
                        return;
                    }
                    if buffer.len() >= 11 || Instant::now().duration_since(start) < duration {
                        qps += 1;
                        break;
                    }
                }
            }
        } => {},
        _= async {
            let mut wait = interval(Duration::from_millis(1));
            while Instant::now().duration_since(start) < duration {
                wait.tick().await;
            }
        } => {},
    }

    if let Err(error) = result {
        error!("{}", error.to_string());
    }

    stream.shutdown().await?;
    tx.send(qps).unwrap();
    return Ok(());
}

async fn do_test_raw(tx: Sender<u32>, addr: SocketAddr) {
    match do_test_raw_impl(&tx, addr).await {
        Err(error) => {
            error!("{}", error.to_string());
            tx.send(0).unwrap();
        }
        _ => {}
    }
}

impl ConcurrencyTest {
    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(egui::Checkbox::new(&mut self.use_raw, "use raw"));
            ui.add(egui::Slider::new(&mut self.port, 0..=65535).text("port"));
            if self.rx.is_none() {
                ui.add(
                    egui::Slider::new(&mut self.concurrent_quantity, 1..=100000)
                        .text("concurrent quantity"),
                );
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
                                        do_test_raw(tx_cloned, addr).await
                                    } else {
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

            match self.rx {
                Some(ref mut rx) => {
                    loop {
                        if let Ok(qps) = rx.try_recv() {
                            self.results.push(TestResult { qps });
                        } else {
                            break;
                        }
                    }

                    if self.results.len() == self.concurrent_quantity as usize {
                        self.rx.take();
                    }
                }
                None => {
                    ui.label(format!(
                        "{}",
                        self.results.iter().map(|x| x.qps).sum::<u32>()
                    ));
                }
            }

            ui.separator();

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::initial(60.0).range(30.0..=60.0))
                .column(Column::initial(100.0).range(60.0..=300.0))
                .column(Column::remainder())
                .min_scrolled_height(0.0);

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("id");
                    });
                    header.col(|ui| {
                        ui.strong("qps");
                    });
                    header.col(|ui| {
                        ui.strong("qps percent");
                    });
                    header.col(|ui| {
                        ui.strong("total percent");
                    });
                })
                .body(|mut body| {
                    let mut max_value = 0;
                    if let Some(max) = self.results.iter().map(|r| r.qps).max() {
                        max_value = max;
                    }
                    let total_value = self.results.iter().map(|r| r.qps).sum::<u32>();

                    let mut index = 0;
                    for result in &self.results {
                        index = index + 1;
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(index.to_string());
                            });
                            row.col(|ui| {
                                ui.label(result.qps.to_string());
                            });
                            row.col(|ui| {
                                let mut progress = 1.0;
                                if max_value > 0 {
                                    progress = result.qps as f32 / max_value as f32;
                                }
                                let progress_bar =
                                    egui::ProgressBar::new(progress).show_percentage();
                                ui.add(progress_bar);
                            });
                            row.col(|ui| {
                                let mut progress = 0.0;
                                if total_value > 0 {
                                    progress = result.qps as f32 / total_value as f32;
                                }
                                // ui.label(format!("{:.2}%", progress * 100.0));
                                let progress_bar =
                                    egui::ProgressBar::new(progress).show_percentage();
                                ui.add(progress_bar);
                            });
                        });
                    }
                });
        });
    }
}
