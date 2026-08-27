//! 设置页面 — 谱面排版参数。

use egui::Ui;

use crate::state::{AppState, ScorePrefs};
use crate::ui::material::MaterialPalette;

/// 绘制设置窗口；返回是否请求关闭
pub fn settings_window(ctx: &egui::Context, state: &mut AppState) -> bool {
    let palette = MaterialPalette::for_mode(state.is_light_theme);
    let mut close = false;
    let mut changed = false;

    egui::Window::new("设置")
        .id(egui::Id::new("settings_page"))
        .collapsible(false)
        .resizable(true)
        .default_width(440.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("谱面设置")
                    .strong()
                    .size(16.0)
                    .color(palette.primary),
            );
            ui.label(
                egui::RichText::new("调整后立即作用于当前乐谱排版")
                    .size(11.0)
                    .color(palette.on_surface_variant),
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            score_settings_form(ui, state, &palette, &mut changed);

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("恢复默认").color(palette.on_surface),
                        )
                        .fill(palette.secondary_container),
                    )
                    .clicked()
                {
                    state.score_prefs = ScorePrefs::default();
                    changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("完成").color(palette.on_primary),
                            )
                            .fill(palette.primary),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });

    if changed {
        state.apply_score_prefs();
    }

    close
}

fn score_settings_form(
    ui: &mut Ui,
    state: &mut AppState,
    palette: &MaterialPalette,
    changed: &mut bool,
) {
    let prefs = &mut state.score_prefs;

    ui.label(
        egui::RichText::new("约束优先级：① 音符必须在谱表内  ② 谱表必须在纸张内；冲突时自动调节其它参数")
            .size(11.0)
            .color(palette.on_surface_variant),
    );
    ui.add_space(8.0);

    ui.label(egui::RichText::new("纸张大小").color(palette.on_surface));
    ui.horizontal_wrapped(|ui| {
        for size in bassoxide_layout::PaperSize::ALL {
            let selected = prefs.paper_size == size;
            let label = format!("{} ({})", size.label(), size.description());
            if ui.selectable_label(selected, label).clicked() && !selected {
                prefs.paper_size = size;
                *changed = true;
            }
        }
    });
    ui.label(
        egui::RichText::new("小节宽度、音符与符杆随纸张相对 A4 自动缩放")
            .size(11.0)
            .color(palette.on_surface_variant),
    );
    ui.add_space(8.0);

    ui.label(egui::RichText::new("字体大小").color(palette.on_surface));
    if ui
        .add(egui::Slider::new(&mut prefs.font_size, 8.0..=28.0).suffix(" px"))
        .changed()
    {
        *changed = true;
    }
    ui.add_space(8.0);

    ui.label(egui::RichText::new("线间距（弦距 / 五线距）").color(palette.on_surface));
    if ui
        .add(egui::Slider::new(&mut prefs.line_spacing, 8.0..=28.0).suffix(" px"))
        .changed()
    {
        *changed = true;
    }
    ui.add_space(8.0);

    ui.label(egui::RichText::new("行间距（谱行间距）").color(palette.on_surface));
    if ui
        .add(egui::Slider::new(&mut prefs.row_spacing, 40.0..=200.0).suffix(" px"))
        .changed()
    {
        *changed = true;
    }
    ui.add_space(8.0);

    ui.label(egui::RichText::new("每行小节数").color(palette.on_surface));
    let mut mpl = prefs.measures_per_line as i32;
    let resp = ui.add(egui::Slider::new(&mut mpl, 0..=12).custom_formatter(|n, _| {
        if n < 0.5 {
            "自动".into()
        } else {
            format!("{n:.0}")
        }
    }));
    if resp.changed() {
        prefs.measures_per_line = mpl as u8;
        *changed = true;
    }
    ui.label(
        egui::RichText::new("0 = 按页面宽度自动换行；>0 时每行固定小节并自动铺满页宽")
            .size(11.0)
            .color(palette.on_surface_variant),
    );
}
