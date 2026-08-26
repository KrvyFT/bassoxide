//! Bassoxide — Rust 实现的 Guitar Pro 乐谱编辑器。

mod app;
mod state;
mod ui;

fn main() -> eframe::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

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
        Box::new(|cc| Ok(Box::new(app::BassoxideApp::new(cc)))),
    )
}
