//! 应用状态管理。

use bassoxide_core::song::Song;
use bassoxide_layout::engine::{LayoutEngine, LayoutResult};
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_render::Theme;

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            song: None,
            layout: None,
            layout_settings: LayoutSettings::default(),
            theme: Theme::dark(),
            cursor: CursorPosition::default(),
            scroll_y: 0.0,
            needs_relayout: false,
            file_path: None,
            status_message: "就绪".to_string(),
        }
    }
}

impl AppState {
    /// 加载乐谱并触发排版
    pub fn load_song(&mut self, song: Song, path: Option<String>) {
        let track_count = song.track_count();
        let measure_count = song.measure_count();
        self.file_path = path;
        self.status_message = format!(
            "已加载: {} | {} 轨道, {} 小节, {} BPM",
            song.info.title.as_str(),
            track_count,
            measure_count,
            song.tempo,
        );
        self.song = Some(song);
        self.needs_relayout = true;
        self.cursor = CursorPosition::default();
        self.scroll_y = 0.0;
    }

    /// 执行排版（仅在需要时调用）
    pub fn relayout(&mut self) {
        if let Some(song) = &self.song {
            let engine = LayoutEngine::new(self.layout_settings.clone());
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
