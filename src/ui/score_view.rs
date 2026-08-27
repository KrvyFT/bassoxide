//! 乐谱主视图 — 轨道显示器：滚动、缩放、点击定位、播放头。

use egui::{Pos2, ScrollArea, Sense, Stroke, Ui};
use bassoxide_audio::{score_secs_in_measure, score_timeline, snap_to_nearest_beat};
use bassoxide_layout::engine::LayoutResult;
use bassoxide_render::ScorePainter;

use crate::state::AppState;
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

    let (song, layout) = match (&state.song, &state.layout) {
        (Some(s), Some(l)) => (s, l),
        _ => {
            ui.painter().rect_filled(ui.max_rect(), 0.0, palette.surface);
            ui.centered_and_justified(|ui| {
                ui.heading(egui::RichText::new("Bassoxide").color(palette.on_surface));
                ui.label(
                    egui::RichText::new("按 Ctrl+O 打开 Guitar Pro 文件 (.gp5)")
                        .color(palette.on_surface_variant),
                );
            });
            return;
        }
    };

    let playhead = state
        .audio_player
        .as_ref()
        .map(|p| p.score_position_secs())
        .unwrap_or(0.0);
    let selected = state.selected_track;

    let viewport = ui.available_size();
    let page_w = layout.total_width;
    let page_h = layout.total_height;
    let center_pad_x = ((viewport.x - page_w).max(0.0) * 0.5).floor();
    let content_width = (page_w + center_pad_x * 2.0 + 48.0).max(viewport.x);
    let content_height = (page_h + 64.0).max(viewport.y);

    let mut seek_request: Option<f64> = None;

    ScrollArea::both()
        .auto_shrink([false, false])
        .drag_to_scroll(true)
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(content_width, content_height),
                Sense::click(),
            );

            painter.rect_filled(response.rect, 0.0, palette.surface);

            let offset = egui::vec2(
                response.rect.min.x + center_pad_x + 24.0,
                response.rect.min.y + 24.0,
            );

            let score_painter = ScorePainter::new(&state.layout_settings, &state.theme);
            score_painter.paint(&painter, song, layout, offset);

            // 播放头（按谱面时间映射到小节 x）
            if let Some((x, y, h)) = playhead_geometry(layout, song, playhead, selected) {
                let px = x + offset.x;
                let top = y + offset.y;
                painter.line_segment(
                    [Pos2::new(px, top), Pos2::new(px, top + h)],
                    Stroke::new(2.0_f32, palette.error),
                );
            }

            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = Pos2::new(pos.x - offset.x, pos.y - offset.y);
                    if let Some(secs) = score_secs_at_layout_pos(layout, song, selected, local) {
                        seek_request = Some(secs);
                    }
                }
            }
        });

    if let Some(secs) = seek_request {
        // 点击谱面：跳转到对应位置并播放
        state.seek_score_secs(secs, true);
    }
}

/// 布局坐标 → 谱面时间（吸附拍点）
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
            // 优先落到最近 beat 列
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

/// 播放头在布局中的竖线几何
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
