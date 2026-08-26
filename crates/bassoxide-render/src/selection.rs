//! 选区高亮渲染。

use egui::{Color32, Painter, Pos2, Rect, Vec2};

/// 绘制选区高亮背景
pub fn draw_selection(
    painter: &Painter,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
    painter.rect_filled(
        rect,
        1,
        Color32::from_rgba_premultiplied(60, 120, 200, 40),
    );
}
