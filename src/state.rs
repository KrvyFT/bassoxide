//! 应用状态管理。

use std::sync::Arc;

use bassoxide_core::song::Song;
use bassoxide_layout::engine::{LayoutEngine, LayoutResult};
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_layout::PaperSize;
use bassoxide_render::Theme;

use crate::ui::audio_track::AudioTrack;
use crate::ui::material::MaterialPalette;

/// 编辑器光标位置
#[derive(Debug, Clone, Default)]
pub struct CursorPosition {
    pub track: usize,
    pub measure: usize,
    pub beat: usize,
    pub string: usize,
}

/// 用户可调的谱面偏好（缩放前的基准值，以 A4 为参考）
#[derive(Debug, Clone)]
pub struct ScorePrefs {
    pub font_size: f32,
    pub line_spacing: f32,
    pub row_spacing: f32,
    /// 0 = 自动
    pub measures_per_line: u8,
    pub paper_size: PaperSize,
}

impl Default for ScorePrefs {
    fn default() -> Self {
        Self {
            font_size: 13.0,
            line_spacing: 10.0,
            // 无重叠最小间距为 0（system 高度已含内容）+ 10px
            row_spacing: 10.0,
            measures_per_line: 4,
            paper_size: PaperSize::A4,
        }
    }
}

/// 全局应用状态
pub struct AppState {
    /// 当前加载的乐谱
    pub song: Option<Song>,
    /// 布局结果（排版缓存）
    pub layout: Option<LayoutResult>,
    /// 布局设置（含缩放后的实际值）
    pub layout_settings: LayoutSettings,
    /// 谱面偏好（设置页编辑的基准值）
    pub score_prefs: ScorePrefs,
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
    /// PCM 音频播放器（外部音频轨，非 MIDI）
    pub audio_player: Option<bassoxide_audio::AudioPlayer>,
    /// 外部音频同步轨
    pub audio_track: Option<AudioTrack>,
    /// 视图缩放系数
    pub zoom_factor: f32,
    /// 当前显示的轨道索引（单轨道显示）
    pub selected_track: usize,
    /// 是否浅色主题（Material You 默认浅色）
    pub is_light_theme: bool,
    /// 设置页面是否打开
    pub settings_open: bool,
    /// 六线谱调弦配置窗口
    pub tuning_editor_open: bool,
    /// 主题是否已应用到 egui
    pub theme_dirty: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let audio_player = match bassoxide_audio::AudioPlayer::new() {
            Ok(player) => Some(player),
            Err(e) => {
                tracing::error!("Failed to initialize audio player: {}", e);
                None
            }
        };

        let is_light_theme = true;
        let palette = MaterialPalette::for_mode(is_light_theme);
        let score_prefs = ScorePrefs::default();
        let mut state = Self {
            song: None,
            layout: None,
            layout_settings: LayoutSettings::default(),
            score_prefs,
            theme: palette.to_score_theme(),
            cursor: CursorPosition::default(),
            scroll_y: 0.0,
            needs_relayout: false,
            file_path: None,
            status_message: "就绪".to_string(),
            audio_player,
            audio_track: None,
            zoom_factor: 1.0,
            selected_track: 0,
            is_light_theme,
            settings_open: false,
            tuning_editor_open: false,
            theme_dirty: true,
        };
        state.apply_score_prefs();
        state
    }
}

