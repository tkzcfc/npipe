mod client;
mod app;
mod backend_panel;
mod frame_history;
mod apps;

use std::env;
use crate::app::Application;

fn main() -> eframe::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 320.0])
            .with_min_inner_size([300.0, 220.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "client",
        native_options,
        Box::new(|cc| Box::new(Application::new(cc))),
    )
}