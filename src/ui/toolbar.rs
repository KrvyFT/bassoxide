//! 工具栏。

use egui::Ui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 绘制工具栏
pub fn toolbar(ui: &mut Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // 歌曲信息
        let (title, artist, tempo) = match &state.song {
            Some(song) => (
                song.info.title.clone(),
                song.info.artist.clone(),
                song.tempo,
            ),
            None => return,
        };

        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(14.0)
                .color(palette.on_surface),
        );
        if !artist.is_empty() {
            ui.label(
                egui::RichText::new(format!("— {}", artist))
                    .size(12.0)
                    .color(palette.on_surface_variant),
            );
        }
        ui.separator();
        ui.label(
            egui::RichText::new(format!("♩={}", tempo))
                .color(palette.primary),
        );
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

            ui.label(
                egui::RichText::new("显示轨道:")
                    .color(palette.on_surface_variant),
            );
            if ui
                .add(egui::Button::new("◀").fill(palette.secondary_container))
                .on_hover_text("上一轨道")
                .clicked()
                && selected > 0
            {
                state.select_track(selected - 1);
            }
            ui.label(
                egui::RichText::new(format!("{}/{} {}", selected + 1, track_count, track_name))
                    .size(12.0)
                    .color(palette.on_surface),
            );
            if ui
                .add(egui::Button::new("▶").fill(palette.secondary_container))
                .on_hover_text("下一轨道")
                .clicked()
                && selected + 1 < track_count
            {
                state.select_track(selected + 1);
            }
        }
    });
}