impl AppState {
    /// 将谱面偏好 × 纸张缩放 × 视图缩放 写入 layout_settings，
    /// 并按「音符∈谱表 > 谱表∈纸张」自动调节冲突项。
    pub fn apply_score_prefs(&mut self) {
        let z = self.zoom_factor;
        let paper = self.score_prefs.paper_size;
        let (page_w, page_h) = paper.size_px();
        let paper_s = paper.content_scale();
        let s = paper_s * z;
        let base = LayoutSettings::default();

        let p = &self.score_prefs;
        self.layout_settings.paper_size = paper;
        self.layout_settings.content_scale = paper_s;
        self.layout_settings.page_width = page_w * z;
        self.layout_settings.page_height = page_h * z;
        self.layout_settings.page_margin = (base.page_margin * s).max(24.0);

        self.layout_settings.tab_font_size = (p.font_size * s).max(7.0);
        self.layout_settings.tab_string_spacing = (p.line_spacing * s).max(5.0);
        self.layout_settings.staff_line_spacing = (p.line_spacing * s).max(5.0);
        self.layout_settings.system_gap = (p.row_spacing * s).max(0.0);
        self.layout_settings.measures_per_line = p.measures_per_line;
        let pack = if p.measures_per_line > 0 {
            (2.6 / f32::from(p.measures_per_line)).clamp(0.45, 1.0)
        } else {
            1.0
        };
        self.layout_settings.rhythm_height =
            (p.font_size * 1.85 * s * pack).clamp(10.0 * paper_s, 52.0 * paper_s.max(1.0));

        self.layout_settings.margin_top = base.margin_top * s;
        self.layout_settings.margin_left = base.margin_left * s;
        self.layout_settings.track_gap = base.track_gap * s;
        self.layout_settings.min_measure_width = (base.min_measure_width * s).max(40.0);
        self.layout_settings.min_beat_spacing = (base.min_beat_spacing * s).max(10.0);
        self.layout_settings.clef_width = (base.clef_width * s).max(16.0);
        self.layout_settings.time_sig_width = (base.time_sig_width * s).max(16.0);

        let fit_ctx = self.staff_fit_context();
        let fit = bassoxide_layout::resolve_fit(&mut self.layout_settings, fit_ctx);

        // 仅在自动调节后回写偏好，避免无意义浮点漂移
        if fit.adjusted && s > 0.0 {
            self.score_prefs.font_size =
                (self.layout_settings.tab_font_size / s).clamp(8.0, 28.0);
            self.score_prefs.line_spacing =
                (self.layout_settings.tab_string_spacing / s).clamp(8.0, 28.0);
            self.score_prefs.row_spacing =
                (self.layout_settings.system_gap / s).clamp(0.0, 200.0);
            if let Some(msg) = fit.summary() {
                self.status_message = format!("自动适配: {msg}");
            }
        }

        self.needs_relayout = true;
    }

    /// 当前选中轨道的谱表形态（约束求解用）
    fn staff_fit_context(&self) -> bassoxide_layout::StaffFitContext {
        let Some(song) = &self.song else {
            return bassoxide_layout::StaffFitContext::default();
        };
        let idx = self.selected_track.min(song.tracks.len().saturating_sub(1));
        let Some(track) = song.tracks.get(idx) else {
            return bassoxide_layout::StaffFitContext::default();
        };
        bassoxide_layout::StaffFitContext {
            show_standard: track.staff_display.show_standard,
            show_tab: track.staff_display.show_tab,
            tab_strings: track
                .tuning
                .string_count()
                .max(track.staff_display.tab_strings as usize)
                .clamp(1, 8) as u8,
        }
    }

    pub fn update_zoom(&mut self) {
        self.apply_score_prefs();
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
        self.status_message = format!(
            "已加载: {} | {} 轨道, {} 小节, {} BPM",
            song.info.title.as_str(),
            track_count,
            measure_count,
            song.display_tempo(),
        );
        // 换谱后按新拍号/速度重新分析已有音频的小节线
        if let Some(track) = self.audio_track.as_mut() {
            let bpb = bassoxide_audio::default_beats_per_bar(Some(&song));
            track.analysis = bassoxide_audio::analyze_beats(
                &track.samples,
                track.sample_rate,
                bpb,
                Some(f64::from(song.tempo)),
            );
        }
        if let Some(player) = &self.audio_player {
            player.stop();
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

    /// 定位播放头到谱面时间；`start_playback` 为真时若未在播放则开始播放
    pub fn seek_score_secs(&mut self, secs: f64, start_playback: bool) {
        let secs = secs.max(0.0);
        if let Some(player) = &self.audio_player {
            player.seek_score_secs(secs);
            if start_playback && player.status() != bassoxide_audio::PlaybackStatus::Playing {
                player.play();
            }
        }
        // 让音频轨视图尽量跟上播放头
        if let Some(track) = self.audio_track.as_mut() {
            let span = track.view_span_secs(800.0);
            if secs < track.view_start_secs || secs > track.view_start_secs + span * 0.9 {
                track.view_start_secs = (secs - span * 0.25).max(0.0);
            }
        }
        self.status_message = format!("定位 {:.2}s", secs);
    }

    /// 将当前音频轨同步到播放器
    pub fn sync_player_from_track(&self) {
        let Some(player) = &self.audio_player else {
            return;
        };
        if let Some(track) = &self.audio_track {
            player.set_audio(Arc::clone(&track.samples), track.sample_rate);
            player.set_sync_offset(track.sync_offset_secs);
        } else {
            player.clear_audio();
        }
    }
}
