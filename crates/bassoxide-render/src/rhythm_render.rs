//! 六线谱下方的节奏符杆渲染（符干 stem、符杠 beam、符尾 flag、附点 dot）。
//!
//! Guitar Pro 风格：每个 beat 从根音数字下方起画向下的符干，
//! 时值短于四分音符时用符杠连接相邻音符，孤立音符画符尾，附点画圆点。

use egui::{Painter, Pos2, Stroke};

use bassoxide_core::beat::Beat;
use bassoxide_core::types::NoteValue;
use bassoxide_layout::spacing::LayoutSettings;

use crate::colors::Theme;

/// 单个 beat 在一小节内的绘制信息
pub struct RhythmBeat<'a> {
    /// 绝对 X 坐标（符干所在位置）
    pub x: f32,
    /// 符干顶端 Y：根音品格数字下沿
    pub stem_top: f32,
    pub beat: &'a Beat,
}

/// beam / flag 级别：八分=1，十六分=2，三十二分=3，六十四分=4，四分及更长=0
fn beam_level(value: NoteValue) -> u8 {
    match value {
        NoteValue::Eighth => 1,
        NoteValue::Sixteenth => 2,
        NoteValue::ThirtySecond => 3,
        NoteValue::SixtyFourth => 4,
        _ => 0,
    }
}

/// 根据小节宽度与节拍间距，得到符杆缩放（小节变窄时符杆变短）
fn stem_fit_scale(beats: &[RhythmBeat], measure_width: f32, settings: &LayoutSettings) -> f32 {
    let ref_measure = (settings.min_measure_width * 2.2).max(120.0);
    let measure_s = (measure_width / ref_measure).clamp(0.55, 1.05);

    let ref_gap = settings.reference_beat_gap();
    let avg_gap = if beats.len() >= 2 {
        let span = (beats.last().unwrap().x - beats.first().unwrap().x).abs();
        span / (beats.len() - 1) as f32
    } else {
        (measure_width * 0.35).max(ref_gap * 0.5)
    };
    let dens_s = (avg_gap / ref_gap).clamp(0.55, 1.05);

    (measure_s * dens_s).sqrt().clamp(0.55, 1.05)
}

