use super::{to_string, TestStatus, TestUnitLogic, TestUnitMutexType};
use crate::apps::rpc_client::RpcClient;
use egui::Ui;
use np_proto::client_server;
use np_proto::message_map::MessageType;
use std::sync::Arc;
use std::time::Instant;

pub(super) struct Test {
    response: String,

    username: String,
    password: String,
}

impl Default for Test {
    fn default() -> Self {
        Self {
            response: "".into(),
            username: "abccccc".into(),
            password: "aaaaaaa".into(),
        }
    }
}

impl TestUnitLogic for Test {
    fn render_parameter(&mut self, ui: &mut Ui) {
        ui.label("username:");
        ui.text_edit_singleline(&mut self.username);

        ui.label("password:");
        ui.text_edit_singleline(&mut self.password);
    }

    fn render_response(&mut self, ui: &mut Ui) {
        ui.label(self.response.as_str());
    }

    fn call(&mut self, rpc: &mut RpcClient, unit_arc: Arc<TestUnitMutexType>) {
        let msg = MessageType::ClientServerLoginReq(client_server::LoginReq {
            username: self.username.clone(),
            password: self.password.clone(),
        });

        let request_data = to_string(Ok(&msg));

        rpc.send_request(msg, move |result: anyhow::Result<&MessageType>| {
            let mut unit = unit_arc.lock().unwrap();

            unit.end_time = Some(Instant::now());
            unit.status = if result.is_err() {
                TestStatus::Error
            } else {
                TestStatus::Ok
            };

            unit.logic.on_response(format!(
                "request:\n{}\nresponse:\n{}",
                request_data,
                to_string(result)
            ));
        });
    }

    fn on_response(&mut self, response: String) -> bool {
        self.response = response;
        true
    }
}
