//! 底部轨道列表面板（选轨；谱面类型在顶部工具栏切换）。

use eframe::egui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 轨道名列宽（窄栏 + 超出省略）
const TRACK_NAME_WIDTH: f32 = 200.0;

fn ellipsize(ui: &egui::Ui, text: &str, max_width: f32, font: &egui::FontId) -> String {
    let full_w = ui.fonts(|f| {
        f.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    });
    if full_w <= max_width {
        return text.to_owned();
    }
    let ellipsis = "…";
    let ellipsis_w = ui.fonts(|f| {
        f.layout_no_wrap(ellipsis.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    });
    let budget = (max_width - ellipsis_w).max(0.0);
    let mut lo = 0usize;
    let mut hi = text.chars().count();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let prefix: String = text.chars().take(mid).collect();
        let w = ui.fonts(|f| {
            f.layout_no_wrap(prefix.clone(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
        });
        if w <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = text.chars().take(lo).collect();
    out.push_str(ellipsis);
    out
}

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("轨道")
                .color(palette.on_surface)
                .size(16.0),
        );
        ui.label(
            egui::RichText::new("点击切换当前显示轨道")
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
    ui.add_space(4.0);

    let mut select_change: Option<usize> = None;
    let current_selected = state.selected_track;

    let full_w = ui.available_width();
    // 直角整块列表，行与行密接，无卡片圆角
    egui::Frame::NONE
        .fill(palette.surface_container_high)
        .corner_radius(egui::CornerRadius::ZERO)
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_width(full_w);
            egui::ScrollArea::vertical()
                .max_height(ui.available_height().max(80.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(song) = &state.song {
                        for (i, track) in song.tracks.iter().enumerate() {
                            let row_w = ui.available_width();
                            let row_h = 32.0;
                            let selected = i == current_selected;
                            let row_fill = if selected {
                                palette.primary_container
                            } else if i % 2 == 0 {
                                palette.surface_container
                            } else {
                                palette.surface_container_high
                            };

                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                egui::vec2(row_w, row_h),
                                egui::Sense::click(),
                            );
                            ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, row_fill);

                            // 底部分隔线（直角）
                            ui.painter().hline(
                                row_rect.x_range(),
                                row_rect.bottom() - 0.5,
                                egui::Stroke::new(1.0_f32, palette.outline_variant),
                            );

                            let label = format!("Trk {}: {}", i + 1, track.name);
                            let name_font = egui::FontId::proportional(13.0);
                            let label_draw =
                                ellipsize(ui, &label, TRACK_NAME_WIDTH - 4.0, &name_font);

                            let name_rect = egui::Rect::from_min_size(
                                row_rect.min + egui::vec2(8.0, 0.0),
                                egui::vec2(TRACK_NAME_WIDTH, row_h),
                            );
                            ui.painter().text(
                                name_rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                label_draw,
                                name_font,
                                if selected {
                                    palette.on_primary_container
                                } else {
                                    palette.on_surface
                                },
                            );

                            row_resp.clone().on_hover_text(&label);

                            if row_resp.clicked() {
                                select_change = Some(i);
                            } else if row_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("打开乐谱后可在此切换轨道")
                                .color(palette.on_surface_variant),
                        );
                    }
                });
        });

    if let Some(idx) = select_change {
        state.select_track(idx);
    }

    ui.add_space(6.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&state.status_message)
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
}
