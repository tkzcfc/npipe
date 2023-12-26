mod client;
mod app;

use std::env;
use crate::app::Application;

fn main() -> eframe::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "client",
        native_options,
        Box::new(|cc| Box::new(Application::new(cc))),
    )
}