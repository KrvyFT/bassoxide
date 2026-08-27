//! 小节内部布局：音符的水平分配。

use bassoxide_core::beat::Voice;

use crate::spacing::LayoutSettings;

/// 小节内每个 Beat 的水平位置
#[derive(Debug, Clone)]
pub struct BeatPosition {
    /// Beat 在 voice 中的索引
    pub beat_index: usize,
    /// 相对于小节左边的 X 偏移
    pub x: f32,
    /// 分配的宽度
    pub width: f32,
}

/// 计算一个声部内各 Beat 的水平位置。
///
/// 按时值比例分配；若 `min_beat_spacing` 导致总宽超出小节，
/// 则按比例压缩，保证所有音符落在小节（谱表）水平范围内。
pub fn layout_voice_beats(
    voice: &Voice,
    measure_width: f32,
    settings: &LayoutSettings,
) -> Vec<BeatPosition> {
    if voice.is_empty() {
        return vec![];
    }

    let beat_count = voice.beats.len();
    let total_ticks = voice.total_ticks().max(1);

    let padding = (8.0 * settings.content_scale).clamp(4.0, 12.0);
    let usable_width = (measure_width - padding * 2.0).max(0.0);

    let mut raw_widths = Vec::with_capacity(beat_count);
    let mut raw_sum = 0.0f32;
    for beat in &voice.beats {
        let tick_ratio = beat.ticks() as f32 / total_ticks as f32;
        let width = (usable_width * tick_ratio).max(settings.min_beat_spacing);
        raw_sum += width;
        raw_widths.push(width);
    }

    // 硬约束：音符必须在小节内 → 超宽则等比压缩
    let scale = if raw_sum > usable_width && raw_sum > 0.0 {
        usable_width / raw_sum
    } else {
        1.0
    };

    let mut positions = Vec::with_capacity(beat_count);
    let mut x = padding;
    for (i, w) in raw_widths.into_iter().enumerate() {
        let width = w * scale;
        positions.push(BeatPosition {
            beat_index: i,
            x,
            width,
        });
        x += width;
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use bassoxide_core::beat::Beat;
    use bassoxide_core::types::Duration;

    #[test]
    fn beats_never_exceed_measure_width() {
        let mut voice = Voice::default();
        for _ in 0..16 {
            voice.beats.push(Beat {
                duration: Duration::default(),
                ..Default::default()
            });
        }
        let settings = LayoutSettings {
            min_beat_spacing: 30.0,
            ..Default::default()
        };
        let measure_w = 120.0;
        let positions = layout_voice_beats(&voice, measure_w, &settings);
        let end = positions
            .last()
            .map(|p| p.x + p.width)
            .unwrap_or(0.0);
        assert!(end <= measure_w + 0.5, "end={end} measure={measure_w}");
    }
}
