mod api;
mod app;
mod gateway;
mod models;
mod state;
mod ui;
mod voice;

use app::App;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Rustcord")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([820.0, 520.0])
            .with_decorations(false),
        ..Default::default()
    };
    eframe::run_native(
        "Rustcord",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
