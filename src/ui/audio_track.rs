//! 底部音频同步轨：波形、谱面节拍轴、检测小节线、滚轮平移、Ctrl+滚轮缩放、点击定位。

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use bassoxide_audio::{
    analyze_beats, compute_peaks, decode_file, default_beats_per_bar, score_timeline,
    snap_to_nearest_beat, BeatAnalysis, ScoreTimeline,
};

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 已加载的外部音频轨
pub struct AudioTrack {
    pub path: String,
    pub samples: Arc<Vec<f32>>,
    pub sample_rate: u32,
    pub duration_secs: f64,
    pub peaks: Vec<f32>,
    pub analysis: BeatAnalysis,
    /// 音频相对谱面起点的偏移（秒）：正值 = 音频延后
    pub sync_offset_secs: f64,
    /// 横向缩放：像素 / 秒
    pub pixels_per_second: f32,
    /// 视图起始谱面时间
    pub view_start_secs: f64,
}

impl AudioTrack {
    pub fn load(
        path: &std::path::Path,
        song: Option<&bassoxide_core::song::Song>,
    ) -> Result<Self, String> {
        let decoded = decode_file(path).map_err(|e| e.to_string())?;
        let samples = Arc::new(decoded.samples);
        let sample_rate = decoded.sample_rate;
        let duration_secs = samples.len() as f64 / f64::from(sample_rate.max(1));
        let peaks = compute_peaks(&samples, 2048);
        let bpb = default_beats_per_bar(song);
        let hint = song.map(|s| f64::from(s.tempo));
        let analysis = analyze_beats(&samples, sample_rate, bpb, hint);
        Ok(Self {
            path: path.display().to_string(),
            samples,
            sample_rate,
            duration_secs,
            peaks,
            analysis,
            sync_offset_secs: 0.0,
            pixels_per_second: 80.0,
            view_start_secs: 0.0,
        })
    }

    /// 可见时长（秒）
    pub fn view_span_secs(&self, width_px: f32) -> f64 {
        f64::from(width_px) / f64::from(self.pixels_per_second.max(1.0))
    }
}

