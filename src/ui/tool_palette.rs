//! 左侧谱面工具栏：音符 / 休止符时值与小节标记。

use bassoxide_core::types::NoteValue;
use egui::{RichText, Sense, Ui, Vec2};

use crate::edit;
use crate::state::{AppState, EditToolKind};
use crate::ui::material::MaterialPalette;

/// 绘制左侧工具选择面板；若切换工具返回 true
pub fn tool_palette(ui: &mut Ui, state: &mut AppState, palette: &MaterialPalette) -> bool {
    let mut changed = false;

    ui.add_space(6.0);
    ui.label(
        RichText::new("工具")
            .size(13.0)
            .color(palette.on_surface)
            .strong(),
    );
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(6.0);

    ui.label(RichText::new("音符").size(11.0).color(palette.on_surface_variant));
    ui.add_space(4.0);
    changed |= duration_grid(ui, state, palette, EditToolKind::Note);

    ui.add_space(10.0);
    ui.label(RichText::new("休止符").size(11.0).color(palette.on_surface_variant));
    ui.add_space(4.0);
    changed |= duration_grid(ui, state, palette, EditToolKind::Rest);

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("标记").size(11.0).color(palette.on_surface_variant));
    ui.add_space(4.0);

    let marker_selected = state.edit_tool.kind == EditToolKind::Marker;
    if tool_button(
        ui,
        palette,
        marker_selected,
        "标记",
        "小节排练标记 / 跳转",
        Vec2::new(ui.available_width(), 32.0),
    ) {
        edit::select_edit_tool(state, EditToolKind::Marker, None);
        changed = true;
    }

    ui.add_space(8.0);
    if ui
        .add_sized(
            Vec2::new(ui.available_width(), 28.0),
            egui::Button::new(RichText::new("附点 ·").color(if state.edit_tool.dotted {
                palette.on_primary
            } else {
                palette.on_surface
            }))
            .fill(if state.edit_tool.dotted {
                palette.primary
            } else {
                palette.surface_container_high
            }),
        )
        .clicked()
    {
        state.edit_tool.dotted = !state.edit_tool.dotted;
        if matches!(
            state.edit_tool.kind,
            EditToolKind::Note | EditToolKind::Rest
        ) {
            edit::apply_duration_grid(state);
        }
        changed = true;
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(6.0);
    if let Some(n) = edit::slots_per_measure(state) {
        ui.label(
            RichText::new(format!("每小节 {} 格", n))
                .size(11.0)
                .color(palette.on_surface_variant),
        );
    } else {
        ui.label(
            RichText::new("此时值无法整除拍号")
                .size(11.0)
                .color(palette.error),
        );
    }
    ui.label(
        RichText::new("←→ 切换空格  ↑↓ 换弦")
            .size(10.0)
            .color(palette.on_surface_variant),
    );
    ui.add_space(4.0);
    if ui.button("在光标处应用工具").clicked() {
        edit::apply_tool_at_cursor(state);
        changed = true;
    }

    changed
}

fn duration_grid(
    ui: &mut Ui,
    state: &mut AppState,
    palette: &MaterialPalette,
    kind: EditToolKind,
) -> bool {
    let mut changed = false;
    let note_items: &[(NoteValue, &str, &str)] = &[
        (NoteValue::Whole, "1", "全音符"),
        (NoteValue::Half, "1/2", "二分音符"),
        (NoteValue::Quarter, "1/4", "四分音符"),
        (NoteValue::Eighth, "1/8", "八分音符"),
        (NoteValue::Sixteenth, "1/16", "十六分音符"),
        (NoteValue::ThirtySecond, "1/32", "三十二分音符"),
    ];
    let rest_items: &[(NoteValue, &str, &str)] = &[
        (NoteValue::Whole, "全", "全休止符"),
        (NoteValue::Half, "二", "二分休止符"),
        (NoteValue::Quarter, "四", "四分休止符"),
        (NoteValue::Eighth, "八", "八分休止符"),
        (NoteValue::Sixteenth, "16", "十六分休止符"),
        (NoteValue::ThirtySecond, "32", "三十二分休止符"),
    ];
    let list = if kind == EditToolKind::Rest {
        rest_items
    } else {
        note_items
    };

    let cols = 2;
    let width = ui.available_width();
    let btn_w = ((width - 4.0) / cols as f32).max(36.0);
    let btn_h = 36.0;

    egui::Grid::new(format!("tool_dur_{kind:?}"))
        .num_columns(cols)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (i, (value, label, tip)) in list.iter().enumerate() {
                let selected = state.edit_tool.kind == kind && state.edit_tool.duration == *value;
                if tool_button(
                    ui,
                    palette,
                    selected,
                    label,
                    tip,
                    Vec2::new(btn_w, btn_h),
                ) {
                    edit::select_edit_tool(state, kind, Some(*value));
                    changed = true;
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
            if list.len() % cols != 0 {
                ui.end_row();
            }
        });

    changed
}

fn tool_button(
    ui: &mut Ui,
    palette: &MaterialPalette,
    selected: bool,
    label: &str,
    tip: &str,
    size: Vec2,
) -> bool {
    let fill = if selected {
        palette.primary
    } else {
        palette.surface_container_high
    };
    let text = if selected {
        palette.on_primary
    } else {
        palette.on_surface
    };
    let response = ui.add_sized(
        size,
        egui::Button::new(RichText::new(label).size(13.0).color(text).strong())
            .fill(fill)
            .sense(Sense::click()),
    );
    let clicked = response.clicked();
    response.on_hover_text(tip);
    clicked
}
