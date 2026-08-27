//! 乐谱主视图 — 滚动、缩放、交互。

use egui::{ScrollArea, Sense, Ui};
use bassoxide_render::ScorePainter;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 绘制乐谱主视图
pub fn score_view(ui: &mut Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

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
            ui.painter().rect_filled(ui.max_rect(), 0.0, palette.surface);
            ui.centered_and_justified(|ui| {
                ui.heading(egui::RichText::new("Bassoxide").color(palette.on_surface));
                ui.label(
                    egui::RichText::new("按 Ctrl+O 打开 Guitar Pro 文件 (.gp5)")
                        .color(palette.on_surface_variant),
                );
            });
            return;
        }
    };

    let viewport = ui.available_size();
    let page_w = layout.total_width;
    let page_h = layout.total_height;
    // 水平居中：视口比页面宽时两侧留 Material You 背景
    let center_pad_x = ((viewport.x - page_w).max(0.0) * 0.5).floor();
    let content_width = (page_w + center_pad_x * 2.0 + 48.0).max(viewport.x);
    let content_height = (page_h + 64.0).max(viewport.y);

    ScrollArea::both()
        .auto_shrink([false, false])
        .drag_to_scroll(true)
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(content_width, content_height),
                Sense::hover(),
            );

            // 画布背景：Material You surface；纸张本身在 ScorePainter 中为纯白
            painter.rect_filled(response.rect, 0.0, palette.surface);

            let offset = egui::vec2(
                response.rect.min.x + center_pad_x + 24.0,
                response.rect.min.y + 24.0,
            );

            let score_painter = ScorePainter::new(&state.layout_settings, &state.theme);
            score_painter.paint(&painter, song, layout, offset);
        });
}
