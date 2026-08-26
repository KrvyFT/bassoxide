//! 播放控制条（Phase 1 占位）。

use egui::Ui;
use crate::state::AppState;
use bassoxide_audio::PlaybackStatus;

/// 绘制播放控制条
pub fn transport_bar(ui: &mut Ui, state: &mut AppState) {
    let has_song = state.song.is_some();
    
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_enabled(false, egui::Button::new("|<"));
        
        let mut is_playing = false;
        if let Some(engine) = &state.audio_engine {
            is_playing = engine.status() == PlaybackStatus::Playing;
        }

        let play_text = if is_playing { "|| 暂停" } else { "▶ 播放" };
        if ui.add_enabled(has_song, egui::Button::new(play_text)).clicked() {
            if let Some(engine) = &state.audio_engine {
                if is_playing {
                    engine.pause();
                } else if let Some(song) = &state.song {
                    engine.play(song);
                }
            }
        }
        
        if ui.add_enabled(has_song, egui::Button::new("⏹ 停止")).clicked() {
            if let Some(engine) = &state.audio_engine {
                engine.stop();
            }
        }
        
        ui.add_enabled(false, egui::Button::new(">|"));
        ui.separator();
        ui.add_enabled(false, egui::Button::new("循环"));
    });
}
