//! 播放控制条（Phase 1 占位）。

use egui::Ui;

/// 绘制播放控制条
pub fn transport_bar(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_enabled(false, egui::Button::new("|<"));
        ui.add_enabled(false, egui::Button::new("> 播放"));
        ui.add_enabled(false, egui::Button::new("[] 停止"));
        ui.add_enabled(false, egui::Button::new(">|"));
        ui.separator();
        ui.add_enabled(false, egui::Button::new("循环"));
    });
}
