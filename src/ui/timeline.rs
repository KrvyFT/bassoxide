//! 混音台面板 + 轨道配置弹窗。

use eframe::egui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// GM 乐器分组，便于选择电吉他/贝斯/键盘/鼓
fn preset_group(bank: i32, program: i32) -> &'static str {
    if bank == 128 || bank == 120 {
        return "鼓组";
    }
    match program {
        0..=7 => "钢琴/键盘",
        8..=15 => "色彩打击",
        16..=23 => "风琴",
        24..=31 => "吉他",
        32..=39 => "贝斯",
        40..=55 => "弦乐/合奏",
        56..=63 => "铜管",
        64..=71 => "簧管",
        72..=79 => "笛类",
        80..=95 => "合成主音/铺底",
        96..=103 => "特效",
        104..=111 => "民族",
        112..=119 => "打击乐",
        120..=127 => "音效",
        _ => "其他",
    }
}

const GROUP_ORDER: &[&str] = &[
    "吉他",
    "贝斯",
    "钢琴/键盘",
    "鼓组",
    "风琴",
    "弦乐/合奏",
    "铜管",
    "簧管",
    "笛类",
    "合成主音/铺底",
    "色彩打击",
    "特效",
    "民族",
    "打击乐",
    "音效",
    "其他",
];

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("混音台")
                .color(palette.on_surface)
                .size(16.0),
        );
        ui.label(
            egui::RichText::new("点击乐器按钮配置谱面与音色")
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
    ui.add_space(4.0);

    let mut needs_reload = false;
    let mut needs_relayout = false;
    let mut select_change: Option<usize> = None;
    let mut open_popup: Option<usize> = None;
    let current_selected = state.selected_track;

    let presets = state
        .audio_engine
        .as_ref()
        .map(|a| a.get_presets())
        .unwrap_or_default();

    // 混音台轨道区：占满底部可用宽度，作为整体表面
    let full_w = ui.available_width();
    egui::Frame::new()
        .fill(palette.surface_container_high)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(full_w - 4.0);
            egui::ScrollArea::vertical()
                .max_height(168.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if let Some(song) = &mut state.song {
                        for (i, track) in song.tracks.iter_mut().enumerate() {
                            let row_w = ui.available_width();
                            let row_h = 34.0;
                            let selected = i == current_selected;

                            egui::Frame::new()
                                .fill(if selected {
                                    palette.primary_container
                                } else {
                                    palette.surface_container
                                })
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::symmetric(8, 4))
                                .show(ui, |ui| {
                                    ui.set_min_width(row_w - 4.0);
                                    ui.horizontal(|ui| {
                                        ui.set_height(row_h);

                                        // 轨道名：固定宽度
                                        let label = format!("Trk {}: {}", i + 1, track.name);
                                        let name_resp = ui.add_sized(
                                            [200.0, row_h],
                                            egui::SelectableLabel::new(selected, label),
                                        );
                                        if name_resp
                                            .on_hover_text("点击仅显示该轨道")
                                            .clicked()
                                        {
                                            select_change = Some(i);
                                        }

                                        // Solo
                                        let solo_btn = if track.is_solo {
                                            egui::Button::new(
                                                egui::RichText::new("S")
                                                    .color(palette.on_primary),
                                            )
                                            .fill(palette.primary)
                                        } else {
                                            egui::Button::new("S")
                                        };
                                        if ui
                                            .add_sized([32.0, row_h], solo_btn)
                                            .on_hover_text("Solo")
                                            .clicked()
                                        {
                                            track.is_solo = !track.is_solo;
                                            needs_reload = true;
                                        }

                                        // Mute
                                        let mute_btn = if track.is_muted {
                                            egui::Button::new(
                                                egui::RichText::new("M").color(palette.on_error),
                                            )
                                            .fill(palette.error)
                                        } else {
                                            egui::Button::new("M")
                                        };
                                        if ui
                                            .add_sized([32.0, row_h], mute_btn)
                                            .on_hover_text("静音")
                                            .clicked()
                                        {
                                            track.is_muted = !track.is_muted;
                                            needs_reload = true;
                                        }

                                        // 乐器按钮：吃掉中间剩余宽度
                                        let current_bank = track.midi_bank as i32;
                                        let current_program = track.midi_program as i32;
                                        let selected_name = presets
                                            .iter()
                                            .find(|p| {
                                                p.0 == current_bank && p.1 == current_program
                                            })
                                            .map(|p| p.2.as_str())
                                            .unwrap_or("选择乐器…");

                                        let vol_w = 220.0_f32;
                                        let instrument_w =
                                            (ui.available_width() - vol_w - 8.0).max(140.0);
                                        let instrument_btn = egui::Button::new(
                                            egui::RichText::new(selected_name)
                                                .color(palette.on_primary_container),
                                        )
                                        .fill(palette.primary_container);
                                        if ui
                                            .add_sized([instrument_w, row_h], instrument_btn)
                                            .on_hover_text("配置谱面类型与乐器")
                                            .clicked()
                                        {
                                            open_popup = Some(i);
                                        }

                                        // 音量：行尾固定宽度
                                        let mut vol = track.volume as i32;
                                        if ui
                                            .add_sized(
                                                [vol_w, row_h],
                                                egui::Slider::new(&mut vol, 0..=127)
                                                    .text("Vol")
                                                    .trailing_fill(true),
                                            )
                                            .changed()
                                        {
                                            track.volume = vol as u8;
                                            needs_reload = true;
                                        }
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("打开乐谱后可在此调整各轨音色与谱面")
                                .color(palette.on_surface_variant),
                        );
                    }
                });
        });

    if let Some(idx) = select_change {
        state.select_track(idx);
    }
    if let Some(idx) = open_popup {
        state.track_config_popup = Some(idx);
    }

    // 轨道配置弹窗
    if let Some(track_idx) = state.track_config_popup {
        let mut close = false;
        let mut apply_reload = false;
        let mut apply_relayout = false;

        let track_name = state
            .song
            .as_ref()
            .and_then(|s| s.tracks.get(track_idx))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| format!("轨道 {}", track_idx + 1));

        egui::Window::new(format!("轨道配置 — {track_name}"))
            .id(egui::Id::new("track_config_popup"))
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                let Some(song) = state.song.as_mut() else {
                    close = true;
                    return;
                };
                let Some(track) = song.tracks.get_mut(track_idx) else {
                    close = true;
                    return;
                };

                ui.label(
                    egui::RichText::new("谱面类型")
                        .strong()
                        .color(palette.primary),
                );
                ui.label(
                    egui::RichText::new("可多选组合；四线谱与六线谱互斥")
                        .size(11.0)
                        .color(palette.on_surface_variant),
                );
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    let mut standard = track.staff_display.show_standard;
                    if ui
                        .checkbox(&mut standard, "五线谱")
                        .on_hover_text("标准五线记谱")
                        .changed()
                    {
                        track.staff_display.show_standard = standard;
                        apply_relayout = true;
                    }

                    let four_on =
                        track.staff_display.show_tab && track.staff_display.tab_strings == 4;
                    let mut four = four_on;
                    if ui
                        .checkbox(&mut four, "四线谱")
                        .on_hover_text("贝斯 Tab（4 弦）")
                        .changed()
                    {
                        if four {
                            track.apply_tab_string_count(4);
                        } else if track.staff_display.tab_strings == 4 {
                            track.staff_display.disable_tab();
                        }
                        apply_relayout = true;
                    }

                    let six_on =
                        track.staff_display.show_tab && track.staff_display.tab_strings == 6;
                    let mut six = six_on;
                    if ui
                        .checkbox(&mut six, "六线谱")
                        .on_hover_text("吉他 Tab（6 弦）")
                        .changed()
                    {
                        if six {
                            track.apply_tab_string_count(6);
                        } else if track.staff_display.tab_strings == 6 {
                            track.staff_display.disable_tab();
                        }
                        apply_relayout = true;
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    egui::RichText::new("乐器选择")
                        .strong()
                        .color(palette.primary),
                );
                ui.label(
                    egui::RichText::new("内置 GeneralUser GS 乐队音源")
                        .size(11.0)
                        .color(palette.on_surface_variant),
                );
                ui.add_space(4.0);

                let current_bank = track.midi_bank as i32;
                let current_program = track.midi_program as i32;

                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for group in GROUP_ORDER {
                            let group_presets: Vec<_> = presets
                                .iter()
                                .filter(|(b, p, _)| preset_group(*b, *p) == *group)
                                .collect();
                            if group_presets.is_empty() {
                                continue;
                            }
                            egui::CollapsingHeader::new(*group)
                                .default_open(
                                    *group == "吉他"
                                        || *group == "贝斯"
                                        || *group == "钢琴/键盘"
                                        || *group == "鼓组",
                                )
                                .show(ui, |ui| {
                                    for (bank, patch, name) in group_presets {
                                        let selected =
                                            *bank == current_bank && *patch == current_program;
                                        if ui.selectable_label(selected, name).clicked() {
                                            track.midi_bank = *bank as u8;
                                            track.midi_program = *patch as u8;
                                            if *bank == 128 {
                                                track.is_percussion = true;
                                            } else if track.is_percussion && *bank != 128 {
                                                track.is_percussion = false;
                                            }
                                            track.sync_instrument_type();
                                            apply_reload = true;
                                        }
                                    }
                                });
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("完成").color(palette.on_primary),
                            )
                            .fill(palette.primary),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
            });

        if close {
            state.track_config_popup = None;
        }
        if apply_reload {
            needs_reload = true;
        }
        if apply_relayout {
            needs_relayout = true;
        }
    }

    if needs_relayout {
        state.needs_relayout = true;
    }

    if needs_reload {
        if let Some(audio) = &state.audio_engine {
            if let Some(song) = &state.song {
                audio.reload_song(song);
            }
        }
    }

    // 状态栏作为底部整体一部分
    ui.add_space(6.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&state.status_message)
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
}
