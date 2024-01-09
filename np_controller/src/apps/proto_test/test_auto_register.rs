use super::{TestStatus, TestUnitLogic, TestUnitMutexType, to_string};
use crate::apps::rpc_client::RpcClient;
use egui::Ui;
use np_proto::client_server;
use np_proto::message_map::MessageType;
use rand::{distributions::Alphanumeric, Rng};
use std::sync::Arc;
use std::time::Instant;

pub(super) struct Test {
    total_count: u32,
    cur_count: u32,
    err_count: u32,
}

impl Default for Test {
    fn default() -> Self {
        Self {
            total_count: 1,
            cur_count: 0,
            err_count: 0,
        }
    }
}

fn random_text(min: usize, max: usize) -> String {
    let mut rng = rand::thread_rng();
    let range: usize = rng.gen_range(min..max);

    let rand_string: String = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .filter(|c| c.is_ascii_alphabetic()) // 确保是字母
        .map(char::from)
        .take(range) // 选择range个字符
        .collect();
    rand_string
}

impl TestUnitLogic for Test {
    fn render_parameter(&mut self, ui: &mut Ui) {
        ui.add(egui::Slider::new(&mut self.total_count, 1..=10000).text("count"));
    }

    fn render_response(&mut self, ui: &mut Ui) {
        ui.label(format!(
            "percent:{}/{}\nerror count: {}",
            self.cur_count, self.total_count, self.err_count
        ));
    }

    fn call(&mut self, rpc: &mut RpcClient, unit_arc: Arc<TestUnitMutexType>) {
        self.err_count = 0;
        self.cur_count = 0;

        for _ in 0..self.total_count {
            let msg = MessageType::ClientServerRegisterReq(client_server::RegisterReq {
                username: random_text(8, 15),
                password: random_text(8, 15),
            });

            let mut show_response = false;
            if self.total_count == 1 {
                println!("request: {}", to_string(Ok(&msg)));
                show_response = true;
            }

            let unit_cloned = unit_arc.clone();
            rpc.send_request(msg, move |result: anyhow::Result<&MessageType>| {
                let response = if result.is_err() {
                    "err".into()
                } else {
                    "ok".into()
                };

                let mut unit = unit_cloned.lock().unwrap();
                if unit.logic.on_response(response) {
                    unit.status = TestStatus::Ok;
                    unit.end_time = Some(Instant::now());
                }

                if show_response {
                    println!("response: {}", to_string(result));
                }
            });
        }
    }

    fn on_response(&mut self, response: String) -> bool {
        if response == "err" {
            self.err_count += 1;
        }
        self.cur_count += 1;
        self.cur_count >= self.total_count
    }
}
