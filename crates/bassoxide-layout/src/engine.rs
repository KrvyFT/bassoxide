//! 排版引擎主入口。
//!
//! 将 `Song` 数据转换为 `LayoutResult`，包含所有元素的屏幕坐标。
//! 当前采用「单轨道 + A4 分页」的排版方式：只显示选中的轨道，
//! 小节按 A4 页面可用宽度自动换行，多行按 A4 页面高度自动分页。

use bassoxide_core::song::Song;

use crate::measure_layout::{layout_voice_beats, BeatPosition};
use crate::page::PageLayout;
use crate::spacing::LayoutSettings;
use crate::staff::{StaffLayout, StaffType};
use crate::system::{MeasurePosition, SystemLayout};

/// 排版引擎
pub struct LayoutEngine {
    pub settings: LayoutSettings,
    /// 当前显示的轨道索引
    pub selected_track: usize,
}

/// 完整的布局结果
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// 所有 System 行
    pub systems: Vec<SystemLayout>,
    /// A4 页面矩形
    pub pages: Vec<PageLayout>,
    /// 单页宽度
    pub page_width: f32,
    /// 单页高度
    pub page_height: f32,
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
        Self {
            settings,
            selected_track: 0,
        }
    }

    /// 指定显示轨道
    pub fn with_selected_track(mut self, track: usize) -> Self {
        self.selected_track = track;
        self
    }

    /// 执行完整排版
    pub fn layout(&self, song: &Song) -> LayoutResult {
        let empty = LayoutResult {
            systems: vec![],
            pages: vec![],
            page_width: self.settings.page_width,
            page_height: self.settings.page_height,
            total_height: 0.0,
            total_width: self.settings.page_width,
            beat_positions: vec![],
        };

        if song.tracks.is_empty() || song.master_bars.is_empty() {
            return empty;
        }

        let selected = self.selected_track.min(song.tracks.len() - 1);

        // 1. 计算每个小节的理想宽度（仅基于所选轨道）
        let measure_widths = self.compute_measure_widths(song, selected);

        // 2. 将小节分配到 System 行（按 A4 页宽自动换行）
        let system_ranges = self.break_into_systems(&measure_widths);

        // 3. 计算所选轨道的谱表堆叠（每行相同）
        let (staff_template, system_height) = self.build_track_staves(song, selected);

        // 4. 分页 + 逐行详细布局
        let page_gap = 30.0;
        let page_left = 24.0;
        // 行间距直接使用 system_gap（默认 80）
        let line_gap = self.settings.system_gap.max(8.0);
        let content_top_pad = self.settings.margin_top.min(self.settings.page_margin);

        let mut pages: Vec<PageLayout> = Vec::new();
        let mut systems: Vec<SystemLayout> = Vec::new();

        // 建立首个页面
        let mut page_index = 0usize;
        let mut page_top = page_gap;
        pages.push(PageLayout {
            index: page_index,
            x: page_left,
            y: page_top,
            width: self.settings.page_width,
            height: self.settings.page_height,
        });
        let mut y = page_top + self.settings.page_margin + content_top_pad;

        for (start, end) in &system_ranges {
            let page_content_bottom = page_top + self.settings.page_height - self.settings.page_margin;

            // 当前行放不下 -> 新建页面
            if y + system_height > page_content_bottom && systems.iter().any(|s| s.page_index == page_index) {
                page_index += 1;
                page_top = page_gap + page_index as f32 * (self.settings.page_height + page_gap);
                pages.push(PageLayout {
                    index: page_index,
                    x: page_left,
                    y: page_top,
                    width: self.settings.page_width,
                    height: self.settings.page_height,
                });
                y = page_top + self.settings.page_margin + content_top_pad;
            }

            let content_left = page_left + self.settings.page_margin;
            let system = self.layout_system(
                *start,
                *end,
                y,
                content_left,
                page_index,
                &measure_widths,
                &staff_template,
                system_height,
            );
            y += system.height + line_gap;
            systems.push(system);
        }

        // 5. 计算 beat 位置
        let beat_positions = self.compute_beat_positions(song, &systems);

        let last_page_bottom = pages
            .last()
            .map(|p| p.y + p.height)
            .unwrap_or(self.settings.page_height);
        let total_width = page_left * 2.0 + self.settings.page_width;

        LayoutResult {
            systems,
            pages,
            page_width: self.settings.page_width,
            page_height: self.settings.page_height,
            total_height: last_page_bottom + page_gap,
            total_width,
            beat_positions,
        }
    }

    /// 计算每个小节的理想宽度（基于所选轨道的内容复杂度）
    fn compute_measure_widths(&self, song: &Song, selected: usize) -> Vec<f32> {
        let measure_count = song.measure_count();
        let mut widths = Vec::with_capacity(measure_count);

        let track = song.tracks.get(selected);
        // 单行内小节最大宽度不超过页面可用宽度（减去前导区）
        let max_measure_width =
            (self.settings.page_content_width() - self.settings.clef_width - self.settings.time_sig_width)
                .max(self.settings.min_measure_width);

        for m in 0..measure_count {
            let max_beats = track
                .and_then(|t| t.measures.get(m))
                .map(|measure| {
                    measure
                        .voices
                        .iter()
                        .map(|v| v.beats.len())
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0)
                .max(1);

            let width = (max_beats as f32 * self.settings.min_beat_spacing + 20.0)
                .max(self.settings.min_measure_width)
                .min(max_measure_width);
            widths.push(width);
        }

        widths
    }

    /// 将小节分配到行：按页宽自动换行，或按固定每行小节数
    fn break_into_systems(&self, widths: &[f32]) -> Vec<(usize, usize)> {
        if widths.is_empty() {
            return vec![];
        }

        let fixed = self.settings.measures_per_line as usize;
        if fixed > 0 {
            let mut ranges = Vec::new();
            let mut start = 0usize;
            while start < widths.len() {
                let end = (start + fixed).min(widths.len());
                ranges.push((start, end));
                start = end;
            }
            return ranges;
        }

        let preamble = self.settings.clef_width + self.settings.time_sig_width;
        let available =
            (self.settings.page_content_width() - preamble).max(self.settings.min_measure_width);

        let mut ranges = Vec::new();
        let mut start = 0usize;
        let mut acc = 0.0f32;

        for (i, w) in widths.iter().enumerate() {
            // 若当前行已有内容且再加入会超宽，则换行
            if i > start && acc + w > available {
                ranges.push((start, i));
                start = i;
                acc = 0.0;
            }
            acc += w;
        }
        if start < widths.len() {
            ranges.push((start, widths.len()));
        }

        ranges
    }

    /// 构建所选轨道的谱表堆叠模板（各行结构相同）
    fn build_track_staves(&self, song: &Song, selected: usize) -> (Vec<StaffLayout>, f32) {
        let s = &self.settings;
        let mut staves = Vec::new();
        let mut y = 0.0f32;
        let mut total = 0.0f32;

        let track = match song.tracks.get(selected) {
            Some(t) => t,
            None => return (staves, 0.0),
        };

        let display = &track.staff_display;
        let mut any = false;

        if display.show_standard {
            let band = s.standard_band_height();
            staves.push(StaffLayout {
                staff_type: StaffType::Standard,
                track_index: selected,
                string_count: 5,
                y,
                height: band,
            });
            total = y + band;
            y = total + s.track_gap;
            any = true;
        }

        if display.show_tab {
            let string_count = display.tab_strings.max(1) as usize;
            let band = s.tab_band_height(string_count);
            staves.push(StaffLayout {
                staff_type: StaffType::Tablature,
                track_index: selected,
                string_count,
                y,
                height: band,
            });
            // Tab 下方预留符杆(节奏)区域 —— 计入 system 高度，保证谱表∈纸张
            total = y + band + s.rhythm_height + 8.0;
            any = true;
        }

        // 兜底：至少显示五线谱，避免空白
        if !any {
            let band = s.standard_band_height();
            staves.push(StaffLayout {
                staff_type: StaffType::Standard,
                track_index: selected,
                string_count: 5,
                y: 0.0,
                height: band,
            });
            total = band;
        }

        (staves, total.max(24.0))
    }

    /// 布局单行 System
    #[allow(clippy::too_many_arguments)]
    fn layout_system(
        &self,
        start: usize,
        end: usize,
        y: f32,
        content_left: f32,
        page_index: usize,
        measure_widths: &[f32],
        staff_template: &[StaffLayout],
        system_height: f32,
    ) -> SystemLayout {
        let s = &self.settings;
        let preamble_width = s.clef_width + s.time_sig_width;
        let count = end.saturating_sub(start).max(1);
        // 自动缩放：本行所有小节总宽铺满页面内容区
        let available = (s.page_content_width() - preamble_width).max(s.min_measure_width);
        let natural_sum: f32 = (start..end).map(|m| measure_widths[m]).sum::<f32>().max(1.0);

        let mut measure_positions = Vec::new();
        let mut x = content_left + preamble_width;
        for m in start..end {
            let width = if s.measures_per_line > 0 {
                // 固定每行 N 小节：等分页宽，观感整齐
                available / count as f32
            } else {
                // 自动换行：按内容比例拉伸铺满
                measure_widths[m] / natural_sum * available
            };
            measure_positions.push(MeasurePosition {
                measure_index: m,
                x,
                width,
            });
            x += width;
        }

        let content_width = s.page_content_width().max(preamble_width);

        SystemLayout {
            start_measure: start,
            end_measure: end,
            y,
            height: system_height,
            content_left,
            content_width,
            page_index,
            staves: staff_template.to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bassoxide_core::beat::{Beat, Voice};
    use bassoxide_core::measure::{MasterBar, Measure};
    use bassoxide_core::note::Note;
    use bassoxide_core::song::{Song, SongInfo};
    use bassoxide_core::track::{StaffDisplay, Track, Tuning};
    use bassoxide_core::types::Duration;
    use crate::PaperSize;

    fn sample_song(measures: usize) -> Song {
        let mut song = Song {
            info: SongInfo {
                title: "layout-test".into(),
                ..Default::default()
            },
            tempo: 120,
            ..Default::default()
        };
        for _ in 0..measures {
            song.master_bars.push(MasterBar::default());
        }
        let mut track = Track {
            name: "Gtr".into(),
            tuning: Tuning::standard_guitar(),
            staff_display: StaffDisplay {
                show_standard: false,
                show_tab: true,
                tab_strings: 6,
            },
            ..Default::default()
        };
        for _ in 0..measures {
            let mut voices = [
                Voice::default(),
                Voice::default(),
                Voice::default(),
                Voice::default(),
            ];
            voices[0].beats = vec![
                Beat {
                    duration: Duration::default(),
                    notes: vec![Note {
                        string: 1,
                        fret: 0,
                        midi_note: 40,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                Beat {
                    duration: Duration::default(),
                    notes: vec![Note {
                        string: 1,
                        fret: 2,
                        midi_note: 42,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ];
            track.measures.push(Measure {
                voices,
                ..Default::default()
            });
        }
        song.tracks.push(track);
        song
    }

    #[test]
    fn four_measures_fill_page_width() {
        let settings = LayoutSettings::default();
        assert_eq!(settings.measures_per_line, 4);
        assert!((settings.tab_font_size - 13.0).abs() < f32::EPSILON);
        assert!((settings.tab_string_spacing - 10.0).abs() < f32::EPSILON);
        assert!((settings.system_gap - 10.0).abs() < f32::EPSILON);

        let song = sample_song(8);
        let engine = LayoutEngine::new(settings.clone()).with_selected_track(0);
        let layout = engine.layout(&song);
        assert!(!layout.systems.is_empty());

        let preamble = settings.clef_width + settings.time_sig_width;
        let available = settings.page_content_width() - preamble;
        for system in &layout.systems {
            let sum: f32 = system.measure_positions.iter().map(|m| m.width).sum();
            assert!(
                (sum - available).abs() < 1.0,
                "sum={sum} available={available}"
            );
            assert!(system.measure_positions.len() <= 4);
            // 固定 4 小节时等宽
            if system.measure_positions.len() == 4 {
                for mp in &system.measure_positions {
                    assert!((mp.width - available / 4.0).abs() < 1.0);
                }
            }
        }
        // 行间距使用完整 system_gap，同页多行时应能自动分页
        assert!(layout.pages.len() >= 1);
        assert_eq!(layout.systems.len(), 2, "8 measures / 4 per line => 2 systems");
    }

    #[test]
    fn paper_size_scales_page_and_measure_width() {
        let mut a4 = LayoutSettings::default();
        a4.paper_size = PaperSize::A4;
        a4.content_scale = PaperSize::A4.content_scale();
        let (w4, h4) = PaperSize::A4.size_px();
        a4.page_width = w4;
        a4.page_height = h4;

        let mut a5 = a4.clone();
        a5.paper_size = PaperSize::A5;
        a5.content_scale = PaperSize::A5.content_scale();
        let (w5, h5) = PaperSize::A5.size_px();
        a5.page_width = w5;
        a5.page_height = h5;
        a5.min_measure_width *= a5.content_scale;
        a5.min_beat_spacing *= a5.content_scale;

        let song = sample_song(4);
        let layout_a4 = LayoutEngine::new(a4.clone())
            .with_selected_track(0)
            .layout(&song);
        let layout_a5 = LayoutEngine::new(a5.clone())
            .with_selected_track(0)
            .layout(&song);

        assert!((layout_a4.page_width - w4).abs() < 1.0);
        assert!((layout_a5.page_width - w5).abs() < 1.0);
        assert!(layout_a5.page_width < layout_a4.page_width);

        let sum_a4: f32 = layout_a4.systems[0]
            .measure_positions
            .iter()
            .map(|m| m.width)
            .sum();
        let sum_a5: f32 = layout_a5.systems[0]
            .measure_positions
            .iter()
            .map(|m| m.width)
            .sum();
        assert!(sum_a5 < sum_a4, "a5={sum_a5} a4={sum_a4}");
    }

    #[test]
    fn row_spacing_affects_page_capacity() {
        let mut tight = LayoutSettings::default();
        tight.system_gap = 40.0;
        let mut loose = LayoutSettings::default();
        loose.system_gap = 200.0;

        let song = sample_song(24);
        let layout_tight = LayoutEngine::new(tight).with_selected_track(0).layout(&song);
        let layout_loose = LayoutEngine::new(loose).with_selected_track(0).layout(&song);

        assert!(
            layout_loose.pages.len() >= layout_tight.pages.len(),
            "loose pages={} tight pages={}",
            layout_loose.pages.len(),
            layout_tight.pages.len()
        );
        assert!(layout_loose.pages.len() > 1 || layout_tight.pages.len() == 1);
    }
}
