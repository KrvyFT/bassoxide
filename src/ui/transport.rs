//! 播放控制条。

use egui::Ui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;
use bassoxide_audio::PlaybackStatus;

/// 绘制播放控制条
pub fn transport_bar(ui: &mut Ui, state: &mut AppState) {
    let has_song = state.song.is_some();
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_enabled(false, egui::Button::new("|<"));

        let mut is_playing = false;
        if let Some(engine) = &state.audio_engine {
            is_playing = engine.status() == PlaybackStatus::Playing;
        }

        let play_text = if is_playing { "|| 暂停" } else { "▶ 播放" };
        let play_btn = egui::Button::new(
            egui::RichText::new(play_text).color(palette.on_primary),
        )
        .fill(palette.primary);
        if ui.add_enabled(has_song, play_btn).clicked() {
            if let Some(engine) = &state.audio_engine {
                if is_playing {
                    engine.pause();
                } else if let Some(song) = &state.song {
                    engine.play(song);
                }
            }
        }

        let stop_btn = egui::Button::new("⏹ 停止").fill(palette.secondary_container);
        if ui.add_enabled(has_song, stop_btn).clicked() {
            if let Some(engine) = &state.audio_engine {
                engine.stop();
            }
        }

        ui.add_enabled(false, egui::Button::new(">|"));
        ui.separator();
        ui.add_enabled(false, egui::Button::new("循环"));
    });
}
