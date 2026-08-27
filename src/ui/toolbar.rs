//! 工具栏。

use bassoxide_core::{midi_note_name, Track, Tuning};
use egui::Ui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 绘制工具栏
pub fn toolbar(ui: &mut Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);
    let mut needs_relayout = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        let (title, artist, tempo) = match &state.song {
            Some(song) => (
                song.info.title.clone(),
                song.info.artist.clone(),
                song.display_tempo(),
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
            egui::RichText::new(format!("♩={}", tempo)).color(palette.primary),
        );
        ui.separator();

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
                egui::RichText::new("显示轨道:").color(palette.on_surface_variant),
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

            ui.separator();

            ui.label(egui::RichText::new("纸张:").color(palette.on_surface_variant));
            egui::ComboBox::from_id_salt("toolbar_paper_size")
                .selected_text(state.score_prefs.paper_size.label())
                .width(72.0)
                .show_ui(ui, |ui| {
                    for size in bassoxide_layout::PaperSize::ALL {
                        let sel = state.score_prefs.paper_size == size;
                        if ui
                            .selectable_label(sel, format!("{} {}", size.label(), size.description()))
                            .clicked()
                            && !sel
                        {
                            state.score_prefs.paper_size = size;
                            state.apply_score_prefs();
                        }
                    }
                });

            ui.separator();

            ui.label(egui::RichText::new("乐谱种类:").color(palette.on_surface_variant));

            let track_idx = state.selected_track.min(track_count - 1);
            let mut set_standard: Option<bool> = None;
            let mut set_tab: Option<bool> = None;
            let mut set_string_count: Option<u8> = None;

            // 先读显示状态（不可变），再画控件，避免与 song 可变借用纠缠
            let (show_standard, show_tab, string_count) = state
                .song
                .as_ref()
                .and_then(|s| s.tracks.get(track_idx))
                .map(|t| {
                    (
                        t.staff_display.show_standard,
                        t.staff_display.show_tab,
                        t.string_count().clamp(1, 8) as u8,
                    )
                })
                .unwrap_or((true, false, 6));

            let mut standard = show_standard;
            if ui
                .checkbox(&mut standard, "五线谱")
                .on_hover_text("标准五线记谱")
                .changed()
            {
                set_standard = Some(standard);
            }

            let mut tab_on = show_tab;
            if ui
                .checkbox(&mut tab_on, "六线谱")
                .on_hover_text("Tab：可配置弦数与每弦音高")
                .changed()
            {
                set_tab = Some(tab_on);
            }

            if show_tab || set_tab == Some(true) {
                let mut n = string_count;
                ui.label(
                    egui::RichText::new("弦数")
                        .size(11.0)
                        .color(palette.on_surface_variant),
                );
                if ui
                    .add(egui::DragValue::new(&mut n).range(1..=8).speed(0.2))
                    .on_hover_text("六线谱线条数量（1–8）")
                    .changed()
                {
                    set_string_count = Some(n);
                }

                // 可切换按钮直接翻转开关，比一次性 Button 更稳
                let label = egui::RichText::new("调弦…").color(if state.tuning_editor_open {
                    palette.on_primary
                } else {
                    palette.on_surface
                });
                if ui
                    .add(
                        egui::Button::new(label)
                            .fill(if state.tuning_editor_open {
                                palette.primary
                            } else {
                                palette.secondary_container
                            })
                            .selected(state.tuning_editor_open),
                    )
                    .on_hover_text("配置每条线的空弦音高（Ctrl+T）")
                    .clicked()
                {
                    state.tuning_editor_open = !state.tuning_editor_open;
                    if state.tuning_editor_open {
                        state.status_message = "打开六线谱调弦".into();
                    }
                }
            }

            if let Some(song) = state.song.as_mut() {
                if let Some(track) = song.tracks.get_mut(track_idx) {
                    if let Some(v) = set_standard {
                        track.staff_display.show_standard = v;
                        needs_relayout = true;
                    }
                    if let Some(v) = set_tab {
                        if v {
                            track.enable_tab();
                        } else {
                            track.staff_display.disable_tab();
                        }
                        needs_relayout = true;
                    }
                    if let Some(n) = set_string_count {
                        track.set_string_count(n as usize);
                        needs_relayout = true;
                    }
                }
            }
        }
    });

    if needs_relayout {
        state.needs_relayout = true;
    }
}

