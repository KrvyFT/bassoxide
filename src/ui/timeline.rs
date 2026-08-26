use eframe::egui;
use crate::state::AppState;

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("混音台与时间轴 (Mixer & Timeline)");
    ui.separator();
    
    // 我们必须检查 audio_engine 并在修改后通知它重新加载
    let mut needs_reload = false;
    
    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
        if let Some(song) = &mut state.song {
            for (i, track) in song.tracks.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Trk {}: {}", i + 1, track.name));
                    
                    // Solo Button
                    let solo_text = if track.is_solo { "S (On)" } else { "S" };
                    let mut solo_btn = egui::Button::new(solo_text);
                    if track.is_solo {
                        solo_btn = solo_btn.fill(egui::Color32::from_rgb(200, 150, 50));
                    }
                    if ui.add(solo_btn).clicked() {
                        track.is_solo = !track.is_solo;
                        needs_reload = true;
                    }
                    
                    // Mute Button
                    let mute_text = if track.is_muted { "M (On)" } else { "M" };
                    let mut mute_btn = egui::Button::new(mute_text);
                    if track.is_muted {
                        mute_btn = mute_btn.fill(egui::Color32::from_rgb(200, 50, 50));
                    }
                    if ui.add(mute_btn).clicked() {
                        track.is_muted = !track.is_muted;
                        needs_reload = true;
                    }
                    
                    // Volume Slider (placeholder)
                    let mut vol = 100;
                    ui.add(egui::Slider::new(&mut vol, 0..=127).text("Vol"));
                });
            }
        }
    });

    if needs_reload {
        if let Some(audio) = &state.audio_engine {
            if let Some(song) = &state.song {
                audio.reload_song(song);
            }
        }
    }
}
