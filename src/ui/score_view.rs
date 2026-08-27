//! 乐谱主视图 — 滚动、缩放、交互。

use egui::{ScrollArea, Sense, Ui};
use bassoxide_render::ScorePainter;

use crate::state::AppState;

/// 绘制乐谱主视图
pub fn score_view(ui: &mut Ui, state: &mut AppState) {
    let mut current_zoom = state.zoom_factor;
    if ui.rect_contains_pointer(ui.max_rect()) {
        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        if zoom_delta != 1.0 {
            current_zoom *= zoom_delta;
            current_zoom = current_zoom.clamp(0.3, 5.0);
        }
    }
    if (current_zoom - state.zoom_factor).abs() > 0.001 {
        state.zoom_factor = current_zoom;
        state.update_zoom();
        state.needs_relayout = true;
    }

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

    ScrollArea::both()
        .auto_shrink([false, false])
        .drag_to_scroll(true)
        .show(ui, |ui| {
            let content_width = layout.total_width + 100.0;
            let content_height = layout.total_height + 100.0;

            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(content_width, content_height),
                Sense::hover(),
            );

            // 填充画布背景（深灰，衬托白色 A4 页面）
            painter.rect_filled(
                response.rect,
                0.0,
                egui::Color32::from_gray(70),
            );

            // 绘制乐谱
            let score_painter = ScorePainter::new(&state.layout_settings, &state.theme);
            score_painter.paint(&painter, song, layout, response.rect.min.to_vec2());
        });
}
