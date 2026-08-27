//! 乐谱主绘制器。
//!
//! 将 `LayoutResult` 绘制到 egui `Painter` 上。
//! 采用「单轨道 + A4 分页」排版，绘制白色 A4 页面、谱表、音符与节奏符杆。

use std::collections::HashSet;

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use bassoxide_core::effects::{HammerOnPullOff, NoteEffect};
use bassoxide_core::measure::check_voice_duration;
use bassoxide_core::note::NoteType;
use bassoxide_core::song::Song;
use bassoxide_core::types::NoteValue;
use bassoxide_layout::engine::LayoutResult;
use bassoxide_layout::spacing::LayoutSettings;

use crate::colors::Theme;
use crate::cursor;
use crate::note_render;
use crate::rhythm_render::{self, RhythmBeat};
use crate::selection;
use crate::staff_render;

/// 编辑光标（由 UI 传入；None 表示不绘制选中态）
#[derive(Debug, Clone, Copy, Default)]
pub struct EditCursor {
    pub track: usize,
    pub measure: usize,
    pub beat: usize,
    pub string: u8,
}

/// 主绘制器：将布局结果渲染到画布上
pub struct ScorePainter<'a> {
    pub settings: &'a LayoutSettings,
    pub theme: &'a Theme,
    pub edit_cursor: Option<EditCursor>,
    /// (measure, beat, string) 多选高亮
    pub selected_notes: HashSet<(usize, usize, u8)>,
    /// 整小节高亮
    pub selected_measure: Option<usize>,
}

impl<'a> ScorePainter<'a> {
    pub fn new(settings: &'a LayoutSettings, theme: &'a Theme) -> Self {
        Self {
            settings,
            theme,
            edit_cursor: None,
            selected_notes: HashSet::new(),
            selected_measure: None,
        }
    }

    pub fn with_edit_cursor(mut self, cursor: EditCursor) -> Self {
        self.edit_cursor = Some(cursor);
        self
    }

    pub fn with_selection(
        mut self,
        notes: HashSet<(usize, usize, u8)>,
        measure: Option<usize>,
    ) -> Self {
        self.selected_notes = notes;
        self.selected_measure = measure;
        self
    }

