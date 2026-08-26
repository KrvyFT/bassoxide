//! 乐谱主视图 — 滚动、缩放、交互。

use egui::{ScrollArea, Sense, Ui};
use bassoxide_render::ScorePainter;

use crate::state::AppState;

/// 绘制乐谱主视图
pub fn score_view(ui: &mut Ui, state: &mut AppState) {
    let (song, layout) = match (&state.song, &state.layout) {
        (Some(s), Some(l)) => (s, l),
        _ => {
            // 无乐谱时显示欢迎信息
            ui.centered_and_justified(|ui| {
                ui.heading("Bassoxide");
                ui.label("按 Ctrl+O 打开 Guitar Pro 文件 (.gp5)");
            });
            return;
        }
    };

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let content_height = layout.total_height + 100.0;

            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(available_width, content_height),
                Sense::click_and_drag(),
            );

            // 填充背景
            painter.rect_filled(
                response.rect,
                0.0,
                state.theme.background,
            );

            // 绘制乐谱
            let score_painter = ScorePainter::new(&state.layout_settings, &state.theme);
            score_painter.paint(&painter, song, layout, state.scroll_y);
        });
}
