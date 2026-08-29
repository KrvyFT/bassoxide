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

fn plain(value: NoteValue) -> Duration {
    Duration {
        value,
        dotted: false,
        double_dotted: false,
        ..Duration::default()
    }
}

fn dotted(value: NoteValue) -> Duration {
    Duration {
        value,
        dotted: true,
        double_dotted: false,
        ..Duration::default()
    }
}

/// 休止符合并候选：大→小，同 tick 时优先无附点的写法（更少符号）
fn rest_merge_candidates() -> Vec<Duration> {
    // 附点优先于「更大无附点拆分」：如 1440 → 附点四分，而非 四分+八分
    [
        plain(NoteValue::Whole),
        dotted(NoteValue::Half), // 2880
        plain(NoteValue::Half),
        dotted(NoteValue::Quarter), // 1440
        plain(NoteValue::Quarter),
        dotted(NoteValue::Eighth), // 720
        plain(NoteValue::Eighth),
        dotted(NoteValue::Sixteenth), // 360
        plain(NoteValue::Sixteenth),
        dotted(NoteValue::ThirtySecond), // 180
        plain(NoteValue::ThirtySecond),
        plain(NoteValue::SixtyFourth),
    ]
    .into_iter()
    .collect()
}

/// 将连续休止的总 tick 贪心拆成尽量少的标准休止时值（含附点）
pub fn merge_rest_durations(total_ticks: u32) -> Vec<Duration> {
    if total_ticks == 0 {
        return Vec::new();
    }
    let candidates = rest_merge_candidates();
    let mut remaining = total_ticks;
    let mut out = Vec::new();
    // 多轮扫描：每轮取能装下的最大候选（列表已按大致从大到小排列）
    'outer: while remaining > 0 {
        for c in &candidates {
            let t = c.ticks();
            if t > 0 && remaining >= t {
                out.push(*c);
                remaining -= t;
                continue 'outer;
            }
        }
        // 无法再拆（理论不应发生）
        break;
    }
    out
}

/// 兼容旧名：仅返回 NoteValue（无附点信息）
pub fn merge_rest_values(total_ticks: u32) -> Vec<NoteValue> {
    merge_rest_durations(total_ticks)
        .into_iter()
        .map(|d| d.value)
        .collect()
}

/// 一次合并后的休止符绘制项（显示层；不改数据网格）
#[derive(Debug, Clone, Copy)]
pub struct MergedRestDraw {
    pub x: f32,
    pub duration: Duration,
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
        let durations = merge_rest_durations(ticks);
        if durations.is_empty() {
            continue;
        }
        let x0 = *xs.first().unwrap();
        let x1 = *xs.last().unwrap();
        let span = (x1 - x0).max(1.0);
        let single = durations.len() == 1;
        let mut acc = 0u32;
        for d in durations {
            let t = d.ticks();
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
            out.push(MergedRestDraw { x, duration: d });
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
        let d = merge_rest_durations(3840);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].value, NoteValue::Whole);
        assert!(!d[0].dotted);
    }

    #[test]
    fn four_eighths_merge_to_half() {
        let d = merge_rest_durations(1920);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].value, NoteValue::Half);
        assert!(!d[0].dotted);
    }

    #[test]
    fn three_eighths_merge_to_dotted_quarter() {
        let d = merge_rest_durations(1440);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].value, NoteValue::Quarter);
        assert!(d[0].dotted);
    }

    #[test]
    fn six_eighths_merge_to_dotted_half() {
        let d = merge_rest_durations(2880);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].value, NoteValue::Half);
        assert!(d[0].dotted);
    }

    #[test]
    fn five_eighths_to_half_plus_eighth() {
        let d = merge_rest_durations(2400);
        assert_eq!(
            d.iter().map(|x| (x.value, x.dotted)).collect::<Vec<_>>(),
            vec![
                (NoteValue::Half, false),
                (NoteValue::Eighth, false),
            ]
        );
    }
}
