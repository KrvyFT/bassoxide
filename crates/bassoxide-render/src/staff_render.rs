//! 谱表线绘制。

use egui::{Painter, Pos2, Stroke};

use bassoxide_layout::spacing::LayoutSettings;

use crate::colors::Theme;

/// 绘制六线谱 (Tab) 的水平弦线
pub fn draw_tab_staff(
    painter: &Painter,
    x: f32,
    y: f32,
    width: f32,
    string_count: usize,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    let stroke = Stroke::new(1.0_f32, theme.staff_line);

    for s in 0..string_count {
        let line_y = y + s as f32 * settings.tab_string_spacing;
        painter.line_segment(
            [Pos2::new(x, line_y), Pos2::new(x + width, line_y)],
            stroke,
        );
    }
}

/// 绘制五线谱 (Standard) 的五根水平线
pub fn draw_standard_staff(
    painter: &Painter,
    x: f32,
    y: f32,
    width: f32,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    let stroke = Stroke::new(1.0_f32, theme.staff_line);
    let line_spacing = settings.staff_line_spacing;

    for s in 0..5 {
        let line_y = y + s as f32 * line_spacing;
        painter.line_segment(
            [Pos2::new(x, line_y), Pos2::new(x + width, line_y)],
            stroke,
        );
    }
}

/// 绘制小节线
pub fn draw_bar_line(
    painter: &Painter,
    x: f32,
    y: f32,
    height: f32,
    theme: &Theme,
) {
    let stroke = Stroke::new(1.2_f32, theme.bar_line);
    painter.line_segment(
        [Pos2::new(x, y), Pos2::new(x, y + height)],
        stroke,
    );
}

/// 绘制终止双小节线
pub fn draw_final_bar_line(
    painter: &Painter,
    x: f32,
    y: f32,
    height: f32,
    theme: &Theme,
) {
    // 细线
    let thin = Stroke::new(1.0_f32, theme.bar_line);
    painter.line_segment(
        [Pos2::new(x - 4.0, y), Pos2::new(x - 4.0, y + height)],
        thin,
    );
    // 粗线
    let thick = Stroke::new(3.0_f32, theme.bar_line);
    painter.line_segment(
        [Pos2::new(x, y), Pos2::new(x, y + height)],
        thick,
    );
}

/// 绘制 "TAB" 谱号
pub fn draw_tab_clef(
    painter: &Painter,
    x: f32,
    y: f32,
    string_count: usize,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    let tab_positions = bassoxide_layout::tablature::tab_clef_positions(string_count, settings);
    let letters = ["T", "A", "B"];
    let font = egui::FontId::new(14.0, egui::FontFamily::Monospace);

    for (i, letter) in letters.iter().enumerate() {
        if let Some(&pos_y) = tab_positions.get(i) {
            painter.text(
                Pos2::new(x + 10.0, y + pos_y),
                egui::Align2::CENTER_CENTER,
                *letter,
                font.clone(),
                theme.clef_color,
            );
        }
    }
}

/// 绘制拍号（上下分数对齐，中间横线）
pub fn draw_time_signature(
    painter: &Painter,
    x: f32,
    y: f32,
    numerator: u8,
    denominator: u8,
    staff_height: f32,
    settings: &LayoutSettings,
    theme: &Theme,
) {
    let font_size = (settings.tab_font_size * 1.35).clamp(12.0, 28.0);
    let font = egui::FontId::new(font_size, egui::FontFamily::Proportional);
    let cx = x;
    let mid_y = y + staff_height * 0.5;
    let gap = (font_size * 0.55).max(8.0);

    // 分子
    painter.text(
        Pos2::new(cx, mid_y - gap),
        egui::Align2::CENTER_BOTTOM,
        numerator.to_string(),
        font.clone(),
        theme.time_sig_color,
    );
    // 分数线
    let bar_w = (font_size * 0.85).max(10.0);
    painter.line_segment(
        [
            Pos2::new(cx - bar_w * 0.5, mid_y),
            Pos2::new(cx + bar_w * 0.5, mid_y),
        ],
        Stroke::new(1.5_f32, theme.time_sig_color),
    );
    // 分母
    painter.text(
        Pos2::new(cx, mid_y + gap),
        egui::Align2::CENTER_TOP,
        denominator.to_string(),
        font,
        theme.time_sig_color,
    );
}
