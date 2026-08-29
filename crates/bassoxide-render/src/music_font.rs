//! SMuFL 乐谱字体（Bravura）码点与 FontFamily 辅助。

use egui::FontFamily;

use bassoxide_core::types::{Duration, NoteValue};

/// egui 中注册的乐谱字体族名
pub const MUSIC_FAMILY_NAME: &str = "Bravura";

/// 返回乐谱 FontFamily（需在 App 启动时注册 Bravura 字体数据）
pub fn music_family() -> FontFamily {
    FontFamily::Name(MUSIC_FAMILY_NAME.into())
}

// ── SMuFL Rests (U+E4E0–U+E4FF) ──
pub const REST_WHOLE: char = '\u{E4F4}';
pub const REST_HALF: char = '\u{E4F5}';
pub const REST_QUARTER: char = '\u{E4E5}';
pub const REST_8TH: char = '\u{E4E6}';
pub const REST_16TH: char = '\u{E4E7}';
pub const REST_32ND: char = '\u{E4E8}';
pub const REST_64TH: char = '\u{E4E9}';

// ── SMuFL Flags down（符尾向下，锚点在符干底端） ──
pub const FLAG_8TH_DOWN: char = '\u{E241}';
pub const FLAG_16TH_DOWN: char = '\u{E243}';
pub const FLAG_32ND_DOWN: char = '\u{E245}';
pub const FLAG_64TH_DOWN: char = '\u{E247}';

/// 时值 → 休止符字形
pub fn rest_glyph(value: NoteValue) -> char {
    match value {
        NoteValue::Whole => REST_WHOLE,
        NoteValue::Half => REST_HALF,
        NoteValue::Quarter => REST_QUARTER,
        NoteValue::Eighth => REST_8TH,
        NoteValue::Sixteenth => REST_16TH,
        NoteValue::ThirtySecond => REST_32ND,
        NoteValue::SixtyFourth => REST_64TH,
    }
}

/// beam level → 向下符尾字形（孤立音符用）
pub fn flag_glyph_down(level: u8) -> Option<char> {
    match level {
        1 => Some(FLAG_8TH_DOWN),
        2 => Some(FLAG_16TH_DOWN),
        3 => Some(FLAG_32ND_DOWN),
        4 => Some(FLAG_64TH_DOWN),
        _ => None,
    }
}

fn value_ticks(value: NoteValue) -> u32 {
    Duration {
        value,
        ..Duration::default()
    }
    .ticks()
}

/// 将连续休止的总 tick 贪心拆成尽量少的标准休止时值（大→小）
pub fn merge_rest_values(total_ticks: u32) -> Vec<NoteValue> {
    if total_ticks == 0 {
        return Vec::new();
    }
    let order = [
        NoteValue::Whole,
        NoteValue::Half,
        NoteValue::Quarter,
        NoteValue::Eighth,
        NoteValue::Sixteenth,
        NoteValue::ThirtySecond,
        NoteValue::SixtyFourth,
    ];
    let mut remaining = total_ticks;
    let mut out = Vec::new();
    for &v in &order {
        let t = value_ticks(v);
        if t == 0 {
            continue;
        }
        while remaining >= t {
            out.push(v);
            remaining -= t;
        }
    }
    out
}

/// 一次合并后的休止符绘制项（显示层；不改数据网格）
#[derive(Debug, Clone, Copy)]
pub struct MergedRestDraw {
    pub x: f32,
    pub value: NoteValue,
}

/// 根据 voice beats + 布局拍位，生成合并后的休止符绘制列表
pub fn plan_merged_rests(
    beats: &[bassoxide_core::beat::Beat],
    beat_positions: &[bassoxide_layout::measure_layout::BeatPosition],
    measure_x: f32,
) -> Vec<MergedRestDraw> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < beat_positions.len() {
        let Some(b0) = beats.get(beat_positions[i].beat_index) else {
            i += 1;
            continue;
        };
        if !b0.is_empty() {
            i += 1;
            continue;
        }
        let mut ticks = 0u32;
        let mut xs = Vec::new();
        while i < beat_positions.len() {
            let Some(b) = beats.get(beat_positions[i].beat_index) else {
                break;
            };
            if !b.is_empty() {
                break;
            }
            ticks = ticks.saturating_add(b.ticks());
            xs.push(measure_x + beat_positions[i].x);
            i += 1;
        }
        if xs.is_empty() {
            continue;
        }
        let values = merge_rest_values(ticks);
        if values.is_empty() {
            continue;
        }
        let x0 = *xs.first().unwrap();
        let x1 = *xs.last().unwrap();
        let span = (x1 - x0).max(1.0);
        let single = values.len() == 1;
        let mut acc = 0u32;
        for v in values {
            let t = value_ticks(v);
            let mid_tick = acc + t / 2;
            let frac = if ticks == 0 {
                0.0
            } else {
                mid_tick as f32 / ticks as f32
            };
            let x = if single {
                (x0 + x1) * 0.5
            } else {
                x0 + span * frac.clamp(0.0, 1.0)
            };
            out.push(MergedRestDraw { x, value: v });
            acc += t;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_eighths_merge_to_whole() {
        assert_eq!(merge_rest_values(3840), vec![NoteValue::Whole]);
    }

    #[test]
    fn four_eighths_merge_to_half() {
        assert_eq!(merge_rest_values(1920), vec![NoteValue::Half]);
    }

    #[test]
    fn three_eighths_to_quarter_plus_eighth() {
        assert_eq!(
            merge_rest_values(1440),
            vec![NoteValue::Quarter, NoteValue::Eighth]
        );
    }
}
