//! 工具栏。

use egui::Ui;

use crate::state::AppState;

/// 绘制工具栏
pub fn toolbar(ui: &mut Ui, state: &AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // 歌曲信息
        if let Some(song) = &state.song {
            ui.label(
                egui::RichText::new(&song.info.title)
                    .strong()
                    .size(14.0),
            );
            if !song.info.artist.is_empty() {
                ui.label(
                    egui::RichText::new(format!("— {}", song.info.artist))
                        .size(12.0)
                        .color(egui::Color32::from_gray(160)),
                );
            }
            ui.separator();
            ui.label(format!("♩={}", song.tempo));
        }
    });
}
