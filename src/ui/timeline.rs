//! 轨道面板 + 谱面配置弹窗（不再驱动 MIDI 发声）。

use eframe::egui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 静态 GM 乐器名（仅用于标注轨道数据，不播放）
fn gm_name(bank: u8, program: u8) -> &'static str {
    if bank >= 120 {
        return "Standard Kit";
    }
    match program {
        0 => "Acoustic Grand Piano",
        4 => "Electric Piano 1",
        25 => "Acoustic Guitar (steel)",
        27 => "Electric Guitar (clean)",
        28 => "Electric Guitar (muted)",
        29 => "Overdriven Guitar",
        30 => "Distortion Guitar",
        33 => "Electric Bass (finger)",
        34 => "Electric Bass (pick)",
        48 => "String Ensemble 1",
        80 => "Lead 1 (square)",
        81 => "Lead 2 (sawtooth)",
        _ => "GM Instrument",
    }
}

fn preset_choices() -> &'static [(u8, u8, &'static str)] {
    &[
        (0, 0, "Acoustic Grand Piano"),
        (0, 4, "Electric Piano 1"),
        (0, 25, "Acoustic Guitar (steel)"),
        (0, 27, "Electric Guitar (clean)"),
        (0, 29, "Overdriven Guitar"),
        (0, 30, "Distortion Guitar"),
        (0, 33, "Electric Bass (finger)"),
        (0, 34, "Electric Bass (pick)"),
        (0, 48, "String Ensemble 1"),
        (0, 80, "Lead 1 (square)"),
        (128, 0, "Standard Kit"),
    ]
}

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("轨道")
                .color(palette.on_surface)
                .size(16.0),
        );
        ui.label(
            egui::RichText::new("配置谱面显示（音符发声已移除，请用底部音频轨）")
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
    ui.add_space(4.0);

    let mut needs_relayout = false;
    let mut select_change: Option<usize> = None;
    let mut open_popup: Option<usize> = None;
    let current_selected = state.selected_track;

    let full_w = ui.available_width();
    egui::Frame::new()
        .fill(palette.surface_container_high)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(full_w - 4.0);
            egui::ScrollArea::vertical()
                .max_height(120.0)
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

                                        let label = format!("Trk {}: {}", i + 1, track.name);
                                        let name_resp = ui.add_sized(
                                            [220.0, row_h],
                                            egui::SelectableLabel::new(selected, label),
                                        );
                                        if name_resp
                                            .on_hover_text("点击仅显示该轨道")
                                            .clicked()
                                        {
                                            select_change = Some(i);
                                        }

                                        let name = gm_name(track.midi_bank, track.midi_program);
                                        let instrument_w = (ui.available_width() - 8.0).max(140.0);
                                        let instrument_btn = egui::Button::new(
                                            egui::RichText::new(name)
                                                .color(palette.on_primary_container),
                                        )
                                        .fill(palette.primary_container);
                                        if ui
                                            .add_sized([instrument_w, row_h], instrument_btn)
                                            .on_hover_text("配置谱面类型与轨道乐器标注")
                                            .clicked()
                                        {
                                            open_popup = Some(i);
                                        }
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("打开乐谱后可在此调整谱面显示")
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

    if let Some(track_idx) = state.track_config_popup {
        let mut close = false;
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
                    egui::RichText::new("乐器标注（不发声）")
                        .strong()
                        .color(palette.primary),
                );
                ui.label(
                    egui::RichText::new("仅写入轨道 MIDI 音色元数据")
                        .size(11.0)
                        .color(palette.on_surface_variant),
                );
                ui.add_space(4.0);

                let current_bank = track.midi_bank;
                let current_program = track.midi_program;

                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (bank, program, name) in preset_choices() {
                        let selected = *bank == current_bank && *program == current_program;
                        if ui.selectable_label(selected, *name).clicked() {
                            track.midi_bank = *bank;
                            track.midi_program = *program;
                            track.is_percussion = *bank >= 120;
                            track.sync_instrument_type();
                        }
                    }
                });

                ui.add_space(8.0);
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

        if close {
            state.track_config_popup = None;
        }
        if apply_relayout {
            needs_relayout = true;
        }
    }

    if needs_relayout {
        state.needs_relayout = true;
    }

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
