//! Bassoxide — Rust 实现的 Guitar Pro 乐谱编辑器。

mod app;
mod demo;
mod state;
mod ui;

fn main() -> eframe::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let load_demo = args.iter().any(|a| a == "--demo");
    let startup_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Bassoxide — Guitar Tablature Editor")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Bassoxide",
        options,
        Box::new(move |cc| {
            let mut app = app::BassoxideApp::new(cc);
            if load_demo {
                app.state
                    .load_song(demo::build_demo_song(), Some("demo://material-you".into()));
            } else if let Some(path) = startup_path {
                app.open_path(std::path::Path::new(&path));
            }
            Ok(Box::new(app))
        }),
    )
}
