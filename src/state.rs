//! 应用状态管理。

use bassoxide_core::song::Song;
use bassoxide_layout::engine::{LayoutEngine, LayoutResult};
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_render::Theme;

use crate::ui::material::MaterialPalette;

/// 编辑器光标位置
#[derive(Debug, Clone, Default)]
pub struct CursorPosition {
    pub track: usize,
    pub measure: usize,
    pub beat: usize,
    pub string: usize,
}

/// 全局应用状态
pub struct AppState {
    /// 当前加载的乐谱
    pub song: Option<Song>,
    /// 布局结果（排版缓存）
    pub layout: Option<LayoutResult>,
    /// 布局设置
    pub layout_settings: LayoutSettings,
    /// 渲染主题
    pub theme: Theme,
    /// 当前光标位置
    pub cursor: CursorPosition,
    /// 乐谱纵向滚动偏移
    pub scroll_y: f32,
    /// 是否需要重新排版
    pub needs_relayout: bool,
    /// 文件路径
    pub file_path: Option<String>,
    /// 状态栏消息
    pub status_message: String,
    /// 音频引擎
    pub audio_engine: Option<bassoxide_audio::AudioEngine>,
    /// 视图缩放系数
    pub zoom_factor: f32,
    /// 当前显示的轨道索引（单轨道显示）
    pub selected_track: usize,
    /// 是否浅色主题（Material You 默认浅色）
    pub is_light_theme: bool,
    /// 打开的轨道配置弹窗（轨道索引）
    pub track_config_popup: Option<usize>,
    /// 主题是否已应用到 egui
    pub theme_dirty: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let audio_engine = match bassoxide_audio::AudioEngine::new() {
            Ok(engine) => Some(engine),
            Err(e) => {
                tracing::error!("Failed to initialize audio engine: {}", e);
                None
            }
        };

        let is_light_theme = true;
        let palette = MaterialPalette::for_mode(is_light_theme);

        Self {
            song: None,
            layout: None,
            layout_settings: LayoutSettings::default(),
            theme: palette.to_score_theme(),
            cursor: CursorPosition::default(),
            scroll_y: 0.0,
            needs_relayout: false,
            file_path: None,
            status_message: "就绪".to_string(),
            audio_engine,
            zoom_factor: 1.0,
            selected_track: 0,
            is_light_theme,
            track_config_popup: None,
            theme_dirty: true,
        }
    }
}

impl AppState {
    pub fn update_zoom(&mut self) {
        // 重置为基础值并乘以上缩放系数
        let base = bassoxide_layout::spacing::LayoutSettings::default();
        let z = self.zoom_factor;
        self.layout_settings.margin_top = base.margin_top * z;
        self.layout_settings.margin_left = base.margin_left * z;
        self.layout_settings.system_gap = base.system_gap * z;
        self.layout_settings.track_gap = base.track_gap * z;
        self.layout_settings.min_measure_width = base.min_measure_width * z;
        self.layout_settings.min_beat_spacing = base.min_beat_spacing * z;
        self.layout_settings.tab_string_spacing = base.tab_string_spacing * z;
        self.layout_settings.tab_font_size = base.tab_font_size * z;
        self.layout_settings.clef_width = base.clef_width * z;
        self.layout_settings.time_sig_width = base.time_sig_width * z;
        self.layout_settings.page_width = base.page_width * z;
        self.layout_settings.page_height = base.page_height * z;
        self.layout_settings.page_margin = base.page_margin * z;
        self.layout_settings.rhythm_height = base.rhythm_height * z;
    }

    /// 切换浅色/深色主题
    pub fn set_light_theme(&mut self, light: bool) {
        if self.is_light_theme != light {
            self.is_light_theme = light;
            let palette = MaterialPalette::for_mode(light);
            self.theme = palette.to_score_theme();
            self.theme_dirty = true;
        }
    }

    /// 切换显示的轨道
    pub fn select_track(&mut self, index: usize) {
        if index != self.selected_track {
            self.selected_track = index;
            self.needs_relayout = true;
        }
    }

    /// 加载乐谱并触发排版
    pub fn load_song(&mut self, song: Song, path: Option<String>) {
        let track_count = song.track_count();
        let measure_count = song.measure_count();
        self.file_path = path;
        self.selected_track = 0;
        self.track_config_popup = None;
        self.status_message = format!(
            "已加载: {} | {} 轨道, {} 小节, {} BPM",
            song.info.title.as_str(),
            track_count,
            measure_count,
            song.tempo,
        );
        if let Some(audio) = &self.audio_engine {
            audio.stop();
            audio.reload_song(&song);
        }
        self.song = Some(song);
        self.needs_relayout = true;
        self.cursor = CursorPosition::default();
        self.scroll_y = 0.0;
    }

    /// 执行排版（仅在需要时调用）
    pub fn relayout(&mut self) {
        if let Some(song) = &self.song {
            let selected = self
                .selected_track
                .min(song.track_count().saturating_sub(1));
            let engine =
                LayoutEngine::new(self.layout_settings.clone()).with_selected_track(selected);
            self.layout = Some(engine.layout(song));
            self.needs_relayout = false;
        }
    }

    /// 更新可用宽度（窗口大小变化时）
    pub fn update_available_width(&mut self, width: f32) {
        if (self.layout_settings.available_width - width).abs() > 1.0 {
            self.layout_settings.available_width = width;
            self.needs_relayout = true;
        }
    }
}
