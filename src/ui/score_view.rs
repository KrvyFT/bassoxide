//! 乐谱主视图 — 轨道显示器：滚动、缩放、点选/拖选、小节选中、播放头。

use std::collections::HashSet;

use egui::{Pos2, Rect, ScrollArea, Sense, Stroke, Ui};
use bassoxide_audio::{score_secs_in_measure, score_timeline, snap_to_nearest_beat};
use bassoxide_layout::engine::LayoutResult;
use bassoxide_render::{EditCursor, ScorePainter};

use crate::state::{AppState, CursorPosition, NoteRef};
use crate::ui::material::MaterialPalette;

/// 绘制乐谱主视图（轨道显示器）
pub fn score_view(ui: &mut Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    let mut current_zoom = state.zoom_factor;
    if ui.rect_contains_pointer(ui.max_rect()) {
        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        if zoom_delta != 1.0 {
            current_zoom *= zoom_delta;
            current_zoom = current_zoom.clamp(0.3, 5.0);
        }
    }
    if (current_zoom - state.zoom_factor).abs() > 0.001 {
        state.zoom_factor = current_zoom;
        state.update_zoom();
        state.needs_relayout = true;
    }

    if state.song.is_none() || state.layout.is_none() {
        ui.painter().rect_filled(ui.max_rect(), 0.0, palette.surface);
        ui.centered_and_justified(|ui| {
            ui.heading(egui::RichText::new("Bassoxide").color(palette.on_surface));
            ui.label(
                egui::RichText::new("按 Ctrl+O 打开乐谱 / .bso 工程")
                    .color(palette.on_surface_variant),
            );
        });
        return;
    }

    let playhead = state
        .audio_player
        .as_ref()
        .map(|p| p.score_position_secs())
        .unwrap_or(0.0);
    let selected = state.selected_track;
    let edit_cursor = EditCursor {
        track: state.cursor.track,
        measure: state.cursor.measure,
        beat: state.cursor.beat,
        string: state.cursor.string,
    };
    let layout_settings = state.layout_settings.clone();
    let selection_notes: HashSet<(usize, usize, u8)> = state
        .selection
        .notes
        .iter()
        .map(|n| (n.measure, n.beat, n.string))
        .collect();
    let selection_measure = state.selection.measure;
    let mut drag_origin = state.drag_select_origin;
    let mut drag_anchor = state.drag_select_anchor;

    let viewport = ui.available_size();
    let mut seek_request: Option<f64> = None;
    let mut cursor_hit: Option<CursorPosition> = None;
    let mut select_notes: Option<HashSet<NoteRef>> = None;
    let mut select_measure: Option<usize> = None;
    let mut clear_selection = false;
    let mut new_drag_origin = drag_origin;
    let mut new_drag_anchor = drag_anchor;
    let mut clear_drag = false;

    {
        let song = state.song.as_ref().unwrap();
        let layout = state.layout.as_ref().unwrap();
        let theme = &state.theme;

        let page_w = layout.total_width;
        let page_h = layout.total_height;
        let center_pad_x = ((viewport.x - page_w).max(0.0) * 0.5).floor();
        let content_width = (page_w + center_pad_x * 2.0 + 48.0).max(viewport.x);
        let content_height = (page_h + 64.0).max(viewport.y);

        // 拖选不抢滚动：滚轮仍可用
        ScrollArea::both()
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::Vec2::new(content_width, content_height),
                    Sense::click_and_drag(),
                );

                painter.rect_filled(response.rect, 0.0, palette.surface);

                let offset = egui::vec2(
                    response.rect.min.x + center_pad_x + 24.0,
                    response.rect.min.y + 24.0,
                );

                let score_painter = ScorePainter::new(&layout_settings, theme)
                    .with_edit_cursor(edit_cursor)
                    .with_selection(selection_notes, selection_measure);
                score_painter.paint(&painter, song, layout, offset);

                if let Some((x, y, h)) = playhead_geometry(layout, song, playhead, selected) {
                    let px = x + offset.x;
                    let top = y + offset.y;
                    painter.line_segment(
                        [Pos2::new(px, top), Pos2::new(px, top + h)],
                        Stroke::new(2.0_f32, palette.error),
                    );
                }

                // 原始指针状态机做拖选（ScrollArea 下 response.drag_* 不可靠）；
                // 单击/小节仍用 response.clicked()，同帧 press+release 不进入拖选。
                let clicked = response.clicked();
                let (raw_pressed, raw_down, raw_released, pointer_pos, shift) =
                    ui.ctx().input(|i| {
                        (
                            i.pointer.primary_pressed(),
                            i.pointer.button_down(egui::PointerButton::Primary),
                            i.pointer.primary_released(),
                            i.pointer
                                .interact_pos()
                                .or(i.pointer.hover_pos())
                                .or(i.pointer.latest_pos()),
                            i.modifiers.shift,
                        )
                    });
                let over_score = pointer_pos.is_some_and(|p| response.rect.contains(p));
                // 同帧 press+release：当作单击候选，不启动跨帧拖选
                let same_frame_click = raw_pressed && raw_released;

                // press 跨帧才记 origin；同帧单击留给 response.clicked()
                if raw_pressed && over_score && !same_frame_click {
                    if let Some(pos) = pointer_pos {
                        new_drag_origin = Some(pos);
                        let local = Pos2::new(pos.x - offset.x, pos.y - offset.y);
                        new_drag_anchor =
                            hit_test_cursor(layout, song, selected, local, &layout_settings);
                    }
                }

                let origin = new_drag_origin.or(drag_origin);
                let anchor = new_drag_anchor.or(drag_anchor);
                let mut drag_distance = 0.0_f32;

                if let (Some(o), Some(p)) = (origin, pointer_pos) {
                    drag_distance = o.distance(p);
                }

                // 按下移动中或松开时：位移超阈值则画橡皮筋并实时写入选区
                let dragging =
                    origin.is_some() && !same_frame_click && (raw_down || raw_released);
                if dragging && drag_distance > 4.0 {
                    if let (Some(o), Some(p)) = (origin, pointer_pos) {
                        new_drag_origin = Some(o);
                        let local = Pos2::new(p.x - offset.x, p.y - offset.y);
                        // 起点若在谱号区未命中，拖入谱面后补建 anchor
                        let mut effective_anchor = anchor;
                        if effective_anchor.is_none() {
                            if let Some(a) = hit_test_cursor(
                                layout,
                                song,
                                selected,
                                local,
                                &layout_settings,
                            ) {
                                effective_anchor = Some(a);
                                new_drag_anchor = Some(a);
                            }
                        } else {
                            new_drag_anchor = effective_anchor;
                        }
                        let rect = Rect::from_two_pos(o, p);
                        painter.rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(60, 120, 200, 36),
                        );
                        painter.rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(1.0, palette.primary),
                            egui::StrokeKind::Outside,
                        );
                        if let Some(a) = effective_anchor {
                            if let Some(b) = hit_test_cursor(
                                layout,
                                song,
                                selected,
                                local,
                                &layout_settings,
                            ) {
                                let notes =
                                    collect_notes_in_cell_range(song, selected, a, b);
                                if !notes.is_empty() {
                                    select_notes = Some(notes);
                                }
                            }
                        }
                    }
                }

                if raw_released && origin.is_some() && drag_distance > 4.0 {
                    clear_drag = true;
                    new_drag_origin = None;
                    new_drag_anchor = None;
                } else if clicked {
                    // 单击 / Shift+加选 / 小节：不依赖 drag_*，也不被拖选吞掉
                    clear_drag = true;
                    new_drag_origin = None;
                    new_drag_anchor = None;
                    let click_pos = response
                        .interact_pointer_pos()
                        .or(pointer_pos);
                    if let Some(pos) = click_pos {
                        let local = Pos2::new(pos.x - offset.x, pos.y - offset.y);
                        let header_hit = hit_test_measure_header(
                            layout,
                            song,
                            selected,
                            local,
                            &layout_settings,
                        );
                        let cursor_ht = hit_test_cursor(
                            layout,
                            song,
                            selected,
                            local,
                            &layout_settings,
                        );
                        if let Some(m) = header_hit {
                            select_measure = Some(m);
                            if let Some(hit) = cursor_ht {
                                cursor_hit = Some(CursorPosition {
                                    track: selected,
                                    measure: m,
                                    beat: hit.beat,
                                    string: hit.string,
                                });
                            } else {
                                cursor_hit = Some(CursorPosition {
                                    track: selected,
                                    measure: m,
                                    beat: 0,
                                    string: 1,
                                });
                            }
                        } else if let Some(hit) = cursor_ht {
                            cursor_hit = Some(hit);
                            if !shift {
                                clear_selection = true;
                            }
                            if shift {
                                if let Some(secs) = score_secs_at_layout_pos(
                                    layout,
                                    song,
                                    selected,
                                    local,
                                ) {
                                    seek_request = Some(secs);
                                }
                            }
                        } else if let Some(secs) =
                            score_secs_at_layout_pos(layout, song, selected, local)
                        {
                            seek_request = Some(secs);
                        }
                    }
                } else if raw_down && origin.is_some() {
                    // 跨帧按住：保留 origin/anchor
                    new_drag_origin = origin;
                    new_drag_anchor = anchor;
                } else if raw_released && origin.is_some() {
                    clear_drag = true;
                    new_drag_origin = None;
                    new_drag_anchor = None;
                }
            });
    }

    if clear_drag {
        state.drag_select_origin = None;
        state.drag_select_anchor = None;
    } else {
        if new_drag_origin.is_some() {
            state.drag_select_origin = new_drag_origin;
        }
        if new_drag_anchor.is_some() {
            state.drag_select_anchor = new_drag_anchor;
        }
    }

    if let Some(notes) = select_notes {
        let n = notes.len();
        state.selection.measure = None;
        state.selection.notes = notes;
        if let Some(first) = state.selection.notes.iter().next().copied() {
            state.cursor.measure = first.measure;
            state.cursor.beat = first.beat;
            state.cursor.string = first.string;
            state.cursor.track = selected;
        }
        state.fret_input.clear();
        state.status_message = format!("已选中 {} 个音符", n);
    } else if let Some(m) = select_measure {
        state.selection.clear();
        state.selection.measure = Some(m);
        // 填入该小节全部音符，便于批量改品格
        if let Some(song) = state.song.as_ref() {
            if let Some(track) = song.tracks.get(selected) {
                if let Some(measure) = track.measures.get(m) {
                    for (bi, beat) in measure.primary_voice().beats.iter().enumerate() {
                        for note in &beat.notes {
                            state.selection.notes.insert(NoteRef {
                                measure: m,
                                beat: bi,
                                string: note.string,
                            });
                        }
                    }
                }
            }
        }
        if let Some(c) = cursor_hit {
            state.cursor = c;
        } else {
            state.cursor.measure = m;
            state.cursor.track = selected;
        }
        state.fret_input.clear();
        state.status_message = format!("选中小节 {}", m + 1);
    } else if let Some(c) = cursor_hit {
        if clear_selection {
            state.selection.select_single(c);
        } else {
            // Shift+点击：加入多选
            state.selection.measure = None;
            state.selection.notes.insert(NoteRef::from(c));
        }
        state.status_message = format!(
            "选中 小节{} 拍{} 弦{}",
            c.measure + 1,
            c.beat + 1,
            c.string
        );
        state.cursor = c;
        state.fret_input.clear();
    }

    if let Some(secs) = seek_request {
        state.seek_score_secs(secs, true);
    }
}

