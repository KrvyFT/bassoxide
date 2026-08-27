//! 菜单栏。

use egui::Ui;

use crate::state::AppState;

/// 菜单栏操作结果
pub enum MenuAction {
    None,
    OpenFile,
    AddAudioTrack,
    Quit,
    LightTheme,
    DarkTheme,
    OpenSettings,
}

/// 绘制菜单栏
pub fn menu_bar(ui: &mut Ui, state: &AppState) -> MenuAction {
    let mut action = MenuAction::None;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("文件", |ui| {
            if ui.button("打开乐谱... (Ctrl+O)").clicked() {
                action = MenuAction::OpenFile;
                ui.close_menu();
            }
            if ui.button("添加音频轨...").clicked() {
                action = MenuAction::AddAudioTrack;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("退出").clicked() {
                action = MenuAction::Quit;
                ui.close_menu();
            }
        });

        ui.menu_button("视图", |ui| {
            if ui
                .selectable_label(state.is_light_theme, "浅色主题")
                .clicked()
            {
                action = MenuAction::LightTheme;
                ui.close_menu();
            }
            if ui
                .selectable_label(!state.is_light_theme, "深色主题")
                .clicked()
            {
                action = MenuAction::DarkTheme;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("设置...").clicked() {
                action = MenuAction::OpenSettings;
                ui.close_menu();
            }
        });

        ui.menu_button("帮助", |ui| {
            if ui.button("关于 Bassoxide").clicked() {
                ui.close_menu();
            }
        });
    });

    action
}