/// 绘制一小节的节奏符杆。
///
/// 每个 `RhythmBeat.stem_top` 为该拍根音数字下沿；符干向下画到统一底部以便连杠。
/// `measure_width` 用于按小节实际宽度压缩符杆。
pub fn draw_measure_rhythm(
    painter: &Painter,
    beats: &[RhythmBeat],
    measure_width: f32,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    if beats.is_empty() {
        return;
    }

    let dens = stem_fit_scale(beats, measure_width, settings);
    let max_stem = (settings.rhythm_height - 1.0).max(8.0);
    // 成品谱：符干略长，保证与底弦有清晰空隙后仍够连杠
    let stem_len = ((settings.rhythm_height * 0.92).max(settings.tab_font_size * 1.55) * dens)
        .clamp(14.0, max_stem);
    let stem_w = (settings.tab_font_size * 0.11 * dens).clamp(1.0, 2.4);
    let stem_stroke = Stroke::new(stem_w, theme.note_text);
    // 符杠：粗实心横条（接近 GP / 参考图）
    let beam_thickness = ((settings.tab_font_size * 0.28).max(settings.rhythm_height * 0.12) * dens)
        .clamp(2.2, 4.5);
    let beam_gap = (beam_thickness + 1.8 * dens).max(2.8);
    let stub_len = (7.0 * dens).max(3.5);
    let dot_r = (1.35 * dens).clamp(0.9, 2.0);
    let flag_size = (settings.tab_font_size * 1.65 * dens).clamp(14.0, 34.0);

    let n = beats.len();
    let mut levels = vec![0u8; n];
    let mut is_note = vec![false; n];
    let mut group_id = vec![usize::MAX; n];
    let mut running_tick: u32 = 0;
    let mut cur_group = 0usize;
    let mut prev_in_group = false;

    for (i, rb) in beats.iter().enumerate() {
        let note = !rb.beat.is_empty();
        is_note[i] = note;
        let lvl = if note {
            beam_level(rb.beat.duration.value)
        } else {
            0
        };
        levels[i] = lvl;

        let quarter_boundary_crossed = running_tick % 960 == 0 && i > 0;
        if note && lvl >= 1 {
            if prev_in_group && !quarter_boundary_crossed {
                group_id[i] = cur_group;
            } else {
                cur_group += 1;
                group_id[i] = cur_group;
            }
            prev_in_group = true;
        } else {
            prev_in_group = false;
        }

        running_tick += rb.beat.ticks();
    }

    let mut group_size = std::collections::HashMap::new();
    for &g in &group_id {
        if g != usize::MAX {
            *group_size.entry(g).or_insert(0usize) += 1;
        }
    }

    // 有音符的拍：统一符干底边 = 各拍 stem_top 的最大者 + stem_len
    let mut stem_bottom_of = vec![0.0_f32; n];
    let max_top = beats
        .iter()
        .enumerate()
        .filter(|(i, _)| is_note[*i])
        .map(|(_, b)| b.stem_top)
        .fold(f32::NEG_INFINITY, f32::max);
    let shared_bottom = if max_top.is_finite() {
        max_top + stem_len
    } else {
        0.0
    };

    for (i, rb) in beats.iter().enumerate() {
        if !is_note[i] {
            continue;
        }
        if rb.beat.duration.value == NoteValue::Whole {
            continue;
        }
        let top = rb.stem_top;
        let bottom = shared_bottom.max(top + stem_len * 0.55);
        stem_bottom_of[i] = bottom;
        painter.line_segment(
            [Pos2::new(rb.x, top), Pos2::new(rb.x, bottom)],
            stem_stroke,
        );

        if rb.beat.duration.dotted || rb.beat.duration.double_dotted {
            let dots = if rb.beat.duration.double_dotted { 2 } else { 1 };
            for d in 0..dots {
                painter.circle_filled(
                    Pos2::new(rb.x + (3.5 + d as f32 * 3.0) * dens, bottom - 1.5 * dens),
                    dot_r,
                    theme.note_text,
                );
            }
        }
    }

    let beam_base = if shared_bottom > 0.0 {
        shared_bottom
    } else {
        beats.first().map(|b| b.stem_top + stem_len).unwrap_or(0.0)
    };

    for level in 1..=4u8 {
        let beam_y = beam_base - (level as f32 - 1.0) * beam_gap;
        let mut i = 0usize;
        while i < n {
            if !(is_note[i] && levels[i] >= level) {
                i += 1;
                continue;
            }
            let g = group_id[i];
            let in_group = g != usize::MAX && group_size.get(&g).copied().unwrap_or(0) >= 2;

            let mut j = i;
            while j + 1 < n
                && is_note[j + 1]
                && levels[j + 1] >= level
                && group_id[j + 1] == g
                && in_group
            {
                j += 1;
            }

            if j > i {
                // 实心符杠矩形，观感更接近成品谱
                let x0 = beats[i].x.min(beats[j].x);
                let x1 = beats[i].x.max(beats[j].x);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        Pos2::new(x0, beam_y - beam_thickness * 0.5),
                        Pos2::new(x1, beam_y + beam_thickness * 0.5),
                    ),
                    0.0,
                    theme.note_text,
                );
            } else if in_group {
                let dir = if i > 0 && group_id[i - 1] == g {
                    -1.0
                } else {
                    1.0
                };
                let x0 = beats[i].x;
                let x1 = beats[i].x + dir * stub_len;
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        Pos2::new(x0.min(x1), beam_y - beam_thickness * 0.5),
                        Pos2::new(x0.max(x1), beam_y + beam_thickness * 0.5),
                    ),
                    0.0,
                    theme.note_text,
                );
            } else if level == levels[i] {
                // 仅最深一层画一次符尾；贴本拍符干底端并与符杆重叠衔接
                let stem_bot = if stem_bottom_of[i] > 0.0 {
                    stem_bottom_of[i]
                } else {
                    beam_base
                };
                draw_flag_glyph(
                    painter,
                    beats[i].x,
                    stem_bot,
                    levels[i],
                    flag_size,
                    stem_w,
                    theme,
                );
            }
            i = j + 1;
        }
    }
}

/// Bravura 符尾字形（向下）：左上角锚在符干底端，再叠一小段符干保证接缝相连
fn draw_flag_glyph(
    painter: &Painter,
    x: f32,
    stem_bottom: f32,
    level: u8,
    size: f32,
    stem_w: f32,
    theme: &Theme,
) {
    let Some(glyph) = crate::music_font::flag_glyph_down(level) else {
        return;
    };
    let font = egui::FontId::new(size, crate::music_font::music_family());
    // egui 文本框顶边对齐；略上移让旗头顶进符干，消除缝隙
    let attach_y = stem_bottom - (size * 0.06).clamp(0.8, 2.5);
    painter.text(
        Pos2::new(x - stem_w * 0.35, attach_y),
        egui::Align2::LEFT_TOP,
        glyph.to_string(),
        font,
        theme.note_text,
    );
    // 符干末端再盖一层，视觉上与符尾连成一体
    let join = (size * 0.12).clamp(2.0, 5.0);
    painter.line_segment(
        [
            Pos2::new(x, stem_bottom - join),
            Pos2::new(x, stem_bottom + join * 0.35),
        ],
        Stroke::new(stem_w, theme.note_text),
    );
}

/// 根音弦号：和弦中 MIDI 最低音所在弦；无音则 None
pub fn root_string(beat: &Beat) -> Option<u8> {
    beat.notes
        .iter()
        .min_by_key(|n| (n.midi_note, std::cmp::Reverse(n.string)))
        .map(|n| n.string)
}
