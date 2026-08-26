//! 音符数字渲染（六线谱模式）。

use egui::{Painter, Pos2, Rect, Vec2};

use bassoxide_core::note::Note;
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_layout::tablature;

use crate::colors::Theme;

/// 在六线谱上绘制单个音符（品格数字）
pub fn draw_tab_note(
    painter: &Painter,
    note: &Note,
    x: f32,
    staff_y: f32,
    settings: &LayoutSettings,
    theme: &Theme,
    is_selected: bool,
) {
    let string_y = staff_y + tablature::string_y_offset(note.string, settings);
    let text = tablature::fret_display(note.fret);

    let font = egui::FontId::new(settings.tab_font_size, egui::FontFamily::Monospace);
    let color = if is_selected {
        theme.selected_note
    } else {
        theme.note_text
    };

    // 先画一个小背景矩形清除弦线（让数字清晰可读）
    let text_size = Vec2::new(settings.tab_font_size * 0.8, settings.tab_font_size);
    let bg_rect = Rect::from_center_size(
        Pos2::new(x, string_y),
        text_size + Vec2::new(4.0, 2.0),
    );
    painter.rect_filled(bg_rect, 0.0, theme.background);

    // 绘制品格数字
    painter.text(
        Pos2::new(x, string_y),
        egui::Align2::CENTER_CENTER,
        &text,
        font,
        color,
    );
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
    theme: &Theme,
) {
    // 1. 获取音高
    let midi_note = note.midi_note.max(tuning.midi_note(note.string, note.fret).unwrap_or(0));
    if midi_note == 0 { return; }

    // 2. 计算五线谱相对高度
    // 五线谱 0.0 = 最底线, 4.0 = 最顶线。绘制是从上往下的 y 坐标，所以最底线是 y + 4 * spacing
    // 注意，吉他五线谱实际记谱比实际音高高八度 (高八度记谱)。我们这里按照实际音高直接映射，低音会需要很多下加线。
    // 为了简单显示，我们将吉他谱自动提升一个八度计算 (+12 MIDI)
    let display_note = midi_note + 12;
    let offset = pitch_to_staff_offset(display_note);
    
    // 3. 映射到屏幕坐标
    // 假设 tab_string_spacing * 0.8 是五线谱间距
    let line_spacing = 15.0 * 0.8; // 这里硬编码 15.0 为默认间距
    
    // 最底线 (第 1 线) 位于 s = 4
    let bottom_line_y = staff_y + 4.0 * line_spacing;
    let note_y = bottom_line_y - offset * line_spacing;
    
    // 4. 画符头 (椭圆)
    let radius = egui::Vec2::new(line_spacing * 0.4, line_spacing * 0.3);
    painter.add(egui::Shape::ellipse_filled(Pos2::new(x, note_y), radius, theme.note_text));
    
    // 5. 附加线 (Ledger lines)
    if offset < 0.0 || offset > 4.0 {
        let mut ledger_offset = if offset < 0.0 { -1.0 } else { 5.0 };
        let end_ledger = offset.round();
        
        while (offset < 0.0 && ledger_offset >= end_ledger) || (offset > 4.0 && ledger_offset <= end_ledger) {
            let ledger_y = bottom_line_y - ledger_offset * line_spacing;
            painter.line_segment(
                [Pos2::new(x - line_spacing * 0.7, ledger_y), Pos2::new(x + line_spacing * 0.7, ledger_y)],
                egui::Stroke::new(1.0_f32, theme.note_text),
            );
            if offset < 0.0 {
                ledger_offset -= 1.0;
            } else {
                ledger_offset += 1.0;
            }
        }
    }
    
    // 6. 画符干 (Stem)
    // 根据在五线谱上的相对位置：通常第三线 (offset = 2.0) 以下符干朝上(画在右侧)，第三线及以上符干朝下(画在左侧)
    let is_stem_up = offset < 2.0;
    let stem_x = if is_stem_up { x + radius.x } else { x - radius.x };
    let stem_dir = if is_stem_up { -1.0 } else { 1.0 };
    // 符干长度大约跨越 3.5 个线间距
    let stem_length = 3.5 * line_spacing;
    let stem_y_end = note_y + stem_dir * stem_length;
    
    painter.line_segment(
        [Pos2::new(stem_x, note_y), Pos2::new(stem_x, stem_y_end)],
        egui::Stroke::new(1.2_f32, theme.note_text),
    );
}


/// 绘制休止符标记
pub fn draw_rest(
    painter: &Painter,
    x: f32,
    staff_y: f32,
    staff_height: f32,
    theme: &Theme,
) {
    let font = egui::FontId::new(14.0, egui::FontFamily::Monospace);
    let center_y = staff_y + staff_height / 2.0;

    painter.text(
        Pos2::new(x, center_y),
        egui::Align2::CENTER_CENTER,
        "𝄾", // 四分休止符 unicode
        font,
        theme.rest_color,
    );
}
