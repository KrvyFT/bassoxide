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
