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
    // 直角整块列表，行与行密接，无卡片圆角
    egui::Frame::NONE
        .fill(palette.surface_container_high)
        .corner_radius(egui::CornerRadius::ZERO)
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_width(full_w);
            egui::ScrollArea::vertical()
                .max_height(ui.available_height().max(80.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(song) = &mut state.song {
                        for (i, track) in song.tracks.iter_mut().enumerate() {
                            let row_w = ui.available_width();
                            let row_h = 32.0;
                            let selected = i == current_selected;
                            let row_fill = if selected {
                                palette.primary_container
                            } else if i % 2 == 0 {
                                palette.surface_container
                            } else {
                                palette.surface_container_high
                            };

                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                egui::vec2(row_w, row_h),
                                egui::Sense::click(),
                            );
                            ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, row_fill);

                            // 底部分隔线（直角）
                            ui.painter().hline(
                                row_rect.x_range(),
                                row_rect.bottom() - 0.5,
                                egui::Stroke::new(1.0_f32, palette.outline_variant),
                            );

                            let label = format!("Trk {}: {}", i + 1, track.name);
                            let name = gm_name(track.midi_bank, track.midi_program);

                            // 名称区
                            let name_rect = egui::Rect::from_min_size(
                                row_rect.min + egui::vec2(8.0, 0.0),
                                egui::vec2(220.0, row_h),
                            );
                            ui.painter().text(
                                name_rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                label,
                                egui::FontId::proportional(13.0),
                                if selected {
                                    palette.on_primary_container
                                } else {
                                    palette.on_surface
                                },
                            );

                            // 乐器区（直角按钮）
                            let inst_x = name_rect.right() + 8.0;
                            let inst_w = (row_rect.right() - inst_x - 8.0).max(120.0);
                            let inst_rect = egui::Rect::from_min_size(
                                egui::pos2(inst_x, row_rect.top() + 4.0),
                                egui::vec2(inst_w, row_h - 8.0),
                            );
                            ui.painter().rect_filled(
                                inst_rect,
                                egui::CornerRadius::ZERO,
                                palette.primary_container,
                            );
                            ui.painter().text(
                                inst_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                name,
                                egui::FontId::proportional(12.0),
                                palette.on_primary_container,
                            );

                            let name_click = row_resp
                                .clone()
                                .with_new_rect(name_rect)
                                .on_hover_text("点击仅显示该轨道");
                            // 用交互：整行点击选轨；点乐器区打开弹窗
                            let pointer = row_resp.interact_pointer_pos();
                            if row_resp.clicked() {
                                if let Some(pos) = pointer {
                                    if inst_rect.contains(pos) {
                                        open_popup = Some(i);
                                    } else {
                                        select_change = Some(i);
                                    }
                                } else {
                                    select_change = Some(i);
                                }
                            }
                            let _ = name_click;
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
