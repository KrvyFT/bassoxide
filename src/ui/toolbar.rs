//! 工具栏。

use egui::Ui;

use crate::state::AppState;

/// 绘制工具栏
pub fn toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // 歌曲信息
        let (title, artist, tempo) = match &state.song {
            Some(song) => (
                song.info.title.clone(),
                song.info.artist.clone(),
                song.tempo,
            ),
            None => return,
        };

        ui.label(egui::RichText::new(title).strong().size(14.0));
        if !artist.is_empty() {
            ui.label(
                egui::RichText::new(format!("— {}", artist))
                    .size(12.0)
                    .color(egui::Color32::from_gray(160)),
            );
        }
        ui.separator();
        ui.label(format!("♩={}", tempo));
        ui.separator();

        // 单轨道显示：轨道切换
        let track_count = state.song.as_ref().map(|s| s.track_count()).unwrap_or(0);
        if track_count > 0 {
            let selected = state.selected_track.min(track_count - 1);
            let track_name = state
                .song
                .as_ref()
                .and_then(|s| s.tracks.get(selected))
                .map(|t| t.name.clone())
                .unwrap_or_default();

            ui.label("显示轨道:");
            if ui.button("◀").on_hover_text("上一轨道").clicked() && selected > 0 {
                state.select_track(selected - 1);
            }
            ui.label(
                egui::RichText::new(format!("{}/{} {}", selected + 1, track_count, track_name))
                    .size(12.0),
            );
            if ui.button("▶").on_hover_text("下一轨道").clicked() && selected + 1 < track_count {
                state.select_track(selected + 1);
            }
        }
    });
}
