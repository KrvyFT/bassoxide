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

/// 根据小节宽度与节拍间距，得到符杆缩放（小节变窄时符杆变短）
fn stem_fit_scale(beats: &[RhythmBeat], measure_width: f32, settings: &LayoutSettings) -> f32 {
    let ref_measure = (settings.min_measure_width * 2.2).max(120.0);
    let measure_s = (measure_width / ref_measure).clamp(0.38, 1.05);

    let ref_gap = settings.reference_beat_gap();
    let avg_gap = if beats.len() >= 2 {
        let span = (beats.last().unwrap().x - beats.first().unwrap().x).abs();
        span / (beats.len() - 1) as f32
    } else {
        // 单音时按小节内可用宽度估计
        (measure_width * 0.35).max(ref_gap * 0.5)
    };
    let dens_s = (avg_gap / ref_gap).clamp(0.4, 1.05);

    // 取更紧的一侧，保证挤窄小节时符杆明显缩短
    (measure_s * dens_s).sqrt().clamp(0.4, 1.05)
}

/// 绘制一小节的节奏符杆。
///
/// `baseline_y` 为符干顶端 Y（一般是六线谱底线下方一点）。
/// `measure_width` 用于按小节实际宽度压缩符杆。
pub fn draw_measure_rhythm(
    painter: &Painter,
    beats: &[RhythmBeat],
    baseline_y: f32,
    measure_width: f32,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    if beats.is_empty() {
        return;
    }

    let dens = stem_fit_scale(beats, measure_width, settings);
    let stem_len = ((settings.rhythm_height * 0.48).max(settings.tab_font_size * 0.65) * dens)
        .clamp(5.0, settings.rhythm_height.max(8.0));
    let stem_top = baseline_y;
    let stem_bottom = baseline_y + stem_len;
    let stem_stroke = Stroke::new(
        (settings.tab_font_size * 0.085 * dens).clamp(0.7, 2.0),
        theme.note_text,
    );
    let beam_thickness = ((settings.tab_font_size * 0.11).max(settings.rhythm_height * 0.07) * dens)
        .clamp(0.9, 2.8);
    let beam_gap = (beam_thickness + 1.2 * dens).max(1.8);
    let stub_len = (5.0 * dens).max(2.5);
    let dot_r = (1.1 * dens).clamp(0.7, 1.6);
    let flag_scale = dens.clamp(0.4, 1.1);

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
        let lvl = if note {
            beam_level(rb.beat.duration.value)
        } else {
            0
        };
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
                    Pos2::new(
                        rb.x + (3.5 + d as f32 * 3.0) * dens,
                        stem_bottom - 1.5 * dens,
                    ),
                    dot_r,
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
                let dir = if i > 0 && group_id[i - 1] == g {
                    -1.0
                } else {
                    1.0
                };
                painter.line_segment(
                    [
                        Pos2::new(beats[i].x, beam_y),
                        Pos2::new(beats[i].x + dir * stub_len, beam_y),
                    ],
                    Stroke::new(beam_thickness, theme.note_text),
                );
            } else {
                // 完全孤立的音符：画符尾（旗）
                draw_flag(painter, beats[i].x, beam_y, flag_scale, theme);
            }
            i = j + 1;
        }
    }
}

/// 绘制符尾（旗），向右下方弯出
fn draw_flag(painter: &Painter, x: f32, y: f32, scale: f32, theme: &Theme) {
    let stroke = Stroke::new((1.4 * scale).clamp(0.8, 2.0), theme.note_text);
    painter.line_segment(
        [
            Pos2::new(x, y),
            Pos2::new(x + 6.5 * scale, y + 3.5 * scale),
        ],
        stroke,
    );
}
