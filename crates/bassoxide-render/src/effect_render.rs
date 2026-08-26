//! 进阶奏法效果绘制（推弦、滑音、揉弦、泛音等）

use egui::{Painter, Pos2, Stroke, Color32, Vec2, Shape};
use bassoxide_core::effects::*;
use bassoxide_core::note::Note;
use crate::colors::Theme;

/// 绘制泛音符号
pub fn draw_harmonic(
    painter: &Painter,
    _harmonic: &HarmonicEffect,
    x: f32,
    y: f32,
    theme: &Theme,
) {
    // 在音符两侧画尖括号 `< >` 比如 `<12>`
    let font = egui::FontId::new(14.0, egui::FontFamily::Monospace);
    let mut color = theme.note_text;
    color[3] = 150; // 稍微变淡

    painter.text(
        Pos2::new(x - 8.0, y),
        egui::Align2::CENTER_CENTER,
        "<",
        font.clone(),
        color,
    );
    painter.text(
        Pos2::new(x + 8.0, y),
        egui::Align2::CENTER_CENTER,
        ">",
        font,
        color,
    );
}

/// 绘制滑音斜线
pub fn draw_slide(
    painter: &Painter,
    _slide_type: &SlideType,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    theme: &Theme,
) {
    let stroke = Stroke::new(1.5, theme.note_text);
    // 从第一个音的右下角滑到下一个音的左上角 (近似)
    painter.line_segment(
        [Pos2::new(x1 + 6.0, y1 + 2.0), Pos2::new(x2 - 6.0, y2 - 2.0)],
        stroke,
    );
}

/// 绘制揉弦波浪线
pub fn draw_vibrato(
    painter: &Painter,
    x: f32,
    y: f32, // y 通常是音符上方
    width: f32,
    theme: &Theme,
) {
    let stroke = Stroke::new(1.5, theme.note_text);
    let mut points = vec![];
    let wave_len = 4.0;
    let wave_height = 2.0;
    
    let mut curr_x = x;
    let mut up = true;
    while curr_x < x + width {
        let offset_y = if up { -wave_height } else { wave_height };
        points.push(Pos2::new(curr_x, y + offset_y));
        curr_x += wave_len;
        up = !up;
    }
    
    if points.len() > 1 {
        painter.add(Shape::line(points, stroke));
    }
}

/// 绘制推弦箭头
pub fn draw_bend(
    painter: &Painter,
    bend: &BendEffect,
    x: f32,
    y: f32, // 音符中心 y
    theme: &Theme,
) {
    let stroke = Stroke::new(1.5, theme.note_text);
    // 简单绘制一个向上的弯折箭头
    let start = Pos2::new(x + 6.0, y);
    let top = Pos2::new(x + 12.0, y - 15.0); // 向上弯折
    
    // 贝塞尔曲线或简单的两段线
    painter.line_segment([start, top], stroke);
    
    // 画箭头
    painter.line_segment([top, Pos2::new(top.x - 3.0, top.y + 4.0)], stroke);
    painter.line_segment([top, Pos2::new(top.x + 3.0, top.y + 4.0)], stroke);
    
    // 标注文字 (如 Full, 1/2)
    let max_val = bend.points.iter().map(|p| p.value).max().unwrap_or(4); // 4 = 100 cents = 1 semitone
    let text = match max_val {
        v if v >= 8 => "Full",
        v if v >= 4 => "1/2",
        v if v >= 2 => "1/4",
        _ => "bend",
    };
    
    painter.text(
        Pos2::new(top.x, top.y - 6.0),
        egui::Align2::CENTER_BOTTOM,
        text,
        egui::FontId::new(10.0, egui::FontFamily::Proportional),
        theme.note_text,
    );
}

/// 绘制放开延音 (Let Ring) / 闷音 (P.M.)
pub fn draw_text_line(
    painter: &Painter,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    theme: &Theme,
) {
    let font = egui::FontId::new(10.0, egui::FontFamily::Proportional);
    let color = theme.note_text;
    
    painter.text(
        Pos2::new(x, y),
        egui::Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    
    // 画虚线
    let mut curr_x = x + 25.0; // 文字后面
    let stroke = Stroke::new(1.0, color);
    while curr_x < x + width {
        painter.line_segment([Pos2::new(curr_x, y), Pos2::new(curr_x + 4.0, y)], stroke);
        curr_x += 8.0;
    }
}
