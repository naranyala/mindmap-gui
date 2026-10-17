mod app;
mod canvas;
mod model;

use app::MindMapApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([720.0, 520.0])
            .with_title("Mindmap"),
        ..Default::default()
    };

    let initial_path = std::env::args().nth(1);

    eframe::run_native(
        "mindmap-gui",
        options,
        Box::new(move |cc| Ok(Box::new(MindMapApp::new(cc, initial_path)))),
    )
}