fn collect_notes_in_cell_range(
    song: &bassoxide_core::song::Song,
    selected_track: usize,
    a: CursorPosition,
    b: CursorPosition,
) -> HashSet<NoteRef> {
    let mut out = HashSet::new();
    let Some(track) = song.tracks.get(selected_track) else {
        return out;
    };
    let m0 = a.measure.min(b.measure);
    let m1 = a.measure.max(b.measure);
    let s0 = a.string.min(b.string);
    let s1 = a.string.max(b.string);
    for m in m0..=m1 {
        let Some(measure) = track.measures.get(m) else {
            continue;
        };
        let beats = &measure.primary_voice().beats;
        if beats.is_empty() {
            continue;
        }
        let (b0, b1) = if m0 == m1 {
            (a.beat.min(b.beat), a.beat.max(b.beat))
        } else if m == m0 {
            let start = if a.measure == m0 { a.beat } else { b.beat };
            (start, beats.len().saturating_sub(1))
        } else if m == m1 {
            let end = if a.measure == m1 { a.beat } else { b.beat };
            (0, end.min(beats.len().saturating_sub(1)))
        } else {
            (0, beats.len().saturating_sub(1))
        };
        for bi in b0..=b1.min(beats.len().saturating_sub(1)) {
            for note in &beats[bi].notes {
                if note.string >= s0 && note.string <= s1 {
                    out.insert(NoteRef {
                        measure: m,
                        beat: bi,
                        string: note.string,
                    });
                }
            }
            // 无音符的格也纳入选区，便于后续批量插入
            for s in s0..=s1 {
                if beats[bi].note_on_string(s).is_none() {
                    out.insert(NoteRef {
                        measure: m,
                        beat: bi,
                        string: s,
                    });
                }
            }
        }
    }
    out
}

