//! 排版引擎主入口。
//!
//! 将 `Song` 数据转换为 `LayoutResult`，包含所有元素的屏幕坐标。

use bassoxide_core::song::Song;

use crate::measure_layout::{layout_voice_beats, BeatPosition};
use crate::spacing::LayoutSettings;
use crate::staff::{StaffLayout, StaffType};
use crate::system::{MeasurePosition, SystemLayout};

/// 排版引擎
pub struct LayoutEngine {
    pub settings: LayoutSettings,
}

/// 完整的布局结果
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// 所有 System 行
    pub systems: Vec<SystemLayout>,
    /// 总内容高度
    pub total_height: f32,
    /// 总内容宽度
    pub total_width: f32,
    /// 每个小节中每个轨道的 beat 位置
    /// [measure_index][track_index] -> Vec<BeatPosition>
    pub beat_positions: Vec<Vec<Vec<BeatPosition>>>,
}

impl LayoutEngine {
    pub fn new(settings: LayoutSettings) -> Self {
        Self { settings }
    }

    /// 执行完整排版
    pub fn layout(&self, song: &Song) -> LayoutResult {
        if song.tracks.is_empty() || song.master_bars.is_empty() {
            return LayoutResult {
                systems: vec![],
                total_height: 0.0,
                total_width: self.settings.available_width,
                beat_positions: vec![],
            };
        }

        // 1. 计算每个小节的理想宽度
        let measure_widths = self.compute_measure_widths(song);

        // 2. 将小节分配到 System 行（自动换行）
        let system_ranges = self.break_into_systems(&measure_widths);

        // 3. 为每个 System 计算详细布局
        let mut systems = Vec::new();
        let mut y = self.settings.margin_top;

        for (start, end) in &system_ranges {
            let system = self.layout_system(song, *start, *end, y, &measure_widths);
            y += system.height + self.settings.system_gap;
            systems.push(system);
        }

        // 4. 计算 beat 位置
        let beat_positions = self.compute_beat_positions(song, &systems);

        let mut max_width = self.settings.available_width;
        if let Some(sys) = systems.first() {
            if let Some(last) = sys.measure_positions.last() {
                max_width = last.x + last.width + self.settings.margin_left;
            }
        }

        LayoutResult {
            systems,
            total_height: y,
            total_width: max_width,
            beat_positions,
        }
    }

    /// 计算每个小节的理想宽度（基于内容复杂度）
    fn compute_measure_widths(&self, song: &Song) -> Vec<f32> {
        let measure_count = song.measure_count();
        let mut widths = Vec::with_capacity(measure_count);

        for m in 0..measure_count {
            // 找所有轨道中此小节的最大 beat 数
            let max_beats = song
                .tracks
                .iter()
                .filter_map(|t| t.measures.get(m))
                .map(|measure| {
                    measure
                        .voices
                        .iter()
                        .map(|v| v.beats.len())
                        .max()
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(1)
                .max(1);

            let width = (max_beats as f32 * self.settings.min_beat_spacing + 20.0)
                .max(self.settings.min_measure_width);
            widths.push(width);
        }

        widths
    }

    /// 将小节分配到行（改为单行无尽横向滚动）
    fn break_into_systems(&self, widths: &[f32]) -> Vec<(usize, usize)> {
        if widths.is_empty() {
            return vec![];
        }
        vec![(0, widths.len())]
    }

    /// 布局单个 System
    fn layout_system(
        &self,
        song: &Song,
        start: usize,
        end: usize,
        y: f32,
        measure_widths: &[f32],
    ) -> SystemLayout {
        let s = &self.settings;

        // 横向滚动模式下，不再按可用屏幕宽度拉伸小节，保持自然宽度 (scale = 1.0)
        let preamble_width = s.clef_width + s.time_sig_width;
        let scale = 1.0;

        // 小节位置
        let mut measure_positions = Vec::new();
        let mut x = s.margin_left + preamble_width;
        for m in start..end {
            let width = measure_widths[m] * scale;
            measure_positions.push(MeasurePosition {
                measure_index: m,
                x,
                width,
            });
            x += width;
        }

        // 各轨道的谱表
        let mut staves = Vec::new();
        let mut staff_y = 0.0;
        for (track_idx, track) in song.tracks.iter().enumerate() {
            let string_count = track.string_count();

            // 1. 五线谱 (Standard)
            let standard_height = 24.0; // 五线谱高度压缩
            staves.push(StaffLayout {
                staff_type: StaffType::Standard,
                track_index: track_idx,
                string_count: 5,
                y: staff_y,
                height: standard_height,
            });
            staff_y += standard_height + 10.0; // 缩小五线谱和六线谱之间的间距

            // 2. 六线谱/指法谱 (Tablature)
            let tab_height = s.tab_staff_height(string_count);
            staves.push(StaffLayout {
                staff_type: StaffType::Tablature,
                track_index: track_idx,
                string_count,
                y: staff_y,
                height: tab_height,
            });

            staff_y += tab_height + s.track_gap;
        }

        let total_height = staff_y - s.track_gap + 10.0; // 减去最后一个 gap

        SystemLayout {
            start_measure: start,
            end_measure: end,
            y,
            height: total_height.max(0.0),
            staves,
            measure_positions,
        }
    }

    /// 计算 beat 位置矩阵
    fn compute_beat_positions(
        &self,
        song: &Song,
        systems: &[SystemLayout],
    ) -> Vec<Vec<Vec<BeatPosition>>> {
        let measure_count = song.measure_count();
        let track_count = song.track_count();
        let mut result = Vec::with_capacity(measure_count);

        for m in 0..measure_count {
            // 查找此小节所在的 system 和宽度
            let measure_width = systems
                .iter()
                .flat_map(|sys| &sys.measure_positions)
                .find(|mp| mp.measure_index == m)
                .map(|mp| mp.width)
                .unwrap_or(self.settings.min_measure_width);

            let mut track_beats = Vec::with_capacity(track_count);
            for track in &song.tracks {
                if let Some(measure) = track.measures.get(m) {
                    let voice = measure.primary_voice();
                    track_beats.push(layout_voice_beats(voice, measure_width, &self.settings));
                } else {
                    track_beats.push(vec![]);
                }
            }
            result.push(track_beats);
        }

        result
    }
}
