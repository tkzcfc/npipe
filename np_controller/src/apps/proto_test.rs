use log::error;
use crate::apps::rpc_client::RpcClient;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::tokio_runtime;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ProtoTest {
    #[serde(skip)]
    client: Arc<Mutex<RpcClient>>,

    host: String,
    port: u16,
}

impl Default for ProtoTest {
    fn default() -> Self {
        let host = "127.0.0.1".into();
        let port = 8118;
        Self {
            client: Arc::new(Mutex::new(RpcClient::new(SocketAddr::from_str(format!("{}:{}", host, port).as_str()).expect("invalid address")))),
            host,
            port,
        }
    }
}

impl ProtoTest {
    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Ok(mut client) = self.client.try_lock() {
                ui.add(egui::Slider::new(&mut self.port, 0..=65535).text("port"));
                ui.label("hots:");
                ui.text_edit_singleline(&mut self.host);
                if client.is_connect() {
                    if ui.button("disconnect").clicked() {
                        client.disconnect();
                    }
                } else {
                    if ui.button("connect").clicked() {
                        let addr = SocketAddr::from_str(format!("{}:{}", self.host, self.port).as_str()).expect("invalid address");
                        client.reset_addr(addr);
                        // let client_cloned = self.client.clone();
                        // tokio_runtime::instance().spawn(async move {
                        //     if let Err(error) = client_cloned.lock().unwrap().connect().await {
                        //         error!("connect failed: {}", error);
                        //     }
                        // });
                    }
                }

                ui.separator();
            }
            else {
                ui.label("connecting");
            }
        });
    }
}