    /// 绘制完整乐谱（按页裁剪，保证谱表墨迹不画到纸外）
    pub fn paint(
        &self,
        painter: &Painter,
        song: &Song,
        layout: &LayoutResult,
        offset: egui::Vec2,
    ) {
        self.draw_pages(painter, layout, offset);

        for system in &layout.systems {
            let page = layout
                .pages
                .get(system.page_index)
                .or_else(|| layout.pages.first());
            let clip = page.map(|p| {
                Rect::from_min_size(
                    Pos2::new(p.x + offset.x, p.y + offset.y),
                    Vec2::new(p.width, p.height),
                )
            });

            let paint_system = |painter: &Painter| {
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
                                painter,
                                staff_x,
                                staff_y,
                                system_width,
                                self.settings,
                                self.theme,
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
                        _ => {}
                    }

                    if let Some(first_mp) = system.measure_positions.first() {
                        if let Some(master_bar) = song.master_bar(first_mp.measure_index) {
                            let ts = &master_bar.time_signature;
                            let denom_num = note_value_to_num(ts.denominator);

                            if matches!(
                                staff.staff_type,
                                bassoxide_layout::staff::StaffType::Tablature
                                    | bassoxide_layout::staff::StaffType::Standard
                            ) {
                                staff_render::draw_time_signature(
                                    painter,
                                    staff_x + self.settings.clef_width + 14.0,
                                    staff_y,
                                    ts.numerator,
                                    denom_num,
                                    staff.height,
                                    self.settings,
                                    self.theme,
                                );
                            }
                        }
                    }

                    for measure_pos in &system.measure_positions {
                        let m = measure_pos.measure_index;
                        let measure_x = measure_pos.x + offset.x;

                        if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                            // 小节号（谱表上方，避开弦 1 光标）
                            let num_font =
                                egui::FontId::new(10.0, egui::FontFamily::Proportional);
                            painter.text(
                                Pos2::new(measure_x + 2.0, staff_y - 18.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("{}", m + 1),
                                num_font,
                                self.theme.clef_color,
                            );

                            // 整小节选区高亮
                            if self.selected_measure == Some(m) {
                                selection::draw_selection(
                                    painter,
                                    measure_x,
                                    staff_y - 16.0,
                                    measure_pos.width,
                                    staff.height + self.settings.rhythm_height + 16.0,
                                );
                            }

                            if let (Some(measure), Some(master)) =
                                (track.measures.get(m), song.master_bar(m))
                            {
                                let status = check_voice_duration(
                                    measure.primary_voice(),
                                    master.time_signature.measure_ticks(),
                                );
                                if !status.is_ok() {
                                    let rect = Rect::from_min_size(
                                        Pos2::new(measure_x, staff_y),
                                        Vec2::new(measure_pos.width, staff.height),
                                    );
                                    painter.rect_filled(
                                        rect,
                                        0.0,
                                        Color32::from_rgba_unmultiplied(220, 60, 60, 40),
                                    );
                                }
                            }
                        }

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
                                let beat_xs: Vec<(usize, f32)> = beat_positions
                                    .iter()
                                    .map(|bp| (bp.beat_index, measure_x + bp.x))
                                    .collect();

                                for bp in beat_positions {
                                    if let Some(beat) = voice.beats.get(bp.beat_index) {
                                        let beat_x = measure_x + bp.x;

                                        if !beat.is_empty() {
                                            for note in &beat.notes {
                                                let selected = self.selected_measure == Some(m)
                                                    || self.selected_notes.contains(&(
                                                        m,
                                                        bp.beat_index,
                                                        note.string,
                                                    ))
                                                    || self.edit_cursor.is_some_and(|c| {
                                                        c.track == track_idx
                                                            && c.measure == m
                                                            && c.beat == bp.beat_index
                                                            && c.string == note.string
                                                    });
                                                self.paint_note(
                                                    painter,
                                                    staff,
                                                    note,
                                                    beat_x,
                                                    staff_y,
                                                    track,
                                                    selected,
                                                    &beat_xs,
                                                    bp.beat_index,
                                                    voice,
                                                );
                                            }
                                        }
                                    }
                                }

                                if let Some(c) = self.edit_cursor {
                                    if c.track == track_idx
                                        && c.measure == m
                                        && staff.staff_type
                                            == bassoxide_layout::staff::StaffType::Tablature
                                    {
                                        if let Some(bp) =
                                            beat_positions.iter().find(|b| b.beat_index == c.beat)
                                        {
                                            let beat_x = measure_x + bp.x;
                                            let sy = staff_y
                                                + bassoxide_layout::tablature::string_y_offset(
                                                    c.string,
                                                    staff.string_count,
                                                    self.settings,
                                                );
                                            let sz = self.settings.tab_font_size + 4.0;
                                            cursor::draw_edit_cursor(
                                                painter,
                                                beat_x - sz * 0.5,
                                                sy - sz * 0.5,
                                                sz,
                                                sz,
                                            );
                                        }
                                    }
                                }

                                if staff.staff_type
                                    == bassoxide_layout::staff::StaffType::Tablature
                                {
                                    let rhythm_beats: Vec<RhythmBeat> = beat_positions
                                        .iter()
                                        .filter_map(|bp| {
                                            voice.beats.get(bp.beat_index).map(|beat| RhythmBeat {
                                                x: measure_x + bp.x,
                                                beat,
                                            })
                                        })
                                        .collect();
                                    let baseline_y = staff_y + staff.height + 2.0;
                                    rhythm_render::draw_measure_rhythm(
                                        painter,
                                        &rhythm_beats,
                                        baseline_y,
                                        measure_pos.width,
                                        self.settings,
                                        self.theme,
                                    );
                                }
                            }
                        }

                        if staff.staff_type == bassoxide_layout::staff::StaffType::Tablature {
                            if let Some(master_bar) = song.master_bar(m) {
                                let font =
                                    egui::FontId::new(11.0, egui::FontFamily::Proportional);
                                let mut ty = staff_y + 2.0;
                                if let Some(marker) = &master_bar.marker {
                                    painter.text(
                                        Pos2::new(measure_x + 4.0, ty),
                                        egui::Align2::LEFT_TOP,
                                        &marker.name,
                                        font.clone(),
                                        self.theme.marker_color,
                                    );
                                    ty += 13.0;
                                }
                                for dir in &master_bar.directions {
                                    let label = match dir {
                                        bassoxide_core::Direction::Coda => "Coda",
                                        bassoxide_core::Direction::DoubleCoda => "D.Coda",
                                        bassoxide_core::Direction::Segno => "Segno",
                                        bassoxide_core::Direction::SegnoSegno => "Segno×2",
                                        bassoxide_core::Direction::Fine => "Fine",
                                        bassoxide_core::Direction::DaCapo => "D.C.",
                                        bassoxide_core::Direction::DaCapoAlCoda => "D.C. al Coda",
                                        bassoxide_core::Direction::DaCapoAlDoubleCoda => {
                                            "D.C. al D.Coda"
                                        }
                                        bassoxide_core::Direction::DaCapoAlFine => "D.C. al Fine",
                                        bassoxide_core::Direction::DalSegno => "D.S.",
                                        bassoxide_core::Direction::DalSegnoAlCoda => "D.S. al Coda",
                                        bassoxide_core::Direction::DalSegnoAlDoubleCoda => {
                                            "D.S. al D.Coda"
                                        }
                                        bassoxide_core::Direction::DalSegnoAlFine => "D.S. al Fine",
                                        bassoxide_core::Direction::DalSegnoSegno => "D.S.S.",
                                        bassoxide_core::Direction::DalSegnoSegnoAlCoda => {
                                            "D.S.S. al Coda"
                                        }
                                        bassoxide_core::Direction::DalSegnoSegnoAlDoubleCoda => {
                                            "D.S.S. al D.Coda"
                                        }
                                        bassoxide_core::Direction::DalSegnoSegnoAlFine => {
                                            "D.S.S. al Fine"
                                        }
                                    };
                                    painter.text(
                                        Pos2::new(measure_x + 4.0, ty),
                                        egui::Align2::LEFT_TOP,
                                        label,
                                        font.clone(),
                                        self.theme.marker_color,
                                    );
                                    ty += 12.0;
                                }
                            }
                        }
                    }
                }
            };

            if let Some(clip_rect) = clip {
                let clipped = painter.with_clip_rect(clip_rect);
                paint_system(&clipped);
            } else {
                paint_system(painter);
            }
        }
    }

    fn paint_note(
        &self,
        painter: &Painter,
        staff: &bassoxide_layout::staff::StaffLayout,
        note: &bassoxide_core::note::Note,
        beat_x: f32,
        staff_y: f32,
        track: &bassoxide_core::track::Track,
        is_selected: bool,
        beat_xs: &[(usize, f32)],
        beat_index: usize,
        voice: &bassoxide_core::beat::Voice,
    ) {
        match staff.staff_type {
            bassoxide_layout::staff::StaffType::Standard => {
                note_render::draw_standard_note(
                    painter, note, beat_x, staff_y, &track.tuning, self.settings, self.theme,
                );
            }
            bassoxide_layout::staff::StaffType::Tablature => {
                note_render::draw_tab_note(
                    painter,
                    note,
                    beat_x,
                    staff_y,
                    staff.string_count,
                    self.settings,
                    self.theme,
                    is_selected,
                );

                let string_y = staff_y
                    + bassoxide_layout::tablature::string_y_offset(
                        note.string,
                        staff.string_count,
                        self.settings,
                    );

                if note.note_type == NoteType::Tie {
                    if let Some(prev_x) = prev_same_string_x(voice, beat_xs, beat_index, note.string)
                    {
                        crate::effect_render::draw_tie_arc(
                            painter, prev_x, string_y, beat_x, string_y, self.theme,
                        );
                    }
                }

                for effect in &note.effects {
                    match effect {
                        NoteEffect::Bend(bend) => {
                            crate::effect_render::draw_bend(
                                painter, bend, beat_x, string_y, self.theme,
                            );
                        }
                        NoteEffect::Harmonic(harm) => {
                            crate::effect_render::draw_harmonic(
                                painter, harm, beat_x, string_y, self.theme,
                            );
                        }
                        NoteEffect::Vibrato(_, _) => {
                            crate::effect_render::draw_vibrato(
                                painter,
                                beat_x,
                                staff_y + self.settings.note_pad() * 0.2,
                                20.0,
                                self.theme,
                            );
                        }
                        NoteEffect::Slide(s) => {
                            if let Some(st) = s.first() {
                                let (x2, y2) = next_same_string_pos(
                                    voice,
                                    beat_xs,
                                    beat_index,
                                    note.string,
                                    staff_y,
                                    staff.string_count,
                                    self.settings,
                                )
                                .unwrap_or((beat_x + 28.0, string_y));
                                crate::effect_render::draw_slide(
                                    painter, st, beat_x, string_y, x2, y2, self.theme,
                                );
                            }
                        }
                        NoteEffect::HammerOnPullOff(hopo) => {
                            let (x2, y2) = next_same_string_pos(
                                voice,
                                beat_xs,
                                beat_index,
                                note.string,
                                staff_y,
                                staff.string_count,
                                self.settings,
                            )
                            .unwrap_or((beat_x + 28.0, string_y));
                            let label = match hopo {
                                HammerOnPullOff::HammerOn => "H",
                                HammerOnPullOff::PullOff => "P",
                            };
                            crate::effect_render::draw_hopo_arc(
                                painter, beat_x, string_y, x2, y2, label, self.theme,
                            );
                        }
                        NoteEffect::LetRing => {
                            crate::effect_render::draw_text_line(
                                painter,
                                "Let Ring",
                                beat_x,
                                staff_y + staff.height + 4.0,
                                30.0,
                                self.theme,
                            );
                        }
                        NoteEffect::PalmMute => {
                            crate::effect_render::draw_text_line(
                                painter,
                                "P.M.",
                                beat_x,
                                staff_y + staff.height + 4.0,
                                30.0,
                                self.theme,
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn draw_pages(&self, painter: &Painter, layout: &LayoutResult, offset: egui::Vec2) {
        let paper = Color32::WHITE;
        let border = Color32::from_rgb(0xC0, 0xC9, 0xC1);
        let shadow = Color32::from_black_alpha(28);

        for page in &layout.pages {
            let min = Pos2::new(page.x + offset.x, page.y + offset.y);
            let rect = Rect::from_min_size(min, Vec2::new(page.width, page.height));

            let shadow_rect = rect.translate(Vec2::new(3.0, 4.0));
            painter.rect_filled(shadow_rect, 2.0, shadow);
            painter.rect_filled(rect, 2.0, paper);
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0_f32, border),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn prev_same_string_x(
    voice: &bassoxide_core::beat::Voice,
    beat_xs: &[(usize, f32)],
    beat_index: usize,
    string: u8,
) -> Option<f32> {
    for i in (0..beat_index).rev() {
        if let Some(beat) = voice.beats.get(i) {
            if beat.note_on_string(string).is_some() {
                return beat_xs.iter().find(|(idx, _)| *idx == i).map(|(_, x)| *x);
            }
        }
    }
    None
}

fn next_same_string_pos(
    voice: &bassoxide_core::beat::Voice,
    beat_xs: &[(usize, f32)],
    beat_index: usize,
    string: u8,
    staff_y: f32,
    string_count: usize,
    settings: &LayoutSettings,
) -> Option<(f32, f32)> {
    for i in (beat_index + 1)..voice.beats.len() {
        if let Some(beat) = voice.beats.get(i) {
            if beat.note_on_string(string).is_some() {
                let x = beat_xs.iter().find(|(idx, _)| *idx == i).map(|(_, x)| *x)?;
                let y = staff_y
                    + bassoxide_layout::tablature::string_y_offset(string, string_count, settings);
                return Some((x, y));
            }
        }
    }
    None
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
