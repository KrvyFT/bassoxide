//! Bassoxide 应用主体 — eframe::App 实现。

use eframe::egui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;
use crate::ui::{menu_bar, toolbar, transport};

/// eframe 应用
pub struct BassoxideApp {
    pub state: AppState,
}

impl BassoxideApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // 加载系统 CJK 字体，解决中文显示为方框的问题
        Self::configure_fonts(&cc.egui_ctx);

        let mut app = Self {
            state: AppState::default(),
        };
        // 默认浅色 Material You
        MaterialPalette::for_mode(app.state.is_light_theme).apply_to_ctx(&cc.egui_ctx);
        app.state.theme_dirty = false;
        app
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
            self.open_path(&path);
        }
    }

    /// 从路径加载乐谱
    pub fn open_path(&mut self, path: &std::path::Path) {
        let path_str = path.display().to_string();
        match bassoxide_io::load_from_path(path) {
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

impl eframe::App for BassoxideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 应用 Material You 主题
        if self.state.theme_dirty {
            MaterialPalette::for_mode(self.state.is_light_theme).apply_to_ctx(ctx);
            self.state.theme_dirty = false;
        }

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

        let palette = MaterialPalette::for_mode(self.state.is_light_theme);

        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.surface_container)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                let action = menu_bar::menu_bar(ui, &self.state);
                match action {
                    menu_bar::MenuAction::OpenFile => self.open_file(),
                    menu_bar::MenuAction::Quit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    menu_bar::MenuAction::LightTheme => self.state.set_light_theme(true),
                    menu_bar::MenuAction::DarkTheme => self.state.set_light_theme(false),
                    menu_bar::MenuAction::None => {}
                }
            });

        // 工具栏
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.surface)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ctx, |ui| {
                toolbar::toolbar(ui, &mut self.state);
            });

        // 播放控制条
        egui::TopBottomPanel::top("transport")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.primary_container)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ctx, |ui| {
                transport::transport_bar(ui, &mut self.state);
            });

        // 底部：混音台 + 状态栏作为整体
        egui::TopBottomPanel::bottom("mixer_bottom")
            .resizable(true)
            .default_height(220.0)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.surface_container)
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .corner_radius(egui::CornerRadius {
                        nw: 16,
                        ne: 16,
                        sw: 0,
                        se: 0,
                    }),
            )
            .show(ctx, |ui| {
                crate::ui::timeline::timeline_panel(ui, &mut self.state);
            });

        // 主内容区：Material You surface 衬底
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(palette.surface)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                crate::ui::score_view::score_view(ui, &mut self.state);
            });
    }
}
