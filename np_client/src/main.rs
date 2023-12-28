mod app;
mod apps;
mod backend_panel;
mod frame_history;
mod tokio_runtime;

use crate::app::Application;
use eframe::Theme;

fn main() -> eframe::Result<()> {
    env_logger::init();

    tokio_runtime::instance();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 320.0])
            .with_min_inner_size([300.0, 220.0])
            .with_drag_and_drop(true),
        default_theme: Theme::Dark,
        ..Default::default()
    };
    let result = eframe::run_native(
        "client",
        native_options,
        Box::new(|cc| Box::new(Application::new(cc))),
    );

    tokio_runtime::destroy();
    result
}