pub fn audio_track_panel(ui: &mut Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    let mut load_path: Option<std::path::PathBuf> = None;
    let mut clear = false;
    let mut reset_offset = false;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("音频同步轨")
                .color(palette.on_surface)
                .size(15.0),
        );

        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("添加音频…").color(palette.on_primary),
                )
                .fill(palette.primary)
                .corner_radius(egui::CornerRadius::ZERO),
            )
            .on_hover_text("支持 WAV / FLAC / MP3 / OGG 等")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Audio",
                    &["wav", "flac", "mp3", "ogg", "m4a", "aac", "aiff", "aif"],
                )
                .add_filter("All", &["*"])
                .pick_file()
            {
                load_path = Some(path);
            }
        }

        if state.audio_track.is_some()
            && ui
                .add(egui::Button::new("清除音频").corner_radius(egui::CornerRadius::ZERO))
                .clicked()
        {
            clear = true;
        }

        if let Some(track) = state.audio_track.as_mut() {
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "偏移 {:+.3}s · 检测 {:.1} BPM",
                    track.sync_offset_secs, track.analysis.bpm
                ))
                .size(11.0)
                .color(palette.on_surface_variant),
            );

            if ui
                .add(egui::Button::new("重置偏移").corner_radius(egui::CornerRadius::ZERO))
                .clicked()
            {
                track.sync_offset_secs = 0.0;
                reset_offset = true;
            }

            ui.add(egui::Slider::new(&mut track.pixels_per_second, 20.0..=400.0).text("缩放"));
        }
    });

    if let Some(path) = load_path {
        match AudioTrack::load(&path, state.song.as_ref()) {
            Ok(track) => {
                if let Some(player) = &state.audio_player {
                    player.set_audio(track.samples.clone(), track.sample_rate);
                    player.set_sync_offset(track.sync_offset_secs);
                }
                state.status_message = format!(
                    "已加载音频: {} | {:.1}s | 检测 {:.1} BPM | {} 小节线",
                    track.path,
                    track.duration_secs,
                    track.analysis.bpm,
                    track.analysis.measure_times.len()
                );
                state.audio_track = Some(track);
            }
            Err(e) => {
                state.status_message = format!("音频加载失败: {e}");
            }
        }
    }
    if clear {
        state.audio_track = None;
        if let Some(player) = &state.audio_player {
            player.clear_audio();
        }
        state.status_message = "已清除音频轨".into();
    }
    if reset_offset {
        if let Some(player) = &state.audio_player {
            player.set_sync_offset(0.0);
        }
    }

    ui.add_space(4.0);

    if state.audio_track.is_none() {
        ui.label(
            egui::RichText::new(
                "添加录音/导唱等音频；滚轮平移 · Ctrl+滚轮缩放 · 点击节拍轴定位播放。",
            )
            .size(12.0)
            .color(palette.on_surface_variant),
        );
        return;
    }

    let score_tl = state
        .song
        .as_ref()
        .map(score_timeline)
        .unwrap_or_default();
    let playhead = state
        .audio_player
        .as_ref()
        .map(|p| p.score_position_secs())
        .unwrap_or(0.0);

    let available = ui.available_width();
    let ruler_h = 28.0;
    let wave_h = 72.0;
    let total_h = ruler_h + wave_h + 4.0;

    let (response, painter) =
        ui.allocate_painter(Vec2::new(available, total_h), Sense::click_and_drag());
    let rect = response.rect;

    let mut pending_offset: Option<f64> = None;
    let mut seek_secs: Option<f64> = None;
    let (pps, view0, sync_offset, duration_secs, peaks, beat_times, measure_times) = {
        let track = state.audio_track.as_mut().unwrap();

        // 拖拽：水平拖改偏移；Shift+拖平移视图
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            if delta.x.abs() >= delta.y.abs() && delta.x.abs() > 0.5 {
                let dx = delta.x;
                if ui.input(|i| i.modifiers.shift) {
                    track.view_start_secs -=
                        f64::from(dx) / f64::from(track.pixels_per_second);
                    track.view_start_secs = track.view_start_secs.max(0.0);
                } else {
                    track.sync_offset_secs +=
                        f64::from(dx) / f64::from(track.pixels_per_second);
                    pending_offset = Some(track.sync_offset_secs);
                }
            }
        }

        // 滚轮：默认平移视图；Ctrl+滚轮缩放（以指针为锚点）
        if response.hovered() {
            let (scroll_y, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));
            if scroll_y.abs() > 0.1 {
                if ctrl {
                    let old_pps = track.pixels_per_second;
                    let zoom = 1.0 + scroll_y * 0.002;
                    let new_pps = (old_pps * zoom).clamp(20.0, 400.0);
                    if let Some(pos) = response.hover_pos() {
                        let local_x = pos.x - rect.left();
                        let anchor_t =
                            track.view_start_secs + f64::from(local_x) / f64::from(old_pps.max(1.0));
                        track.pixels_per_second = new_pps;
                        track.view_start_secs =
                            (anchor_t - f64::from(local_x) / f64::from(new_pps)).max(0.0);
                    } else {
                        track.pixels_per_second = new_pps;
                    }
                } else {
                    // 滚轮向下 → 视图向右（时间增大）
                    track.view_start_secs -=
                        f64::from(scroll_y) / f64::from(track.pixels_per_second);
                    track.view_start_secs = track.view_start_secs.max(0.0);
                }
                ui.ctx().input_mut(|i| {
                    i.smooth_scroll_delta.y = 0.0;
                    i.raw_scroll_delta.y = 0.0;
                });
            }
        }

        // 单击（非拖拽）→ 定位播放头；靠近拍点时吸附
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let score_t = track.view_start_secs
                    + f64::from(pos.x - rect.left()) / f64::from(track.pixels_per_second.max(1.0));
                let snap_window = (12.0 / track.pixels_per_second) as f64;
                let snapped = if score_tl.beat_times.is_empty() {
                    score_t.max(0.0)
                } else {
                    snap_to_nearest_beat(&score_tl, score_t.max(0.0), snap_window.max(0.05))
                };
                seek_secs = Some(snapped);
            }
        }

        (
            track.pixels_per_second,
            track.view_start_secs,
            track.sync_offset_secs,
            track.duration_secs,
            track.peaks.clone(),
            track.analysis.beat_times.clone(),
            track.analysis.measure_times.clone(),
        )
    };

    if let Some(off) = pending_offset {
        if let Some(player) = &state.audio_player {
            player.set_sync_offset(off);
        }
    }
    if let Some(t) = seek_secs {
        state.seek_score_secs(t, false);
    }

    let to_x = |score_t: f64| -> f32 { rect.left() + ((score_t - view0) as f32) * pps };

    painter.rect_filled(rect, egui::CornerRadius::ZERO, palette.surface_container_high);

    let ruler_rect = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top()),
        Pos2::new(rect.right(), rect.top() + ruler_h),
    );
    let wave_rect = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + ruler_h + 2.0),
        Pos2::new(rect.right(), rect.bottom()),
    );

    painter.rect_filled(ruler_rect, egui::CornerRadius::ZERO, palette.surface_container);
    painter.text(
        Pos2::new(ruler_rect.left() + 6.0, ruler_rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        "谱面节拍",
        egui::FontId::proportional(10.0),
        palette.on_surface_variant,
    );

    draw_score_axis(
        &painter,
        ruler_rect,
        &score_tl,
        &to_x,
        palette.outline,
        palette.primary,
        palette.on_surface_variant,
    );

    painter.rect_filled(wave_rect, egui::CornerRadius::ZERO, palette.surface);
    draw_waveform_peaks(
        &painter,
        wave_rect,
        &peaks,
        duration_secs,
        sync_offset,
        view0,
        pps,
        palette.primary.gamma_multiply(0.55),
    );

    draw_audio_grid_times(
        &painter,
        wave_rect,
        &beat_times,
        &measure_times,
        sync_offset,
        view0,
        pps,
        Color32::from_rgba_unmultiplied(0x4F, 0x63, 0x57, 90),
        palette.secondary,
    );

    let ph_x = to_x(playhead);
    if ph_x >= rect.left() && ph_x <= rect.right() {
        painter.line_segment(
            [Pos2::new(ph_x, rect.top()), Pos2::new(ph_x, rect.bottom())],
            Stroke::new(2.0_f32, palette.error),
        );
    }

    painter.text(
        Pos2::new(wave_rect.left() + 6.0, wave_rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "拖动对齐 · Shift+拖平移 · 滚轮滚动 · Ctrl+滚轮缩放 · 点击定位",
        egui::FontId::proportional(10.0),
        palette.on_surface_variant,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
}

fn draw_score_axis(
    painter: &egui::Painter,
    rect: Rect,
    timeline: &ScoreTimeline,
    to_x: &dyn Fn(f64) -> f32,
    beat_color: Color32,
    measure_color: Color32,
    text_color: Color32,
) {
    let view_left = rect.left();
    let view_right = rect.right();

    for (i, &t) in timeline.measure_times.iter().enumerate() {
        let x = to_x(t);
        if x < view_left - 2.0 || x > view_right + 2.0 {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(x, rect.top() + 10.0),
                Pos2::new(x, rect.bottom() - 2.0),
            ],
            Stroke::new(1.5_f32, measure_color),
        );
        painter.text(
            Pos2::new(x + 3.0, rect.top() + 10.0),
            egui::Align2::LEFT_TOP,
            format!("M{}", i + 1),
            egui::FontId::proportional(9.0),
            text_color,
        );
    }

    for &t in &timeline.beat_times {
        let x = to_x(t);
        if x < view_left || x > view_right {
            continue;
        }
        let on_measure = timeline
            .measure_times
            .iter()
            .any(|&m| (m - t).abs() < 1e-4);
        if on_measure {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(x, rect.center().y),
                Pos2::new(x, rect.bottom() - 2.0),
            ],
            Stroke::new(1.0_f32, beat_color),
        );
    }
}

