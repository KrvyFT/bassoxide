//! 乐谱主绘制器。
//!
//! 将 `LayoutResult` 绘制到 egui `Painter` 上。

use egui::Painter;
use bassoxide_core::song::Song;
use bassoxide_core::types::NoteValue;
use bassoxide_layout::engine::LayoutResult;
use bassoxide_layout::spacing::LayoutSettings;

use crate::colors::Theme;
use crate::note_render;
use crate::staff_render;

/// 主绘制器：将布局结果渲染到画布上
pub struct ScorePainter<'a> {
    pub settings: &'a LayoutSettings,
    pub theme: &'a Theme,
}

impl<'a> ScorePainter<'a> {
    pub fn new(settings: &'a LayoutSettings, theme: &'a Theme) -> Self {
        Self { settings, theme }
    }

    /// 绘制完整乐谱
    pub fn paint(
        &self,
        painter: &Painter,
        song: &Song,
        layout: &LayoutResult,
        _scroll_y: f32,
    ) {
        for system in &layout.systems {
            // 绘制每个轨道的谱表
            for (track_idx, staff) in system.staves.iter().enumerate() {
                let staff_y = system.y + staff.y;

                let track = match song.tracks.get(track_idx) {
                    Some(t) => t,
                    None => continue,
                };

                // 绘制弦线
                let system_width = system
                    .measure_positions
                    .last()
                    .map(|mp| mp.x + mp.width - self.settings.margin_left)
                    .unwrap_or(self.settings.available_width);

                staff_render::draw_tab_staff(
                    painter,
                    self.settings.margin_left,
                    staff_y,
                    system_width,
                    staff.string_count,
                    self.settings,
                    self.theme,
                );

                // 绘制 TAB 谱号（每行开头）
                staff_render::draw_tab_clef(
                    painter,
                    self.settings.margin_left,
                    staff_y,
                    staff.string_count,
                    self.settings,
                    self.theme,
                );

                // 绘制拍号（第一行开头或拍号变化时）
                if let Some(first_mp) = system.measure_positions.first() {
                    if let Some(master_bar) = song.master_bar(first_mp.measure_index) {
                        let ts = &master_bar.time_signature;
                        let denom_num = match ts.denominator {
                            NoteValue::Whole => 1,
                            NoteValue::Half => 2,
                            NoteValue::Quarter => 4,
                            NoteValue::Eighth => 8,
                            NoteValue::Sixteenth => 16,
                            NoteValue::ThirtySecond => 32,
                            NoteValue::SixtyFourth => 64,
                        };
                        staff_render::draw_time_signature(
                            painter,
                            self.settings.margin_left + self.settings.clef_width + 12.0,
                            staff_y,
                            ts.numerator,
                            denom_num,
                            staff.string_count,
                            self.settings,
                            self.theme,
                        );
                    }
                }

                // 绘制每个小节的内容
                for measure_pos in &system.measure_positions {
                    let m = measure_pos.measure_index;

                    // 绘制小节线
                    staff_render::draw_bar_line(
                        painter,
                        measure_pos.x + measure_pos.width,
                        staff_y,
                        staff.height,
                        self.theme,
                    );

                    // 绘制音符
                    if let Some(measure) = track.measures.get(m) {
                        if let Some(beat_positions) = layout
                            .beat_positions
                            .get(m)
                            .and_then(|tracks| tracks.get(track_idx))
                        {
                            let voice = measure.primary_voice();
                            for bp in beat_positions {
                                if let Some(beat) = voice.beats.get(bp.beat_index) {
                                    let beat_x = measure_pos.x + bp.x;

                                    if beat.is_empty() {
                                        // 休止符
                                        note_render::draw_rest(
                                            painter,
                                            beat_x,
                                            staff_y,
                                            staff.height,
                                            self.theme,
                                        );
                                    } else {
                                        // 各音符
                                        for note in &beat.notes {
                                            note_render::draw_tab_note(
                                                painter,
                                                note,
                                                beat_x,
                                                staff_y,
                                                self.settings,
                                                self.theme,
                                                false,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 排练标记
                    if let Some(master_bar) = song.master_bar(m) {
                        if let Some(marker) = &master_bar.marker {
                            let font = egui::FontId::new(11.0, egui::FontFamily::Proportional);
                            painter.text(
                                egui::Pos2::new(measure_pos.x + 4.0, staff_y - 14.0),
                                egui::Align2::LEFT_BOTTOM,
                                &marker.name,
                                font,
                                self.theme.marker_color,
                            );
                        }
                    }
                }
            }
        }
    }
}
