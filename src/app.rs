//! Bassoxide 应用主体 — eframe::App 实现。

use eframe::egui;

use crate::state::AppState;
use crate::ui::{menu_bar, score_view, toolbar, transport};

/// eframe 应用
pub struct BassoxideApp {
    pub state: AppState,
}

impl BassoxideApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // 加载系统 CJK 字体，解决中文显示为方框的问题
        Self::configure_fonts(&cc.egui_ctx);

        Self {
            state: AppState::default(),
        }
    }

    /// 配置字体：追加系统中文字体作为 fallback
    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // 尝试按优先级加载系统 CJK 字体
        let cjk_font_paths = [
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ];

        let mut loaded = false;
        for path in &cjk_font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "noto_sans_cjk".to_string(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );

                // 将 CJK 字体追加到 Proportional 和 Monospace 的 fallback 链
                if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    families.push("noto_sans_cjk".to_string());
                }
                if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    families.push("noto_sans_cjk".to_string());
                }

                tracing::info!("已加载 CJK 字体: {path}");
                loaded = true;
                break;
            }
        }

        if !loaded {
            tracing::warn!("未找到系统 CJK 字体，中文可能无法正常显示");
        }

        ctx.set_fonts(fonts);
    }

    /// 处理文件打开
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Guitar Pro", &["gp5", "gp4", "gp3", "gpx", "gp"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            let path_str = path.display().to_string();
            match bassoxide_io::load_from_path(&path) {
                Ok(song) => {
                    self.state.load_song(song, Some(path_str));
                }
                Err(e) => {
                    self.state.status_message = format!("加载失败: {e}");
                    tracing::error!("Failed to load file: {e}");
                }
            }
        }
    }
}

impl eframe::App for BassoxideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 键盘快捷键
        if ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command) {
            self.open_file();
        }

        // 更新可用宽度
        let available_width = ctx.screen_rect().width();
        self.state.update_available_width(available_width);

        // 需要时重新排版
        if self.state.needs_relayout {
            self.state.relayout();
        }

        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            let action = menu_bar::menu_bar(ui, &self.state);
            match action {
                menu_bar::MenuAction::OpenFile => self.open_file(),
                menu_bar::MenuAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                menu_bar::MenuAction::None => {}
            }
        });

        // 工具栏
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            toolbar::toolbar(ui, &self.state);
        });

        // 播放控制条
        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            transport::transport_bar(ui);
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new(&self.state.status_message)
                            .size(11.0)
                            .color(egui::Color32::from_gray(160)),
                    );
                });
            });

        // 主内容区
        egui::CentralPanel::default().show(ctx, |ui| {
            score_view::score_view(ui, &mut self.state);
        });
    }
}