/// 六线谱调弦配置窗口
pub fn tuning_editor_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.tuning_editor_open {
        return;
    }
    let palette = MaterialPalette::for_mode(state.is_light_theme);
    let track_idx = state.selected_track;
    let mut close = false;
    let mut changed = false;

    egui::Window::new("六线谱调弦")
        .id(egui::Id::new("tab_tuning_editor"))
        .collapsible(false)
        .resizable(true)
        .default_width(360.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("修改弦数或空弦音高后，谱面音符会按原音高自动换到新弦位。")
                    .size(12.0)
                    .color(palette.on_surface_variant),
            );
            ui.add_space(6.0);

            let Some(song) = state.song.as_mut() else {
                ui.label("未加载乐谱");
                return;
            };
            let idx = track_idx.min(song.tracks.len().saturating_sub(1));
            let Some(track) = song.tracks.get_mut(idx) else {
                return;
            };

            ui.horizontal(|ui| {
                ui.label("弦数");
                let mut n = track.string_count().clamp(1, 8) as u8;
                if ui
                    .add(egui::DragValue::new(&mut n).range(1..=8))
                    .changed()
                {
                    track.set_string_count(n as usize);
                    changed = true;
                }
                if ui.button("标准吉他").clicked() {
                    apply_preset(track, Tuning::standard_guitar());
                    changed = true;
                }
                if ui.button("标准贝斯").clicked() {
                    apply_preset(track, Tuning::standard_bass());
                    changed = true;
                }
            });

            ui.separator();
            ui.label(egui::RichText::new("各弦空弦音高（MIDI）").strong());

            let string_count = track.string_count();
            for i in 0..string_count {
                let number = (i + 1) as u8;
                let mut midi = track
                    .tuning
                    .strings
                    .get(i)
                    .map(|s| s.tuning)
                    .unwrap_or(40);
                ui.horizontal(|ui| {
                    ui.label(format!("弦 {number}"));
                    if ui
                        .add(egui::DragValue::new(&mut midi).range(12..=96).speed(0.3))
                        .changed()
                    {
                        track.set_string_open_pitch(number, midi);
                        changed = true;
                    }
                    ui.label(
                        egui::RichText::new(midi_note_name(midi))
                            .color(palette.primary)
                            .strong(),
                    );
                });
            }

            ui.add_space(10.0);
            ui.separator();
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("完成").color(palette.on_primary))
                        .fill(palette.primary),
                )
                .clicked()
            {
                close = true;
            }
        });

    if close {
        state.tuning_editor_open = false;
    }
    if changed {
        state.needs_relayout = true;
        state.status_message = "已更新六线谱调弦并重映射音符".into();
    }
}

fn apply_preset(track: &mut Track, new_tuning: Tuning) {
    let old = track.tuning.clone();
    track.tuning = new_tuning;
    track.remap_notes_preserving_pitch(&old);
    track.sync_tab_string_count();
    track.staff_display.show_tab = true;
}

fn direction_label(d: bassoxide_core::Direction) -> &'static str {
    use bassoxide_core::Direction::*;
    match d {
        Coda => "Coda",
        DoubleCoda => "Double Coda",
        Segno => "Segno",
        SegnoSegno => "SegnoSegno",
        Fine => "Fine",
        DaCapo => "D.C.",
        DaCapoAlCoda => "D.C. al Coda",
        DaCapoAlDoubleCoda => "D.C. al Double Coda",
        DaCapoAlFine => "D.C. al Fine",
        DalSegno => "D.S.",
        DalSegnoAlCoda => "D.S. al Coda",
        DalSegnoAlDoubleCoda => "D.S. al Double Coda",
        DalSegnoAlFine => "D.S. al Fine",
        DalSegnoSegno => "D.S.S.",
        DalSegnoSegnoAlCoda => "D.S.S. al Coda",
        DalSegnoSegnoAlDoubleCoda => "D.S.S. al Double Coda",
        DalSegnoSegnoAlFine => "D.S.S. al Fine",
    }
}

