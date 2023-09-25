use egui_tracing::EguiTracing;
use tracing::{span, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    let collector = egui_tracing::EguiTracing::default();
    tracing_subscriber::registry()
        .with(collector.layer())
        .init();

    let span = span!(Level::INFO, "my_span").entered();

    span.exit();

    let options = eframe::NativeOptions {
        resizable: true,
        initial_window_size: Some(egui::vec2(800.0, 500.0)),
        ..Default::default()
    };
    eframe::run_native(
        "tracing",
        options,
        Box::new(|_cc| Box::new(MyApp::new(collector))),
    )
    .unwrap();
}

pub struct MyApp {
    collector: EguiTracing,
}

impl MyApp {
    fn new(collector: EguiTracing) -> Self {
        Self { collector }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("window").constrain(true).show(ctx, |ui| {
            self.collector.ui(ui);
        });
    }
}
