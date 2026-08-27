//! 播放控制条 — 控制外部音频轨（非谱面 MIDI）。

use egui::Ui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;
use bassoxide_audio::PlaybackStatus;

/// 绘制播放控制条
pub fn transport_bar(ui: &mut Ui, state: &mut AppState) {
    let has_audio = state.audio_track.is_some();
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let mut is_playing = false;
        if let Some(player) = &state.audio_player {
            is_playing = player.status() == PlaybackStatus::Playing;
        }

        let play_text = if is_playing { "|| 暂停" } else { "▶ 播放音频" };
        let play_btn = egui::Button::new(
            egui::RichText::new(play_text).color(palette.on_primary),
        )
        .fill(palette.primary);
        if ui
            .add_enabled(has_audio, play_btn)
            .on_hover_text("播放/暂停已加载的外部音频轨（空格）")
            .clicked()
        {
            if let Some(player) = &state.audio_player {
                player.toggle_play_pause();
            }
        }

        let stop_btn = egui::Button::new("⏹ 停止").fill(palette.secondary_container);
        if ui.add_enabled(has_audio, stop_btn).clicked() {
            if let Some(player) = &state.audio_player {
                player.stop();
            }
        }

        ui.separator();
        if has_audio {
            if let Some(player) = &state.audio_player {
                let t = player.score_position_secs();
                ui.label(
                    egui::RichText::new(format!("谱面时间 {t:.2}s"))
                        .size(12.0)
                        .color(palette.on_surface),
                );
            }
        } else {
            ui.label(
                egui::RichText::new("请先在底部添加音频轨")
                    .size(12.0)
                    .color(palette.on_surface_variant),
            );
        }
    });
}
