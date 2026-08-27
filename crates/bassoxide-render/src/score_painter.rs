//! 乐谱主绘制器。
//!
//! 将 `LayoutResult` 绘制到 egui `Painter` 上。
//! 采用「单轨道 + A4 分页」排版，绘制白色 A4 页面、谱表、音符与节奏符杆。

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use bassoxide_core::song::Song;
use bassoxide_core::types::NoteValue;
use bassoxide_layout::engine::LayoutResult;
use bassoxide_layout::spacing::LayoutSettings;

use crate::colors::Theme;
use crate::note_render;
use crate::rhythm_render::{self, RhythmBeat};
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
        // 0. 先绘制 A4 白色页面
        self.draw_pages(painter, layout, offset);

        for system in &layout.systems {
            let staff_x = system.content_left + offset.x;
            let system_width = system.content_width;

            for staff in &system.staves {
                let track_idx = staff.track_index;
                let staff_y = system.y + staff.y + offset.y;

                let track = match song.tracks.get(track_idx) {
                    Some(t) => t,
                    None => continue,
                };

                match staff.staff_type {
                    bassoxide_layout::staff::StaffType::Standard => {
                        staff_render::draw_standard_staff(
                            painter, staff_x, staff_y, system_width, self.settings, self.theme,
                        );
                    }
                    bassoxide_layout::staff::StaffType::Tablature => {
                        staff_render::draw_tab_staff(
                            painter,
                            staff_x,
                            staff_y,
                            system_width,
                            staff.string_count,
                            self.settings,
                            self.theme,
                        );
                        staff_render::draw_tab_clef(
                            painter,
                            staff_x,
                            staff_y,
                            staff.string_count,
                            self.settings,
                            self.theme,
                        );
                    }
                    bassoxide_layout::staff::StaffType::Numbered => {
                        staff_render::draw_numbered_staff(
                            painter, staff_x, staff_y, system_width, self.settings, self.theme,
                        );
                    }
                    _ => {}
                }

                // 绘制该行开头的拍号
                if let Some(first_mp) = system.measure_positions.first() {
                    if let Some(master_bar) = song.master_bar(first_mp.measure_index) {
                        let ts = &master_bar.time_signature;
                        let denom_num = note_value_to_num(ts.denominator);

                        if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                            staff_render::draw_time_signature(
                                painter,
                                staff_x + self.settings.clef_width + 12.0,
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

                    staff_render::draw_bar_line(
                        painter,
                        measure_x + measure_pos.width,
                        staff_y,
                        staff.height,
                        self.theme,
                    );

                    if let Some(measure) = track.measures.get(m) {
                        let voice = measure.primary_voice();
                        if let Some(beat_positions) = layout
                            .beat_positions
                            .get(m)
                            .and_then(|tracks| tracks.get(track_idx))
                        {
                            for bp in beat_positions {
                                if let Some(beat) = voice.beats.get(bp.beat_index) {
                                    let beat_x = measure_x + bp.x;

                                    if beat.is_empty() {
                                        if staff.staff_type
                                            == bassoxide_layout::staff::StaffType::Tablature
                                        {
                                            note_render::draw_rest(
                                                painter,
                                                beat_x,
                                                staff_y,
                                                staff.height,
                                                self.theme,
                                            );
                                        }
                                    } else {
                                        for note in &beat.notes {
                                            self.paint_note(painter, staff, note, beat_x, staff_y, track);
                                        }
                                    }
                                }
                            }

                            // 六线谱：在小节下方绘制节奏符杆
                            if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                                let rhythm_beats: Vec<RhythmBeat> = beat_positions
                                    .iter()
                                    .filter_map(|bp| {
                                        voice.beats.get(bp.beat_index).map(|beat| RhythmBeat {
                                            x: measure_x + bp.x,
                                            beat,
                                        })
                                    })
                                    .collect();
                                let baseline_y = staff_y + staff.height + 6.0;
                                rhythm_render::draw_measure_rhythm(
                                    painter,
                                    &rhythm_beats,
                                    baseline_y,
                                    self.settings,
                                    self.theme,
                                );
                            }
                        }
                    }

                    // 排练标记
                    if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                        if let Some(master_bar) = song.master_bar(m) {
                            if let Some(marker) = &master_bar.marker {
                                let font = egui::FontId::new(11.0, egui::FontFamily::Proportional);
                                painter.text(
                                    Pos2::new(measure_x + 4.0, staff_y - 14.0),
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

    /// 绘制单个音符（含特效）
    fn paint_note(
        &self,
        painter: &Painter,
        staff: &bassoxide_layout::staff::StaffLayout,
        note: &bassoxide_core::note::Note,
        beat_x: f32,
        staff_y: f32,
        track: &bassoxide_core::track::Track,
    ) {
        match staff.staff_type {
            bassoxide_layout::staff::StaffType::Standard => {
                note_render::draw_standard_note(
                    painter, note, beat_x, staff_y, &track.tuning, self.theme,
                );
            }
            bassoxide_layout::staff::StaffType::Tablature => {
                note_render::draw_tab_note(
                    painter, note, beat_x, staff_y, self.settings, self.theme, false,
                );

                let string_y = staff_y
                    + bassoxide_layout::tablature::string_y_offset(note.string, self.settings);
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
                    painter, note, beat_x, staff_y, &track.tuning, self.theme, false,
                );
            }
            _ => {}
        }
    }

    /// 绘制 A4 白色页面（含淡阴影）
    fn draw_pages(&self, painter: &Painter, layout: &LayoutResult, offset: egui::Vec2) {
        let paper = Color32::from_gray(252);
        let border = Color32::from_gray(170);
        let shadow = Color32::from_black_alpha(40);

        for page in &layout.pages {
            let min = Pos2::new(page.x + offset.x, page.y + offset.y);
            let rect = Rect::from_min_size(min, Vec2::new(page.width, page.height));

            // 阴影
            let shadow_rect = rect.translate(Vec2::new(4.0, 5.0));
            painter.rect_filled(shadow_rect, 2.0, shadow);
            // 纸张
            painter.rect_filled(rect, 2.0, paper);
            painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, border), egui::StrokeKind::Inside);
        }
    }
}

fn note_value_to_num(v: NoteValue) -> u8 {
    match v {
        NoteValue::Whole => 1,
        NoteValue::Half => 2,
        NoteValue::Quarter => 4,
        NoteValue::Eighth => 8,
        NoteValue::Sixteenth => 16,
        NoteValue::ThirtySecond => 32,
        NoteValue::SixtyFourth => 64,
    }
}
