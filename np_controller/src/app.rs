
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
enum Anchor {
    ConcurrencyTest,
    Clock,
    ProtoTest,
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<Anchor> for egui::WidgetText {
    fn from(value: Anchor) -> Self {
        Self::RichText(egui::RichText::new(value.to_string()))
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::ConcurrencyTest
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
#[must_use]
enum Command {
    Nothing,
    ResetEverything,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ProtoTestApp {
    logic: crate::apps::proto_test::ProtoTest,
}

impl eframe::App for ProtoTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.logic.ui(ctx, frame);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct FractalClockApp {
    fractal_clock: crate::apps::fractal_clock::FractalClock,
}

impl eframe::App for FractalClockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| {
                self.fractal_clock.ui(ui, Some(seconds_since_midnight()));
            });
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ConcurrencyTestApp {
    logic: crate::apps::concurrency_test::ConcurrencyTest,
}

impl eframe::App for crate::app::ConcurrencyTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.logic.ui(ctx, frame);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct State {
    concurrency_test: ConcurrencyTestApp,
    clock: FractalClockApp,
    proto_test:ProtoTestApp,

    selected_anchor: Anchor,
    backend_panel: super::backend_panel::BackendPanel,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Application {
    state: State,
}

impl Application {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut slf = Self {
            state: State::default(),
        };

        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                slf.state = state;
            }
        }

        slf
    }
    fn apps_iter_mut(&mut self) -> impl Iterator<Item = (&str, Anchor, &mut dyn eframe::App)> {
        let vec = vec![
            (
                "✨ Concurrency Test",
                Anchor::ConcurrencyTest,
                &mut self.state.concurrency_test as &mut dyn eframe::App,
            ),
            (
                "🕑 Fractal Clock",
                Anchor::Clock,
                &mut self.state.clock as &mut dyn eframe::App,
            ),
            (
                "🕑 Proto Test",
                Anchor::ProtoTest,
                &mut self.state.proto_test as &mut dyn eframe::App,
            ),
        ];
        vec.into_iter()
    }

    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);

        ui.separator();

        ui.toggle_value(&mut self.state.backend_panel.open, "💻 Backend");

        ui.separator();

        let mut selected_anchor = self.state.selected_anchor;
        for (name, anchor, _app) in self.apps_iter_mut() {
            if ui
                .selectable_label(selected_anchor == anchor, name)
                .clicked()
            {
                selected_anchor = anchor;
                if frame.is_web() {
                    ui.ctx()
                        .open_url(egui::OpenUrl::same_tab(format!("#{anchor}")));
                }
            }
        }
        self.state.selected_anchor = selected_anchor;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::warn_if_debug_build(ui);
        });
    }

    fn backend_panel(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Command {
        // The backend-panel can be toggled on/off.
        // We show a little animation when the user switches it.
        let is_open =
            self.state.backend_panel.open || ctx.memory(|mem| mem.everything_is_visible());

        let mut cmd = Command::Nothing;

        egui::SidePanel::left("backend_panel")
            .resizable(false)
            .show_animated(ctx, is_open, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("💻 Backend");
                });

                ui.separator();
                self.backend_panel_contents(ui, frame, &mut cmd);
            });

        cmd
    }

    fn backend_panel_contents(
        &mut self,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
        cmd: &mut Command,
    ) {
        self.state.backend_panel.ui(ui, frame);

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button("Reset egui")
                .on_hover_text("Forget scroll, positions, sizes etc")
                .clicked()
            {
                ui.ctx().memory_mut(|mem| *mem = Default::default());
                ui.close_menu();
            }

            if ui.button("Reset everything").clicked() {
                *cmd = Command::ResetEverything;
                ui.close_menu();
            }
        });
    }

    fn show_selected_app(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let selected_anchor = self.state.selected_anchor;
        for (_name, anchor, app) in self.apps_iter_mut() {
            if anchor == selected_anchor || ctx.memory(|mem| mem.everything_is_visible()) {
                app.update(ctx, frame);
            }
        }
    }

    fn run_cmd(&mut self, ctx: &egui::Context, cmd: Command) {
        match cmd {
            Command::Nothing => {}
            Command::ResetEverything => {
                self.state = State::default();
                ctx.memory_mut(|mem| *mem = Default::default());
            }
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                self.bar_contents(ui, frame);
            });
        });

        self.state.backend_panel.update(ctx, frame);

        let cmd = self.backend_panel(ctx, frame);

        self.show_selected_app(ctx, frame);

        self.state.backend_panel.end_of_frame(ctx);

        self.run_cmd(ctx, cmd);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }
}

/// Time of day as seconds since midnight. Used for clock in demo app.
fn seconds_since_midnight() -> f64 {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
