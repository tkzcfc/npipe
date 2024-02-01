mod test_auto_register;
mod test_register;

use crate::apps::rpc_client::RpcClient;
use crate::tokio_runtime;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use log::error;
use np_proto::message_map::{serialize_to_json, MessageType};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

enum TestStatus {
    None,
    Requesting,
    Ok,
    Error,
}

type TestUnitMutexType = std::sync::Mutex<TestUnit>;

pub(super) struct TestUnit {
    name: String,
    status: TestStatus,
    logic: Box<dyn TestUnitLogic + Send>,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
    param_height: f32,
    response_height: f32,
}

impl TestUnit {
    fn new(name: &str, logic: Box<dyn TestUnitLogic + Send>) -> Arc<TestUnitMutexType> {
        Arc::new(std::sync::Mutex::new(Self {
            name: name.into(),
            status: TestStatus::None,
            logic,
            start_time: None,
            end_time: None,
            param_height: 20.0,
            response_height: 20.0,
        }))
    }
}

trait TestUnitLogic {
    fn render_parameter(&mut self, _ui: &mut Ui) {}
    fn render_response(&mut self, _ui: &mut Ui) {}
    fn call(&mut self, _rpc: &mut RpcClient, _unit_arc: Arc<TestUnitMutexType>) {
        todo!()
    }
    fn on_response(&mut self, _response: String) -> bool {
        true
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ProtoTest {
    #[serde(skip)]
    client: Arc<Mutex<RpcClient>>,

    #[serde(skip)]
    test_units: Vec<Arc<TestUnitMutexType>>,

    host: String,
    port: u16,
}

impl Default for ProtoTest {
    fn default() -> Self {
        let host = "127.0.0.1".into();
        let port = 8118;
        Self {
            client: Arc::new(Mutex::new(RpcClient::new(
                SocketAddr::from_str(format!("{}:{}", host, port).as_str())
                    .expect("invalid address"),
            ))),
            test_units: vec![
                TestUnit::new("register", Box::new(test_register::Test::default())),
                TestUnit::new(
                    "auto register",
                    Box::new(test_auto_register::Test::default()),
                ),
            ],
            host,
            port,
        }
    }
}

impl ProtoTest {
    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Ok(mut client) = self.client.try_lock() {
                client.update();

                ui.add(egui::Slider::new(&mut self.port, 0..=65535).text("port"));
                ui.label("hots:");
                ui.text_edit_singleline(&mut self.host);
                if client.is_connect() {
                    if ui.button("disconnect").clicked() {
                        client.disconnect();
                    }
                } else {
                    if ui.button("connect").clicked() {
                        let addr =
                            SocketAddr::from_str(format!("{}:{}", self.host, self.port).as_str())
                                .expect("invalid address");
                        client.reset_addr(addr);
                        let client_cloned = self.client.clone();
                        tokio_runtime::instance().spawn(async move {
                            if let Err(error) = client_cloned.lock().await.connect().await {
                                error!("connect failed: {}", error);
                            }
                        });
                    }

                    for unit in &self.test_units {
                        if let Ok(mut unit) = unit.try_lock() {
                            unit.status = TestStatus::None;
                        }
                    }
                }

                ui.separator();

                let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .min_scrolled_height(0.0);

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("name");
                        });
                        header.col(|ui| {
                            ui.strong("test");
                        });
                        header.col(|ui| {
                            ui.strong("status");
                        });
                        header.col(|ui| {
                            ui.strong("time");
                        });
                        header.col(|ui| {
                            ui.strong("parameter");
                        });
                        header.col(|ui| {
                            ui.strong("response");
                        });
                    })
                    .body(|mut body| {
                        for unit_arc in &self.test_units {
                            if let Ok(mut unit) = unit_arc.try_lock() {
                                let head_height = if unit.param_height > unit.response_height {
                                    unit.param_height
                                } else {
                                    unit.response_height
                                };

                                body.row(head_height, |mut row| {
                                    row.col(|ui| {
                                        ui.label(unit.name.as_str());
                                    });
                                    row.col(|ui| {
                                        if client.is_connect() && ui.button("do test").clicked() {
                                            match unit.status {
                                                TestStatus::None | TestStatus::Ok => {
                                                    unit.status = TestStatus::Requesting;
                                                    unit.start_time = Some(Instant::now());
                                                    unit.end_time = None;
                                                    unit.logic.call(&mut client, unit_arc.clone());
                                                }
                                                _ => {}
                                            }
                                        }
                                    });
                                    row.col(|ui| {
                                        match unit.status {
                                            TestStatus::None => ui.label("None"),
                                            TestStatus::Requesting => ui.label("Requesting"),
                                            TestStatus::Ok => ui.label("Ok"),
                                            TestStatus::Error => ui.label("Error"),
                                        };
                                    });
                                    row.col(|ui| {
                                        if unit.end_time.is_some() && unit.end_time.is_some() {
                                            ui.label(format!(
                                                "{}ms",
                                                unit.end_time
                                                    .unwrap()
                                                    .duration_since(unit.start_time.unwrap())
                                                    .as_millis()
                                            ));
                                        } else {
                                            ui.label("--");
                                        }
                                    });
                                    row.col(|ui| match unit.status {
                                        TestStatus::None | TestStatus::Ok => {
                                            let response = ui.vertical(|sub_ui| {
                                                unit.logic.render_parameter(sub_ui)
                                            });
                                            unit.param_height = response.response.rect.size().y;
                                        }
                                        _ => {}
                                    });
                                    row.col(|ui| {
                                        let response = ui
                                            .vertical(|sub_ui| unit.logic.render_response(sub_ui));
                                        unit.response_height = response.response.rect.size().y;
                                    });
                                });
                            }
                        }
                    });
            } else {
                ui.label("connecting");
            }
        });
    }
}

fn to_string(result: anyhow::Result<&MessageType>) -> String {
    match result {
        Ok(response) => serialize_to_json(response).unwrap_or_else(|err| err.to_string()),
        Err(err) => err.to_string(),
    }
}
