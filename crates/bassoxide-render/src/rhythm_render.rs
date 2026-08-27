//! 六线谱下方的节奏符杆渲染（符干 stem、符杠 beam、符尾 flag、附点 dot）。
//!
//! Guitar Pro 风格：每个 beat 在六线谱下方画一根向下的符干，
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

/// 绘制一小节的节奏符杆。
///
/// `baseline_y` 为符干顶端 Y（一般是六线谱底线下方一点）。
pub fn draw_measure_rhythm(
    painter: &Painter,
    beats: &[RhythmBeat],
    baseline_y: f32,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    if beats.is_empty() {
        return;
    }

    let stem_len = (settings.rhythm_height * 0.62).max(10.0);
    let stem_top = baseline_y;
    let stem_bottom = baseline_y + stem_len;
    let stem_stroke = Stroke::new(1.3_f32, theme.note_text);
    let beam_thickness = (settings.rhythm_height * 0.09).clamp(1.6, 2.6);
    let beam_gap = beam_thickness + 1.6;

    // 计算每个 beat 的节奏属性
    let n = beats.len();
    let mut levels = vec![0u8; n];
    let mut is_note = vec![false; n];
    let mut group_id = vec![usize::MAX; n]; // beam 分组
    let mut running_tick: u32 = 0;
    let mut cur_group = 0usize;
    let mut prev_in_group = false;

    for (i, rb) in beats.iter().enumerate() {
        let note = !rb.beat.is_empty();
        is_note[i] = note;
        let lvl = if note { beam_level(rb.beat.duration.value) } else { 0 };
        levels[i] = lvl;

        // 判断是否可与前一个连成一组：均为音符、均可连杠、且不跨越四分拍边界
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

    // 统计每组大小
    let mut group_size = std::collections::HashMap::new();
    for &g in &group_id {
        if g != usize::MAX {
            *group_size.entry(g).or_insert(0usize) += 1;
        }
    }

    // 1. 画符干（休止符与全音符不画）
    for (i, rb) in beats.iter().enumerate() {
        if !is_note[i] {
            continue;
        }
        if rb.beat.duration.value == NoteValue::Whole {
            continue;
        }
        painter.line_segment(
            [Pos2::new(rb.x, stem_top), Pos2::new(rb.x, stem_bottom)],
            stem_stroke,
        );

        // 附点
        if rb.beat.duration.dotted || rb.beat.duration.double_dotted {
            let dots = if rb.beat.duration.double_dotted { 2 } else { 1 };
            for d in 0..dots {
                painter.circle_filled(
                    Pos2::new(rb.x + 4.0 + d as f32 * 3.5, stem_bottom - 2.0),
                    1.4,
                    theme.note_text,
                );
            }
        }
    }

    // 2. 画符杠 / 符尾
    for level in 1..=4u8 {
        let beam_y = stem_bottom - (level as f32 - 1.0) * beam_gap;
        let mut i = 0usize;
        while i < n {
            if !(is_note[i] && levels[i] >= level) {
                i += 1;
                continue;
            }
            // 找到当前组内该级别的连续段
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
                // 连续 >=2 个：画完整符杠
                painter.line_segment(
                    [Pos2::new(beats[i].x, beam_y), Pos2::new(beats[j].x, beam_y)],
                    Stroke::new(beam_thickness, theme.note_text),
                );
            } else if in_group {
                // 组内但该级别孤立：画一小段部分符杠（指向组内方向）
                let dir = if i > 0 && group_id[i - 1] == g { -1.0 } else { 1.0 };
                let stub = 6.0;
                painter.line_segment(
                    [
                        Pos2::new(beats[i].x, beam_y),
                        Pos2::new(beats[i].x + dir * stub, beam_y),
                    ],
                    Stroke::new(beam_thickness, theme.note_text),
                );
            } else {
                // 完全孤立的音符：画符尾（旗）
                draw_flag(painter, beats[i].x, beam_y, beam_gap, theme);
            }
            i = j + 1;
        }
    }
}

/// 绘制符尾（旗），向右下方弯出
fn draw_flag(painter: &Painter, x: f32, y: f32, _gap: f32, theme: &Theme) {
    let stroke = Stroke::new(1.6_f32, theme.note_text);
    painter.line_segment([Pos2::new(x, y), Pos2::new(x + 7.0, y + 4.0)], stroke);
}