fn draw_waveform_peaks(
    painter: &egui::Painter,
    rect: Rect,
    peaks: &[f32],
    duration_secs: f64,
    sync_offset: f64,
    view0: f64,
    pps: f32,
    color: Color32,
) {
    if peaks.is_empty() || duration_secs <= 0.0 {
        return;
    }
    let mid_y = rect.center().y;
    let amp = rect.height() * 0.42;
    let n = peaks.len();

    for (i, &peak) in peaks.iter().enumerate() {
        let audio_t = (i as f64 + 0.5) / n as f64 * duration_secs;
        let score_t = audio_t + sync_offset;
        let x = rect.left() + ((score_t - view0) as f32) * pps;
        if x < rect.left() - 1.0 || x > rect.right() + 1.0 {
            continue;
        }
        let h = peak * amp;
        painter.line_segment(
            [Pos2::new(x, mid_y - h), Pos2::new(x, mid_y + h)],
            Stroke::new(1.0_f32, color),
        );
    }
}

fn draw_audio_grid_times(
    painter: &egui::Painter,
    rect: Rect,
    beat_times: &[f64],
    measure_times: &[f64],
    sync_offset: f64,
    view0: f64,
    pps: f32,
    beat_color: Color32,
    measure_color: Color32,
) {
    for &audio_t in beat_times {
        let score_t = audio_t + sync_offset;
        let x = rect.left() + ((score_t - view0) as f32) * pps;
        if x < rect.left() || x > rect.right() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(x, rect.top() + 4.0),
                Pos2::new(x, rect.bottom() - 4.0),
            ],
            Stroke::new(1.0_f32, beat_color),
        );
    }
    for (i, &audio_t) in measure_times.iter().enumerate() {
        let score_t = audio_t + sync_offset;
        let x = rect.left() + ((score_t - view0) as f32) * pps;
        if x < rect.left() - 2.0 || x > rect.right() + 2.0 {
            continue;
        }
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(2.0_f32, measure_color),
        );
        painter.text(
            Pos2::new(x + 3.0, rect.bottom() - 14.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{}", i + 1),
            egui::FontId::proportional(9.0),
            measure_color,
        );
    }
}
