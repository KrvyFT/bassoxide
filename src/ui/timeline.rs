use eframe::egui;
use crate::state::AppState;

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("混音台与时间轴 (Mixer & Timeline)");
    ui.separator();
    
    // SF2 选择
    ui.horizontal(|ui| {
        ui.label("音源 (SoundFont): ");
        let sf2_options = ["Orchestra_HQ.sf2"]; // 为了简单目前只放一个，或者扫描目录
        
        let mut changed = false;
        egui::ComboBox::from_id_salt("sf2_combo")
            .selected_text(&state.current_sf2)
            .show_ui(ui, |ui| {
                for option in sf2_options {
                    if ui.selectable_value(&mut state.current_sf2, option.to_string(), option).changed() {
                        changed = true;
                    }
                }
            });
            
        if changed {
            if let Some(audio) = &state.audio_engine {
                let path = format!("assets/{}", state.current_sf2);
                if let Err(e) = audio.load_soundfont(&path) {
                    state.status_message = format!("加载音源失败: {e}");
                } else {
                    state.status_message = format!("成功加载音源: {}", state.current_sf2);
                }
            }
        }
    });
    ui.separator();
    
    // 我们必须检查 audio_engine 并在修改后通知它重新加载
    let mut needs_reload = false;
    // 待切换的显示轨道（避免与 song 的可变借用冲突，循环后再应用）
    let mut select_change: Option<usize> = None;
    let current_selected = state.selected_track;

    let presets = state
        .audio_engine
        .as_ref()
        .map(|a| a.get_presets())
        .unwrap_or_default();

    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
        if let Some(song) = &mut state.song {
            for (i, track) in song.tracks.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // 点击轨道名可切换为“单轨道显示”的当前轨道
                    let label = format!("Trk {}: {}", i + 1, track.name);
                    if ui
                        .selectable_label(i == current_selected, label)
                        .on_hover_text("点击仅显示该轨道")
                        .clicked()
                    {
                        select_change = Some(i);
                    }
                    
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
                    
                    // 音色选择
                    let current_bank = track.midi_bank as i32;
                    let current_program = track.midi_program as i32;
                    let selected_name = presets
                        .iter()
                        .find(|p| p.0 == current_bank && p.1 == current_program)
                        .map(|p| p.2.as_str())
                        .unwrap_or("Unknown");
                        
                    egui::ComboBox::from_id_salt(format!("prog_track_{}", i))
                        .selected_text(selected_name)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for (bank, patch, name) in &presets {
                                let is_selected = *bank == current_bank && *patch == current_program;
                                if ui.selectable_label(is_selected, name).clicked() {
                                    track.midi_bank = *bank as u8;
                                    track.midi_program = *patch as u8;
                                    needs_reload = true;
                                }
                            }
                        });
                        
                    // Volume Slider (placeholder)
                    let mut vol = 100;
                    ui.add(egui::Slider::new(&mut vol, 0..=127).text("Vol"));
                });
            }
        }
    });

    if let Some(idx) = select_change {
        state.select_track(idx);
    }

    if needs_reload {
        if let Some(audio) = &state.audio_engine {
            if let Some(song) = &state.song {
                audio.reload_song(song);
            }
        }
    }
}
