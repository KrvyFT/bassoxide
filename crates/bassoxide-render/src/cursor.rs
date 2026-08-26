//! 播放/编辑光标渲染。

use egui::{Color32, Painter, Pos2, Rect, StrokeKind, Vec2};

use crate::colors::Theme;

/// 绘制播放光标（高亮当前拍所在列）
pub fn draw_playback_cursor(
    painter: &Painter,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    theme: &Theme,
) {
    let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
    painter.rect_filled(rect, 2, theme.cursor_color);
}

/// 绘制编辑光标（当前选中位置的边框）
pub fn draw_edit_cursor(painter: &Painter, x: f32, y: f32, width: f32, height: f32) {
    let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
    painter.rect_stroke(
        rect,
        1,
        egui::Stroke::new(2.0_f32, Color32::from_rgb(100, 200, 255)),
        StrokeKind::Outside,
    );
}
