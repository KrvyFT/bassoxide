//! 音符数字渲染（六线谱模式）。

use egui::{Painter, Pos2, Rect, Vec2};

use bassoxide_core::note::Note;
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_layout::tablature;

use crate::colors::Theme;

/// 在六线谱上绘制单个音符（品格数字）。
/// 音符 Y 夹在谱表带内（含 note_pad）。
pub fn draw_tab_note(
    painter: &Painter,
    note: &Note,
    x: f32,
    staff_y: f32,
    string_count: usize,
    settings: &LayoutSettings,
    theme: &Theme,
    is_selected: bool,
) {
    let string_y =
        staff_y + tablature::string_y_offset(note.string, string_count, settings);
    let text = tablature::tab_note_text(note);

    let font = egui::FontId::new(settings.tab_font_size, egui::FontFamily::Proportional);
    let color = if is_selected {
        theme.selected_note
    } else {
        theme.note_text
    };

    // 先画一个小背景矩形清除弦线（让数字清晰可读）
    let char_w = if text.len() > 2 {
        settings.tab_font_size * 1.15
    } else if text.len() > 1 {
        settings.tab_font_size * 0.95
    } else {
        settings.tab_font_size * 0.7
    };
    let text_size = Vec2::new(char_w, settings.tab_font_size);
    let bg_rect = Rect::from_center_size(
        Pos2::new(x, string_y),
        text_size + Vec2::new(3.0, 1.5),
    );
    painter.rect_filled(bg_rect, 0.0, theme.background);

    painter.text(
        Pos2::new(x, string_y),
        egui::Align2::CENTER_CENTER,
        &text,
        font,
        color,
    );
}

/// 在 TAB 谱表中部绘制休止符（成品谱面风格几何符号）
pub fn draw_tab_rest(
    painter: &Painter,
    x: f32,
    staff_y: f32,
    staff_height: f32,
    duration: bassoxide_core::types::Duration,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    use bassoxide_core::types::NoteValue;
    let mid_y = staff_y + staff_height * 0.5;
    let s = (settings.tab_font_size * 0.12).clamp(0.85, 1.35);
    let color = theme.rest_color;
    let stroke = egui::Stroke::new((1.4 * s).clamp(1.0, 2.2), color);

    match duration.value {
        NoteValue::Whole | NoteValue::Half => {
            // 全/半休止：谱表中部短粗横块（全休止略偏上）
            let w = (settings.tab_font_size * 0.95 * s).max(8.0);
            let h = (settings.tab_font_size * 0.28 * s).max(3.0);
            let y = if duration.value == NoteValue::Whole {
                mid_y - h * 0.9
            } else {
                mid_y - h * 0.15
            };
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x - w * 0.5, y), Vec2::new(w, h)),
                0.0,
                color,
            );
        }
        NoteValue::Quarter => {
            // 四分休止：锯齿折线
            let h = settings.tab_font_size * 1.15 * s;
            let w = settings.tab_font_size * 0.35 * s;
            let top = mid_y - h * 0.5;
            let pts = [
                Pos2::new(x + w * 0.2, top),
                Pos2::new(x - w * 0.5, top + h * 0.28),
                Pos2::new(x + w * 0.45, top + h * 0.5),
                Pos2::new(x - w * 0.55, top + h * 0.72),
                Pos2::new(x + w * 0.15, top + h),
            ];
            for i in 0..pts.len() - 1 {
                painter.line_segment([pts[i], pts[i + 1]], stroke);
            }
        }
        NoteValue::Eighth | NoteValue::Sixteenth | NoteValue::ThirtySecond | NoteValue::SixtyFourth =>
        {
            // 八分及更短：竖干 + 右侧旗钩（层数随时值）
            let levels = match duration.value {
                NoteValue::Eighth => 1,
                NoteValue::Sixteenth => 2,
                NoteValue::ThirtySecond => 3,
                _ => 4,
            };
            let h = settings.tab_font_size * (0.85 + 0.12 * levels as f32) * s;
            let top = mid_y - h * 0.45;
            let bot = top + h;
            painter.line_segment([Pos2::new(x, top), Pos2::new(x, bot)], stroke);
            for i in 0..levels {
                let fy = top + i as f32 * (settings.tab_font_size * 0.28 * s);
                // 旗钩：向右下弯
                painter.line_segment(
                    [
                        Pos2::new(x, fy),
                        Pos2::new(x + settings.tab_font_size * 0.42 * s, fy + settings.tab_font_size * 0.22 * s),
                    ],
                    stroke,
                );
                painter.circle_filled(
                    Pos2::new(
                        x + settings.tab_font_size * 0.42 * s,
                        fy + settings.tab_font_size * 0.22 * s,
                    ),
                    (1.6 * s).clamp(1.1, 2.4),
                    color,
                );
            }
        }
    }
}

