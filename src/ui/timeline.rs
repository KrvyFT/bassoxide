//! 底部轨道列表面板（选轨；谱面类型在顶部工具栏切换）。

use eframe::egui::{self, Align, Layout, RichText, Sense};

use crate::state::AppState;
use crate::ui::material::MaterialPalette;

/// 单行轨道行高（含内边距，避免文字压分隔线）
const ROW_HEIGHT: f32 = 36.0;

pub fn timeline_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("轨道")
                .color(palette.on_surface)
                .size(16.0),
        );
        ui.label(
            RichText::new("点击切换当前显示轨道")
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
    ui.add_space(4.0);

    let mut select_change: Option<usize> = None;
    let current_selected = state.selected_track;

    let full_w = ui.available_width();
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
                        let track_count = song.tracks.len();
                        for i in 0..track_count {
                            let name = song.tracks[i].name.clone();
                            let selected = i == current_selected;
                            let row_fill = if selected {
                                palette.primary_container
                            } else if i % 2 == 0 {
                                palette.surface_container
                            } else {
                                palette.surface_container_high
                            };
                            let text_color = if selected {
                                palette.on_primary_container
                            } else {
                                palette.on_surface
                            };

                            let row_w = ui.available_width();
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                egui::vec2(row_w, ROW_HEIGHT),
                                Sense::click(),
                            );

                            // 背景
                            ui.painter().rect_filled(
                                row_rect,
                                egui::CornerRadius::ZERO,
                                row_fill,
                            );
                            // 底部分隔线（贴底，与文字留出空隙）
                            ui.painter().hline(
                                row_rect.x_range(),
                                row_rect.bottom() - 0.5,
                                egui::Stroke::new(1.0_f32, palette.outline_variant),
                            );

                            // 单行垂直居中：Trk N: Name，超出省略
                            let label = format!("Trk {}: {name}", i + 1);
                            let inner = egui::Rect::from_min_max(
                                egui::pos2(row_rect.left() + 10.0, row_rect.top()),
                                egui::pos2(row_rect.right() - 8.0, row_rect.bottom()),
                            );
                            ui.allocate_new_ui(
                                egui::UiBuilder::new()
                                    .max_rect(inner)
                                    .layout(Layout::left_to_right(Align::Center)),
                                |ui| {
                                    ui.set_min_height(ROW_HEIGHT);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&label).size(13.0).color(text_color),
                                        )
                                        .truncate()
                                        .selectable(false),
                                    );
                                },
                            );

                            row_resp.clone().on_hover_text(&label);

                            if row_resp.clicked() {
                                select_change = Some(i);
                            } else if row_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                // 悬停描边（不改变行高）
                                ui.painter().rect_stroke(
                                    row_rect,
                                    egui::CornerRadius::ZERO,
                                    egui::Stroke::new(1.0_f32, palette.outline_variant),
                                    egui::StrokeKind::Inside,
                                );
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new("打开乐谱后可在此切换轨道")
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
            RichText::new(&state.status_message)
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    });
}