/// 点在谱表上方热区（含小节号与弦 1 上方衬垫）→ 整小节
fn hit_test_measure_header(
    layout: &LayoutResult,
    song: &bassoxide_core::song::Song,
    selected_track: usize,
    pos: Pos2,
    settings: &bassoxide_layout::spacing::LayoutSettings,
) -> Option<usize> {
    let _ = song;
    for system in &layout.systems {
        let Some(staff) = system.staves.iter().find(|s| {
            s.track_index == selected_track
                && s.staff_type == bassoxide_layout::staff::StaffType::Tablature
        }) else {
            continue;
        };
        let staff_y = system.y + staff.y;
        // 覆盖小节号（约 staff_y-18），下缘止于弦 1 之上，避免抢走音符点击
        let header_top = staff_y - 36.0;
        let header_bot = staff_y + (settings.note_pad() * 0.4).clamp(4.0, 12.0);
        if pos.y < header_top || pos.y > header_bot {
            continue;
        }
        for mp in &system.measure_positions {
            if pos.x >= mp.x && pos.x <= mp.x + mp.width {
                return Some(mp.measure_index);
            }
        }
    }
    None
}

/// 布局坐标 → 编辑光标（最近 beat + 最近弦）
fn hit_test_cursor(
    layout: &LayoutResult,
    song: &bassoxide_core::song::Song,
    selected_track: usize,
    pos: Pos2,
    settings: &bassoxide_layout::spacing::LayoutSettings,
) -> Option<CursorPosition> {
    let track = song.tracks.get(selected_track)?;
    let string_count = track
        .tuning
        .string_count()
        .max(track.staff_display.tab_strings as usize)
        .clamp(1, 8);

    for system in &layout.systems {
        if pos.y < system.y || pos.y > system.y + system.height {
            continue;
        }
        let staff = system
            .staves
            .iter()
            .find(|s| {
                s.track_index == selected_track
                    && s.staff_type == bassoxide_layout::staff::StaffType::Tablature
            })
            .or_else(|| {
                system
                    .staves
                    .iter()
                    .find(|s| s.track_index == selected_track)
            })?;
        let staff_y = system.y + staff.y;
        if pos.y < staff_y - 4.0 || pos.y > staff_y + staff.height + settings.rhythm_height {
            continue;
        }

        // 优先命中所在小节；若在谱号区等外侧，吸附到最近小节
        let mut best_mp_idx: Option<usize> = None;
        let mut best_mp_d = f32::MAX;
        for (i, mp) in system.measure_positions.iter().enumerate() {
            if pos.x >= mp.x && pos.x <= mp.x + mp.width {
                best_mp_idx = Some(i);
                best_mp_d = 0.0;
                break;
            }
            let d = if pos.x < mp.x {
                mp.x - pos.x
            } else {
                pos.x - (mp.x + mp.width)
            };
            if d < best_mp_d {
                best_mp_d = d;
                best_mp_idx = Some(i);
            }
        }
        let Some(mp) = best_mp_idx.map(|i| &system.measure_positions[i]) else {
            continue;
        };
        // 离小节过远（例如完全在别的系统）则跳过
        if best_mp_d > mp.width.max(40.0) {
            continue;
        }
        let rel = (pos.x - mp.x).clamp(0.0, mp.width);
        let Some(beats) = layout
            .beat_positions
            .get(mp.measure_index)
            .and_then(|tracks| tracks.get(selected_track))
        else {
            continue;
        };
        if beats.is_empty() {
            return Some(CursorPosition {
                track: selected_track,
                measure: mp.measure_index,
                beat: 0,
                string: string_from_y(pos.y - staff_y, string_count, settings),
            });
        }
        let mut best_i = 0usize;
        let mut best_d = f32::MAX;
        for (i, b) in beats.iter().enumerate() {
            let cx = b.x + b.width * 0.5;
            let d = (rel - cx).abs();
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        let beat_index = beats[best_i].beat_index;
        return Some(CursorPosition {
            track: selected_track,
            measure: mp.measure_index,
            beat: beat_index,
            string: string_from_y(pos.y - staff_y, string_count, settings),
        });
    }
    None
}

fn string_from_y(
    y_in_staff: f32,
    string_count: usize,
    settings: &bassoxide_layout::spacing::LayoutSettings,
) -> u8 {
    let mut best = 1u8;
    let mut best_d = f32::MAX;
    for s in 1..=string_count as u8 {
        let sy = bassoxide_layout::tablature::string_y_offset(s, string_count, settings);
        let d = (y_in_staff - sy).abs();
        if d < best_d {
            best_d = d;
            best = s;
        }
    }
    best
}

fn score_secs_at_layout_pos(
    layout: &LayoutResult,
    song: &bassoxide_core::song::Song,
    selected_track: usize,
    pos: Pos2,
) -> Option<f64> {
    let timeline = score_timeline(song);
    for system in &layout.systems {
        if pos.y < system.y || pos.y > system.y + system.height {
            continue;
        }
        for mp in &system.measure_positions {
            if pos.x < mp.x || pos.x > mp.x + mp.width {
                continue;
            }
            let rel = pos.x - mp.x;
            if let Some(beats) = layout
                .beat_positions
                .get(mp.measure_index)
                .and_then(|tracks| tracks.get(selected_track))
            {
                if !beats.is_empty() {
                    let mut best_i = 0usize;
                    let mut best_d = f32::MAX;
                    for (i, b) in beats.iter().enumerate() {
                        let cx = b.x + b.width * 0.5;
                        let d = (rel - cx).abs();
                        if d < best_d {
                            best_d = d;
                            best_i = i;
                        }
                    }
                    let frac = if beats.len() <= 1 {
                        0.0
                    } else {
                        best_i as f64 / beats.len().saturating_sub(1) as f64
                    };
                    let t = score_secs_in_measure(&timeline, mp.measure_index, frac);
                    return Some(snap_to_nearest_beat(&timeline, t, 0.35));
                }
            }
            let frac = (rel / mp.width.max(1.0)).clamp(0.0, 1.0) as f64;
            let t = score_secs_in_measure(&timeline, mp.measure_index, frac);
            return Some(snap_to_nearest_beat(&timeline, t, 0.35));
        }
    }
    None
}

fn playhead_geometry(
    layout: &LayoutResult,
    song: &bassoxide_core::song::Song,
    secs: f64,
    _selected: usize,
) -> Option<(f32, f32, f32)> {
    let timeline = score_timeline(song);
    if timeline.measure_times.len() < 2 {
        return None;
    }
    let (measure, frac) = bassoxide_audio::measure_at_score_secs(&timeline, secs);
    for system in &layout.systems {
        if let Some(mp) = system
            .measure_positions
            .iter()
            .find(|m| m.measure_index == measure)
        {
            let x = mp.x + mp.width * frac as f32;
            return Some((x, system.y, system.height));
        }
    }
    None
}