/// 计算 MIDI 音高在五线谱上的纵向位置 (0 = 第一线/最底线, 0.5 = 第一间)
fn pitch_to_staff_offset(midi_note: u8) -> f32 {
    // 简单算法（忽略调号临时升降号，仅假设 C 大调）：
    // C4 (Middle C) = MIDI 60, 它在下加一线 (即 offset = -1.0)
    // D4 = 62 = -0.5
    // E4 = 64 = 0.0 (第一线)
    // F4 = 65 = 0.5 (第一间)
    // G4 = 67 = 1.0 (第二线)
    
    // C 大调自然音阶 (C, D, E, F, G, A, B)
    let octave = (midi_note / 12) as i32 - 1; // MIDI 60 -> Octave 4
    let pitch_class = midi_note % 12;
    
    // 映射 pitch class 到度数 (C=0, D=1, E=2, F=3, G=4, A=5, B=6)
    let degree = match pitch_class {
        0 | 1 => 0.0, // C, C#
        2 | 3 => 1.0, // D, D#
        4 => 2.0,     // E
        5 | 6 => 3.0, // F, F#
        7 | 8 => 4.0, // G, G#
        9 | 10 => 5.0,// A, A#
        11 => 6.0,    // B
        _ => 0.0,
    };
    
    // 以 E4 (MIDI 64) 作为 0.0 (最下面一根线)
    // E4 = Octave 4, degree 2
    let absolute_degree = octave as f32 * 7.0 + degree;
    let e4_degree = 4.0 * 7.0 + 2.0; // 30.0
    
    (absolute_degree - e4_degree) * 0.5
}

/// 在五线谱上绘制符头
pub fn draw_standard_note(
    painter: &Painter,
    note: &Note,
    x: f32,
    staff_y: f32,
    tuning: &bassoxide_core::track::Tuning,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    // 1. 获取音高
    let midi_note = note.midi_note.max(tuning.midi_note(note.string, note.fret).unwrap_or(0));
    if midi_note == 0 { return; }

    // 2. 计算五线谱相对高度
    let display_note = midi_note + 12;
    let offset = pitch_to_staff_offset(display_note);

    // 3. 映射到屏幕坐标（谱表带含 ledger_pad，五线位于垫内）
    let line_spacing = settings.staff_line_spacing;
    let pad = settings.ledger_pad();
    let band_bottom = staff_y + settings.standard_band_height();
    let band_top = staff_y;

    // 最底线 (第 1 线) 位于 pad + 4 * spacing
    let bottom_line_y = staff_y + pad + 4.0 * line_spacing;
    let mut note_y = bottom_line_y - offset * line_spacing;
    // 硬约束：符头落在谱表带内
    note_y = note_y.clamp(band_top + line_spacing * 0.4, band_bottom - line_spacing * 0.4);

    // 4. 画符头 (椭圆)
    let radius = egui::Vec2::new(line_spacing * 0.7, line_spacing * 0.5);
    painter.add(egui::Shape::ellipse_filled(
        Pos2::new(x, note_y),
        radius,
        theme.note_text,
    ));

    // 5. 附加线 (Ledger lines) — 仅在带内绘制
    if offset < 0.0 || offset > 4.0 {
        let mut ledger_offset = if offset < 0.0 { -1.0 } else { 5.0 };
        let end_ledger = offset.round();

        while (offset < 0.0 && ledger_offset >= end_ledger)
            || (offset > 4.0 && ledger_offset <= end_ledger)
        {
            let ledger_y = bottom_line_y - ledger_offset * line_spacing;
            if ledger_y >= band_top && ledger_y <= band_bottom {
                painter.line_segment(
                    [
                        Pos2::new(x - line_spacing * 0.7, ledger_y),
                        Pos2::new(x + line_spacing * 0.7, ledger_y),
                    ],
                    egui::Stroke::new(1.0_f32, theme.note_text),
                );
            }
            if offset < 0.0 {
                ledger_offset -= 1.0;
            } else {
                ledger_offset += 1.0;
            }
        }
    }

    // 6. 画符干 — 长度限制在谱表带内
    let is_stem_up = offset < 2.0;
    let stem_x = if is_stem_up {
        x + radius.x
    } else {
        x - radius.x
    };
    let stem_dir = if is_stem_up { -1.0 } else { 1.0 };
    let ideal = 3.2 * line_spacing;
    let max_len = if is_stem_up {
        (note_y - band_top).max(line_spacing)
    } else {
        (band_bottom - note_y).max(line_spacing)
    };
    let stem_length = ideal.min(max_len);
    let stem_y_end = note_y + stem_dir * stem_length;
    let stem_w = (line_spacing * 0.12).clamp(0.8, 2.0);

    painter.line_segment(
        [Pos2::new(stem_x, note_y), Pos2::new(stem_x, stem_y_end)],
        egui::Stroke::new(stem_w, theme.note_text),
    );
}