const COMMON_DIRECTIONS: &[bassoxide_core::Direction] = &[
    bassoxide_core::Direction::Segno,
    bassoxide_core::Direction::Coda,
    bassoxide_core::Direction::Fine,
    bassoxide_core::Direction::DaCapo,
    bassoxide_core::Direction::DalSegno,
    bassoxide_core::Direction::DaCapoAlCoda,
    bassoxide_core::Direction::DalSegnoAlCoda,
    bassoxide_core::Direction::DaCapoAlFine,
    bassoxide_core::Direction::DalSegnoAlFine,
];

/// 小节排练标记 / 段落方向编辑
pub fn marker_editor_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.marker_editor_open {
        return;
    }
    let palette = MaterialPalette::for_mode(state.is_light_theme);
    let measure_idx = state.cursor.measure;
    let mut close = false;
    let mut jump_name: Option<String> = None;
    let mut jump_dir: Option<bassoxide_core::Direction> = None;

    egui::Window::new("小节标记")
        .id(egui::Id::new("marker_editor"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("小节 {}", measure_idx + 1))
                    .color(palette.on_surface_variant),
            );
            ui.horizontal(|ui| {
                ui.label("排练标记:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.marker_edit_name)
                        .desired_width(160.0)
                        .hint_text("如 A / Chorus"),
                );
            });

            ui.horizontal(|ui| {
                if ui.button("应用标记").clicked() {
                    if let Some(song) = state.song.as_mut() {
                        if let Some(mb) = song.master_bars.get_mut(measure_idx) {
                            let name = state.marker_edit_name.trim().to_string();
                            if name.is_empty() {
                                mb.marker = None;
                            } else {
                                mb.marker = Some(bassoxide_core::Marker {
                                    name,
                                    color: bassoxide_core::types::Color::rgb(200, 120, 0),
                                });
                            }
                            state.needs_relayout = true;
                            state.status_message = "已更新小节标记".into();
                        }
                    }
                }
                if ui.button("清除标记").clicked() {
                    state.marker_edit_name.clear();
                    if let Some(song) = state.song.as_mut() {
                        if let Some(mb) = song.master_bars.get_mut(measure_idx) {
                            mb.marker = None;
                            state.needs_relayout = true;
                        }
                    }
                }
                if ui.button("跳转到此标记").clicked() {
                    let name = state.marker_edit_name.trim().to_string();
                    if !name.is_empty() {
                        jump_name = Some(name);
                    }
                }
            });

            ui.separator();
            ui.label(
                egui::RichText::new("段落方向")
                    .color(palette.on_surface_variant),
            );

            let current_dirs: Vec<bassoxide_core::Direction> = state
                .song
                .as_ref()
                .and_then(|s| s.master_bars.get(measure_idx))
                .map(|mb| mb.directions.clone())
                .unwrap_or_default();

            ui.horizontal_wrapped(|ui| {
                for &d in COMMON_DIRECTIONS {
                    let on = current_dirs.contains(&d);
                    if ui.selectable_label(on, direction_label(d)).clicked() {
                        if let Some(song) = state.song.as_mut() {
                            if let Some(mb) = song.master_bars.get_mut(measure_idx) {
                                if let Some(pos) = mb.directions.iter().position(|x| *x == d) {
                                    mb.directions.remove(pos);
                                } else {
                                    mb.directions.push(d);
                                }
                                state.needs_relayout = true;
                            }
                        }
                    }
                }
            });

            ui.horizontal(|ui| {
                for &d in &[
                    bassoxide_core::Direction::Segno,
                    bassoxide_core::Direction::Coda,
                    bassoxide_core::Direction::Fine,
                ] {
                    if ui
                        .button(format!("跳转 {}", direction_label(d)))
                        .clicked()
                    {
                        jump_dir = Some(d);
                    }
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    close = true;
                }
            });
        });

    if let Some(name) = jump_name {
        state.jump_to_marker_name(&name);
    }
    if let Some(d) = jump_dir {
        state.jump_to_direction(d);
    }
    if close {
        state.marker_editor_open = false;
    }
}

