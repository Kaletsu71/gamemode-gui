mod app;
mod backend;
mod config;
mod heroic;
mod steam;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GameMode Manager")
            .with_min_inner_size([500.0, 620.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "GameMode Manager",
        options,
        Box::new(|cc| Ok(Box::new(app::GameModeApp::new(cc)))),
    )
}
