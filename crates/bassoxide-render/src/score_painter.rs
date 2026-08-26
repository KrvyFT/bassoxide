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
        offset: egui::Vec2,
    ) {
        let margin_left = self.settings.margin_left + offset.x;

        for system in &layout.systems {
            // 绘制每个轨道的谱表
            for staff in &system.staves {
                let track_idx = staff.track_index;
                let staff_y = system.y + staff.y + offset.y;

                let track = match song.tracks.get(track_idx) {
                    Some(t) => t,
                    None => continue,
                };

                let system_width = system
                    .measure_positions
                    .last()
                    .map(|mp| mp.x + mp.width - self.settings.margin_left)
                    .unwrap_or(self.settings.available_width);

                match staff.staff_type {
                    bassoxide_layout::staff::StaffType::Standard => {
                        staff_render::draw_standard_staff(
                            painter,
                            margin_left,
                            staff_y,
                            system_width,
                            self.settings,
                            self.theme,
                        );
                        // TODO: 绘制高音/低音谱号、调号等
                    }
                    bassoxide_layout::staff::StaffType::Tablature => {
                        staff_render::draw_tab_staff(
                            painter,
                            margin_left,
                            staff_y,
                            system_width,
                            staff.string_count,
                            self.settings,
                            self.theme,
                        );
                        staff_render::draw_tab_clef(
                            painter,
                            margin_left,
                            staff_y,
                            staff.string_count,
                            self.settings,
                            self.theme,
                        );
                    }
                    bassoxide_layout::staff::StaffType::Numbered => {
                        staff_render::draw_numbered_staff(
                            painter,
                            margin_left,
                            staff_y,
                            system_width,
                            self.settings,
                            self.theme,
                        );
                    }
                    _ => {} // 其他谱表
                }

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
                        
                        if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
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
                }

                // 绘制每个小节的内容
                for measure_pos in &system.measure_positions {
                    let m = measure_pos.measure_index;
                    let measure_x = measure_pos.x + offset.x;

                    // 绘制小节线
                    staff_render::draw_bar_line(
                        painter,
                        measure_x + measure_pos.width,
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
                                    let beat_x = measure_x + bp.x;

                                    if beat.is_empty() {
                                        // 休止符
                                        if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                                            note_render::draw_rest(
                                                painter,
                                                beat_x,
                                                staff_y,
                                                staff.height,
                                                self.theme,
                                            );
                                        }
                                    } else {
                                        // 各音符
                                        for note in &beat.notes {
                                            match staff.staff_type {
                                                bassoxide_layout::staff::StaffType::Standard => {
                                                    note_render::draw_standard_note(
                                                        painter,
                                                        note,
                                                        beat_x,
                                                        staff_y,
                                                        &track.tuning,
                                                        self.theme,
                                                    );
                                                }
                                                bassoxide_layout::staff::StaffType::Tablature => {
                                                    note_render::draw_tab_note(
                                                        painter,
                                                        note,
                                                        beat_x,
                                                        staff_y,
                                                        self.settings,
                                                        self.theme,
                                                        false,
                                                    );
                                                    
                                                    // 绘制特效
                                                    let string_y = staff_y + bassoxide_layout::tablature::string_y_offset(note.string, self.settings);
                                                    for effect in &note.effects {
                                                        match effect {
                                                            bassoxide_core::effects::NoteEffect::Bend(bend) => {
                                                                crate::effect_render::draw_bend(painter, bend, beat_x, string_y, self.theme);
                                                            }
                                                            bassoxide_core::effects::NoteEffect::Harmonic(harm) => {
                                                                crate::effect_render::draw_harmonic(painter, harm, beat_x, string_y, self.theme);
                                                            }
                                                            bassoxide_core::effects::NoteEffect::Vibrato(_, _) => {
                                                                crate::effect_render::draw_vibrato(painter, beat_x, staff_y - 10.0, 20.0, self.theme);
                                                            }
                                                            bassoxide_core::effects::NoteEffect::Slide(s) => {
                                                                // 简单向右画一条滑音线
                                                                if !s.is_empty() {
                                                                    crate::effect_render::draw_slide(painter, &s[0], beat_x, string_y, beat_x + 30.0, string_y, self.theme);
                                                                }
                                                            }
                                                            bassoxide_core::effects::NoteEffect::LetRing => {
                                                                crate::effect_render::draw_text_line(painter, "Let Ring", beat_x, staff_y + staff.height + 10.0, 30.0, self.theme);
                                                            }
                                                            bassoxide_core::effects::NoteEffect::PalmMute => {
                                                                crate::effect_render::draw_text_line(painter, "P.M.", beat_x, staff_y + staff.height + 10.0, 30.0, self.theme);
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                bassoxide_layout::staff::StaffType::Numbered => {
                                                    note_render::draw_numbered_note(
                                                        painter,
                                                        note,
                                                        beat_x,
                                                        staff_y,
                                                        &track.tuning,
                                                        self.theme,
                                                        false,
                                                    );
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 排练标记 (仅在每个 System 的最顶端谱表绘制)
                    if staff.staff_type == bassoxide_layout::staff::StaffType::Standard {
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
}
