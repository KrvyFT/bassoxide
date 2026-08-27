//! Bassoxide 应用主体 — eframe::App 实现。

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use eframe::egui;

use crate::project::{self, BsoLoaded};
use crate::state::AppState;
use crate::ui::audio_track::AudioTrack;
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

    /// 处理文件打开（GP / .bso）
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Bassoxide / Guitar Pro", &["bso", "gp5", "gp4", "gp3", "gpx", "gp"])
            .add_filter("Bassoxide 工程", &["bso"])
            .add_filter("Guitar Pro", &["gp5", "gp4", "gp3", "gpx", "gp"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_path(&path);
        }
    }

    fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Bassoxide 工程", &["bso"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_bso(&path);
        }
    }

    fn add_audio_track(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Audio",
                &["wav", "flac", "mp3", "ogg", "m4a", "aac", "aiff", "aif"],
            )
            .add_filter("All", &["*"])
            .pick_file()
        {
            self.load_audio_path(&path);
        }
    }

    pub fn load_audio_path(&mut self, path: &Path) {
        if self.state.busy_message.is_some() {
            self.state.status_message = "正在处理中，请稍候".into();
            return;
        }
        let path_buf = path.to_path_buf();
        let song = self.state.song.clone();
        let (tx, rx) = mpsc::channel();
        self.state.busy_message = Some("处理中…".into());
        self.state.audio_job_rx = Some(rx);
        self.state.status_message = "正在解码音频…".into();
        thread::spawn(move || {
            let result = AudioTrack::load(&path_buf, song.as_ref());
            let _ = tx.send(result);
        });
    }

    /// 从路径加载乐谱或工程
    pub fn open_path(&mut self, path: &Path) {
        let is_bso = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("bso"))
            .unwrap_or(false);
        if is_bso {
            self.open_bso(path);
            return;
        }
        let path_str = path.display().to_string();
        match bassoxide_io::load_from_path(path) {
            Ok(song) => {
                self.state.project_path = None;
                self.state.load_song(song, Some(path_str));
            }
            Err(e) => {
                self.state.status_message = format!("加载失败: {e}");
                tracing::error!("Failed to load file: {e}");
            }
        }
    }

    pub fn open_bso(&mut self, path: &Path) {
        if self.state.busy_message.is_some() {
            self.state.status_message = "正在处理中，请稍候".into();
            return;
        }
        match project::load_bso(path) {
            Ok(loaded) => self.apply_bso(loaded, path.to_path_buf()),
            Err(e) => {
                self.state.status_message = format!("打开工程失败: {e}");
                tracing::error!("Failed to load .bso: {e}");
            }
        }
    }

    fn apply_bso(&mut self, loaded: BsoLoaded, path: PathBuf) {
        let meta = loaded.meta.clone();
        project::apply_meta_and_song(&mut self.state, &loaded, path);
        self.state.status_message = format!(
            "已打开工程: {} | {} 轨道",
            self.state
                .file_path
                .clone()
                .unwrap_or_else(|| "unnamed.bso".into()),
            self.state.song.as_ref().map(|s| s.track_count()).unwrap_or(0)
        );

        // 异步解码内嵌音频
        if let Some((name, bytes)) = loaded.audio_file {
            let song = self.state.song.clone();
            let (tx, rx) = mpsc::channel();
            self.state.busy_message = Some("处理中…".into());
            self.state.audio_job_rx = Some(rx);
            let sync = meta.audio_sync_offset_secs;
            let pps = meta.audio_pixels_per_second;
            let view = meta.audio_view_start_secs;
            thread::spawn(move || {
                let result = AudioTrack::from_bytes(bytes, &name, song.as_ref()).map(|mut t| {
                    t.sync_offset_secs = sync;
                    t.pixels_per_second = pps;
                    t.view_start_secs = view;
                    t
                });
                let _ = tx.send(result);
            });
        } else if let Some((sr, samples)) = loaded.pcm {
            let song = self.state.song.clone();
            let (tx, rx) = mpsc::channel();
            self.state.busy_message = Some("处理中…".into());
            self.state.audio_job_rx = Some(rx);
            let sync = meta.audio_sync_offset_secs;
            let pps = meta.audio_pixels_per_second;
            let view = meta.audio_view_start_secs;
            thread::spawn(move || {
                let result = AudioTrack::from_pcm(samples, sr, song.as_ref(), "embedded.pcm").map(
                    |mut t| {
                        t.sync_offset_secs = sync;
                        t.pixels_per_second = pps;
                        t.view_start_secs = view;
                        t
                    },
                );
                let _ = tx.send(result);
            });
        } else {
            self.state.audio_track = None;
            if let Some(player) = &self.state.audio_player {
                player.clear_audio();
            }
        }
    }

    fn save_project(&mut self) {
        if self.state.song.is_none() {
            self.state.status_message = "没有可保存的乐谱".into();
            return;
        }
        if let Some(path) = self.state.project_path.clone() {
            self.save_bso_to(&path);
        } else {
            self.save_project_as();
        }
    }

    fn save_project_as(&mut self) {
        if self.state.song.is_none() {
            self.state.status_message = "没有可保存的乐谱".into();
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter("Bassoxide 工程", &["bso"]);
        if let Some(p) = &self.state.project_path {
            if let Some(parent) = p.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(name) = p.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy());
            }
        } else if let Some(fp) = &self.state.file_path {
            let pb = PathBuf::from(fp);
            if let Some(stem) = pb.file_stem() {
                dialog = dialog.set_file_name(format!("{}.bso", stem.to_string_lossy()));
            }
        }
        if let Some(path) = dialog.save_file() {
            let path = if path.extension().is_none() {
                path.with_extension("bso")
            } else {
                path
            };
            self.save_bso_to(&path);
        }
    }

    fn save_bso_to(&mut self, path: &Path) {
        match project::save_bso(path, &self.state) {
            Ok(()) => {
                self.state.project_path = Some(path.to_path_buf());
                self.state.file_path = Some(path.display().to_string());
                self.state.status_message = format!("已保存工程: {}", path.display());
            }
            Err(e) => {
                self.state.status_message = format!("保存失败: {e}");
            }
        }
    }

    fn open_marker_editor(&mut self) {
        if self.state.song.is_none() {
            return;
        }
        let m = self.state.playhead_measure_index().max(self.state.cursor.measure);
        self.state.cursor.measure = m;
        self.state.marker_edit_name = self
            .state
            .song
            .as_ref()
            .and_then(|s| s.master_bars.get(m))
            .and_then(|mb| mb.marker.as_ref())
            .map(|mk| mk.name.clone())
            .unwrap_or_default();
        self.state.marker_editor_open = true;
    }

    fn poll_audio_job(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.state.audio_job_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(track)) => {
                self.state.install_audio_track(track);
            }
            Ok(Err(e)) => {
                self.state.busy_message = None;
                self.state.status_message = format!("音频加载失败: {e}");
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.state.audio_job_rx = Some(rx);
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.state.busy_message = None;
                self.state.status_message = "音频任务中断".into();
            }
        }
    }

    fn draw_busy_overlay(&self, ctx: &egui::Context) {
        let Some(msg) = &self.state.busy_message else {
            return;
        };
        let palette = MaterialPalette::for_mode(self.state.is_light_theme);
        egui::Area::new(egui::Id::new("busy_overlay"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120))
                    .inner_margin(egui::Margin::symmetric(28, 20))
                    .corner_radius(8.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(msg)
                                .size(18.0)
                                .color(palette.on_primary),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("请稍候，正在解码/分析音频…")
                                .size(12.0)
                                .color(egui::Color32::from_gray(220)),
                        );
                    });
            });
        // 全屏半透明阻挡交互
        egui::Area::new(egui::Id::new("busy_blocker"))
            .order(egui::Order::Middle)
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let screen = ctx.screen_rect();
                ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(20, 20, 20, 40),
                );
            });
    }

    /// 谱面编辑快捷键（方向键、插删、效果等）
    fn handle_edit_keys(&mut self, ctx: &egui::Context) {
        use bassoxide_core::types::NoteValue;
        use crate::edit::{self, CursorMove};

        if self.state.song.is_none() {
            return;
        }

        let mods = ctx.input(|i| i.modifiers);
        let cmd = mods.command;

        // Ctrl+↑/↓ 改弦；普通 ↑/↓ 移光标弦
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if cmd {
                edit::change_note_string(&mut self.state, -1);
            } else {
                edit::move_cursor(&mut self.state, CursorMove::UpString);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if cmd {
                edit::change_note_string(&mut self.state, 1);
            } else {
                edit::move_cursor(&mut self.state, CursorMove::DownString);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            edit::move_cursor(&mut self.state, CursorMove::Left);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            edit::move_cursor(&mut self.state, CursorMove::Right);
        }

        // Insert / I
        if ctx.input(|i| i.key_pressed(egui::Key::Insert) || (i.key_pressed(egui::Key::I) && !cmd))
        {
            edit::insert_note(&mut self.state);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            edit::delete_note(&mut self.state);
        }

        // 附点：Period
        if ctx.input(|i| i.key_pressed(egui::Key::Period)) {
            edit::toggle_dotted(&mut self.state);
        }

        // 时值 Q/W/E/R
        if ctx.input(|i| i.key_pressed(egui::Key::Q) && !cmd) {
            edit::set_duration(&mut self.state, NoteValue::Whole);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::W) && !cmd) {
            edit::set_duration(&mut self.state, NoteValue::Half);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::E) && !cmd) {
            edit::set_duration(&mut self.state, NoteValue::Quarter);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R) && !cmd) {
            edit::set_duration(&mut self.state, NoteValue::Eighth);
        }

        // 效果
        if ctx.input(|i| i.key_pressed(egui::Key::H) && !cmd) {
            edit::toggle_hammer_on(&mut self.state);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::P) && !cmd) {
            edit::toggle_pull_off(&mut self.state);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S) && !cmd && !mods.shift) {
            edit::toggle_slide_up(&mut self.state);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S) && !cmd && mods.shift) {
            edit::toggle_slide_down(&mut self.state);
        }
        // T 无 Ctrl：延音（Ctrl+T 已用于调弦）
        if ctx.input(|i| i.key_pressed(egui::Key::T) && !cmd) {
            edit::toggle_tie(&mut self.state);
        }

        // 数字品格
        for (key, ch) in [
            (egui::Key::Num0, '0'),
            (egui::Key::Num1, '1'),
            (egui::Key::Num2, '2'),
            (egui::Key::Num3, '3'),
            (egui::Key::Num4, '4'),
            (egui::Key::Num5, '5'),
            (egui::Key::Num6, '6'),
            (egui::Key::Num7, '7'),
            (egui::Key::Num8, '8'),
            (egui::Key::Num9, '9'),
        ] {
            if ctx.input(|i| i.key_pressed(key) && !cmd) {
                if let Some(fret) = self.state.fret_input.push_digit(ch) {
                    edit::set_fret(&mut self.state, fret);
                }
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

        self.poll_audio_job(ctx);

        if let Some(path) = self.state.pending_audio_path.take() {
            self.load_audio_path(&path);
        }

        let busy = self.state.busy_message.is_some();

        // 键盘快捷键
        if !busy && ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command && i.modifiers.shift)
        {
            self.open_project_dialog();
        } else if !busy && ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command) {
            self.open_file();
        }
        if !busy
            && ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift)
        {
            self.save_project_as();
        } else if !busy && ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            self.save_project();
        }
        if !ctx.wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::T) && i.modifiers.command)
        {
            self.state.tuning_editor_open = !self.state.tuning_editor_open;
            if self.state.tuning_editor_open {
                self.state.status_message = "打开六线谱调弦".into();
            }
        }
        // 空格：播放 / 暂停（有音频轨且未在输入框中时）
        if !busy
            && !ctx.wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Space))
            && self.state.audio_track.is_some()
        {
            self.state.sync_playback_tools_to_player();
            if let Some(player) = &self.state.audio_player {
                player.toggle_play_pause();
            }
        }

        // 播放工具快捷键：M 节拍器、L 循环、[ ] 设 A/B
        if !busy
            && !ctx.wants_keyboard_input()
            && !self.state.tuning_editor_open
            && !self.state.settings_open
            && !self.state.marker_editor_open
        {
            if ctx.input(|i| i.key_pressed(egui::Key::M) && !i.modifiers.command) {
                self.state.toggle_metronome();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::L) && !i.modifiers.command) {
                self.state.toggle_loop_enabled();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::OpenBracket) && !i.modifiers.command) {
                self.state.set_loop_a_here();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::CloseBracket) && !i.modifiers.command) {
                self.state.set_loop_b_here();
            }
        }

        // 谱面编辑快捷键
        if !busy
            && !ctx.wants_keyboard_input()
            && !self.state.tuning_editor_open
            && !self.state.settings_open
            && !self.state.marker_editor_open
        {
            self.handle_edit_keys(ctx);
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
                if busy {
                    return;
                }
                match action {
                    menu_bar::MenuAction::OpenFile => self.open_file(),
                    menu_bar::MenuAction::OpenProject => self.open_project_dialog(),
                    menu_bar::MenuAction::SaveProject => self.save_project(),
                    menu_bar::MenuAction::SaveProjectAs => self.save_project_as(),
                    menu_bar::MenuAction::AddAudioTrack => self.add_audio_track(),
                    menu_bar::MenuAction::EditMarker => self.open_marker_editor(),
                    menu_bar::MenuAction::Quit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    menu_bar::MenuAction::LightTheme => self.state.set_light_theme(true),
                    menu_bar::MenuAction::DarkTheme => self.state.set_light_theme(false),
                    menu_bar::MenuAction::OpenSettings => self.state.settings_open = true,
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

        // 底部整体：音频同步轨 + 轨道（贴边、无圆角；固定高度避免鼠标误触拉伸上移）
        egui::TopBottomPanel::bottom("bottom_dock_fixed")
            .resizable(false)
            .exact_height(360.0)
            .show_separator_line(true)
            .frame(
                egui::Frame::NONE
                    .fill(palette.surface_container)
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 10,
                        top: 12,
                        bottom: 6,
                    })
                    .outer_margin(egui::Margin::ZERO)
                    .corner_radius(egui::CornerRadius::ZERO)
                    .shadow(egui::epaint::Shadow::NONE)
                    .stroke(egui::Stroke::NONE),
            )
            .show(ctx, |ui| {
                // 与谱面区留白；内容按自然高度堆叠，避免比例分配随 hover 抖动
                ui.add_space(6.0);
                crate::ui::audio_track::audio_track_panel(ui, &mut self.state);
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);
                crate::ui::timeline::timeline_panel(ui, &mut self.state);
            });

        // 播放：有声卡时由音频回调推进时间；无声卡时用帧 dt 推进。始终请求重绘以刷新播放头。
        if let Some(player) = &self.state.audio_player {
            if player.status() == bassoxide_audio::PlaybackStatus::Playing {
                let dt = ctx.input(|i| i.stable_dt) as f64;
                player.tick(dt);
                ctx.request_repaint();
            }
        }

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

        // 设置页面
        if self.state.settings_open {
            if crate::ui::settings::settings_window(ctx, &mut self.state) {
                self.state.settings_open = false;
            }
        }

        // 六线谱调弦配置
        crate::ui::toolbar::tuning_editor_window(ctx, &mut self.state);

        // 小节标记编辑
        crate::ui::toolbar::marker_editor_window(ctx, &mut self.state);

        self.draw_busy_overlay(ctx);
    }
}
