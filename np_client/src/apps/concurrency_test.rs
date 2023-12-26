#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ConcurrencyTest {}

impl Default for ConcurrencyTest {
    fn default() -> Self {
        Self {}
    }
}

impl ConcurrencyTest {
    pub fn ui(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {}
}
