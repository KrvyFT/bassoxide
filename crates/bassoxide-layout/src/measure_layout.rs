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

/// 计算一个声部内各 Beat 的水平位置
///
/// 使用"按时值比例分配"算法：
/// 时值长的音符占更多水平空间。
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

    // 预留左右边距
    let padding = 8.0;
    let usable_width = (measure_width - padding * 2.0).max(0.0);

    let mut positions = Vec::with_capacity(beat_count);
    let mut x = padding;

    for (i, beat) in voice.beats.iter().enumerate() {
        let tick_ratio = beat.ticks() as f32 / total_ticks as f32;
        let width = (usable_width * tick_ratio).max(settings.min_beat_spacing);

        positions.push(BeatPosition {
            beat_index: i,
            x,
            width,
        });
        x += width;
    }

    positions
}
