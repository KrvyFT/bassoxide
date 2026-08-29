//! 应用状态管理。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use bassoxide_core::song::Song;
use bassoxide_core::types::NoteValue;
use bassoxide_layout::engine::{LayoutEngine, LayoutResult};
use bassoxide_layout::spacing::LayoutSettings;
use bassoxide_layout::PaperSize;
use bassoxide_render::Theme;

use crate::ui::audio_track::AudioTrack;
use crate::ui::material::MaterialPalette;

/// 后台音频解码任务结果
pub type AudioJobReceiver = Receiver<Result<AudioTrack, String>>;

/// 编辑器光标位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorPosition {
    pub track: usize,
    pub measure: usize,
    pub beat: usize,
    /// 弦号 (1-based，与 Note.string 一致)
    pub string: u8,
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self {
            track: 0,
            measure: 0,
            beat: 0,
            string: 1,
        }
    }
}

/// 多选音符格（当前轨道）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteRef {
    pub measure: usize,
    pub beat: usize,
    pub string: u8,
}

impl From<CursorPosition> for NoteRef {
    fn from(c: CursorPosition) -> Self {
        Self {
            measure: c.measure,
            beat: c.beat,
            string: c.string,
        }
    }
}

/// 左侧编辑工具类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditToolKind {
    #[default]
    Note,
    Rest,
    Marker,
}

/// 当前选用的谱面输入工具（时值 + 类别）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTool {
    pub kind: EditToolKind,
    pub duration: NoteValue,
    pub dotted: bool,
}

impl Default for EditTool {
    fn default() -> Self {
        Self {
            kind: EditToolKind::Note,
            duration: NoteValue::Quarter,
            dotted: false,
        }
    }
}

impl EditTool {
    pub fn slot_duration(self) -> bassoxide_core::types::Duration {
        bassoxide_core::types::Duration {
            value: self.duration,
            dotted: self.dotted,
            double_dotted: false,
            tuplet_numerator: 1,
            tuplet_denominator: 1,
        }
    }
}

/// 谱面选区：多音符和/或整小节
#[derive(Debug, Clone, Default)]
pub struct ScoreSelection {
    pub notes: HashSet<NoteRef>,
    /// 整小节选中（高亮该小节全部内容）
    pub measure: Option<usize>,
}

impl ScoreSelection {
    pub fn clear(&mut self) {
        self.notes.clear();
        self.measure = None;
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.measure.is_none()
    }

    pub fn select_single(&mut self, c: CursorPosition) {
        self.clear();
        self.notes.insert(NoteRef::from(c));
    }

    pub fn contains_note(&self, measure: usize, beat: usize, string: u8) -> bool {
        if self.measure == Some(measure) {
            return true;
        }
        self.notes.contains(&NoteRef {
            measure,
            beat,
            string,
        })
    }
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
    /// 文件路径（乐谱或工程显示用）
    pub file_path: Option<String>,
    /// 当前 `.bso` 工程路径（有则 Ctrl+S 直接覆盖保存）
    pub project_path: Option<PathBuf>,
    /// 状态栏消息
    pub status_message: String,
    /// 居中遮罩：「处理中…」
    pub busy_message: Option<String>,
    /// 后台音频解码接收端
    pub audio_job_rx: Option<AudioJobReceiver>,
    /// 音频轨面板请求加载的路径（由 update 转异步任务）
    pub pending_audio_path: Option<PathBuf>,
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
    /// 排练标记编辑窗口
    pub marker_editor_open: bool,
    /// 标记编辑缓冲（当前光标小节）
    pub marker_edit_name: String,
    /// 主题是否已应用到 egui
    pub theme_dirty: bool,
    /// 练习变速（0.5–1.5）
    pub playback_rate: f32,
    /// A-B 循环点（谱面秒）
    pub loop_a: Option<f64>,
    pub loop_b: Option<f64>,
    pub loop_enabled: bool,
    /// 节拍器开关
    pub metronome_enabled: bool,
    /// 品格数字输入缓冲
    pub fret_input: crate::edit::FretInputBuffer,
    /// 多选 / 小节选区
    pub selection: ScoreSelection,
    /// 拖选起点（屏幕坐标）；拖动中保留
    pub drag_select_origin: Option<egui::Pos2>,
    /// 拖选起点光标格
    pub drag_select_anchor: Option<CursorPosition>,
    /// 左侧工具栏当前工具（音符 / 休止符 / 标记）
    pub edit_tool: EditTool,
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
            project_path: None,
            status_message: "就绪".to_string(),
            busy_message: None,
            audio_job_rx: None,
            pending_audio_path: None,
            audio_player,
            audio_track: None,
            zoom_factor: 1.0,
            selected_track: 0,
            is_light_theme,
            settings_open: false,
            tuning_editor_open: false,
            marker_editor_open: false,
            marker_edit_name: String::new(),
            theme_dirty: true,
            playback_rate: 1.0,
            loop_a: None,
            loop_b: None,
            loop_enabled: false,
            metronome_enabled: false,
            fret_input: crate::edit::FretInputBuffer::default(),
            selection: ScoreSelection::default(),
            drag_select_origin: None,
            drag_select_anchor: None,
            edit_tool: EditTool::default(),
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
        // 符杆区：与每行小节数弱相关，避免挤窄时缩成短 stubs
        let rhythm_pack = if p.measures_per_line > 0 {
            (3.2 / f32::from(p.measures_per_line)).clamp(0.7, 1.0)
        } else {
            1.0
        };
        self.layout_settings.rhythm_height =
            (p.font_size * 3.2 * s * rhythm_pack).clamp(24.0 * paper_s, 80.0 * paper_s.max(1.0));

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
            self.cursor.track = index;
            self.cursor.measure = 0;
            self.cursor.beat = 0;
            self.cursor.string = 1;
            self.needs_relayout = true;
        }
    }

    /// 加载乐谱并触发排版
    pub fn load_song(&mut self, song: Song, path: Option<String>) {
        let track_count = song.track_count();
        let measure_count = song.measure_count();
        self.file_path = path;
        // 非 .bso 打开时清空工程路径（.bso 由 apply_meta_and_song 单独设置）
        if self
            .project_path
            .as_ref()
            .map(|p| p.display().to_string())
            != self.file_path
        {
            // 仅当 file_path 不是当前 project 时保持；GP 打开会换路径
            if self
                .file_path
                .as_ref()
                .map(|p| !p.ends_with(".bso"))
                .unwrap_or(true)
            {
                self.project_path = None;
            }
        }
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
        self.selection.clear();
        self.scroll_y = 0.0;
        self.sync_playback_tools_to_player();
    }

    /// 安装已解码的音频轨并同步播放器
    pub fn install_audio_track(&mut self, mut track: AudioTrack) {
        if let Some(player) = &self.audio_player {
            player.set_audio(track.samples.clone(), track.sample_rate);
            player.set_sync_offset(track.sync_offset_secs);
        }
        self.status_message = format!(
            "已加载音频: {} | {:.1}s | 检测 {:.1} BPM | {} 小节线",
            track.path,
            track.duration_secs,
            track.analysis.bpm,
            track.analysis.measure_times.len()
        );
        // 保留视图偏移若刚从工程恢复（调用方先写入字段）
        let _ = &mut track;
        self.audio_track = Some(track);
        self.busy_message = None;
        self.sync_playback_tools_to_player();
    }

    /// 当前播放头所在小节索引
    pub fn playhead_measure_index(&self) -> usize {
        let secs = self
            .audio_player
            .as_ref()
            .map(|p| p.score_position_secs())
            .unwrap_or(0.0);
        let Some(song) = &self.song else {
            return self.cursor.measure;
        };
        let tl = bassoxide_audio::score_timeline(song);
        let (idx, _) = bassoxide_audio::measure_at_score_secs(&tl, secs);
        idx
    }

    /// 跳转到指定排练标记名
    pub fn jump_to_marker_name(&mut self, name: &str) -> bool {
        let Some(song) = &self.song else {
            return false;
        };
        let idx = song.master_bars.iter().position(|mb| {
            mb.marker
                .as_ref()
                .map(|m| m.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        });
        let Some(idx) = idx else {
            self.status_message = format!("未找到标记「{name}」");
            return false;
        };
        let tl = bassoxide_audio::score_timeline(song);
        let secs = tl.measure_times.get(idx).copied().unwrap_or(0.0);
        self.cursor.measure = idx;
        self.seek_score_secs(secs, false);
        self.status_message = format!("跳转到标记「{name}」（小节 {}）", idx + 1);
        true
    }

    /// 跳转到首个匹配的段落方向标记所在小节
    pub fn jump_to_direction(&mut self, dir: bassoxide_core::Direction) -> bool {
        let Some(song) = &self.song else {
            return false;
        };
        let idx = song
            .master_bars
            .iter()
            .position(|mb| mb.directions.contains(&dir));
        let Some(idx) = idx else {
            self.status_message = format!("未找到方向标记 {dir:?}");
            return false;
        };
        let tl = bassoxide_audio::score_timeline(song);
        let secs = tl.measure_times.get(idx).copied().unwrap_or(0.0);
        self.cursor.measure = idx;
        self.seek_score_secs(secs, false);
        true
    }

    /// 把变速 / 循环 / 节拍器调度同步到 AudioPlayer
    pub fn sync_playback_tools_to_player(&self) {
        let Some(player) = &self.audio_player else {
            return;
        };
        player.set_playback_rate(f64::from(self.playback_rate));
        player.set_metronome(self.metronome_enabled);
        let (a, b, en) = match (self.loop_a, self.loop_b) {
            (Some(a), Some(b)) if a < b => (a, b, self.loop_enabled),
            (Some(a), Some(b)) if b < a => (b, a, self.loop_enabled),
            _ => (0.0, 0.0, false),
        };
        player.set_loop(a, b, en);
        if let Some(song) = &self.song {
            let tl = bassoxide_audio::score_timeline(song);
            player.set_metronome_schedule(tl.beat_times, tl.measure_times);
        } else {
            player.set_metronome_schedule(Vec::new(), Vec::new());
        }
    }

    pub fn set_loop_a_here(&mut self) {
        let t = self
            .audio_player
            .as_ref()
            .map(|p| p.score_position_secs())
            .unwrap_or(0.0);
        self.loop_a = Some(t);
        self.status_message = format!("循环 A = {t:.2}s");
        self.sync_playback_tools_to_player();
    }

    pub fn set_loop_b_here(&mut self) {
        let t = self
            .audio_player
            .as_ref()
            .map(|p| p.score_position_secs())
            .unwrap_or(0.0);
        self.loop_b = Some(t);
        self.status_message = format!("循环 B = {t:.2}s");
        self.sync_playback_tools_to_player();
    }

    pub fn clear_loop_points(&mut self) {
        self.loop_a = None;
        self.loop_b = None;
        self.loop_enabled = false;
        self.status_message = "已清除 A-B 循环".into();
        self.sync_playback_tools_to_player();
    }

    pub fn toggle_loop_enabled(&mut self) {
        if self.loop_a.is_none() || self.loop_b.is_none() {
            self.status_message = "请先设置 A / B 点".into();
            return;
        }
        self.loop_enabled = !self.loop_enabled;
        self.status_message = if self.loop_enabled {
            "A-B 循环：开".into()
        } else {
            "A-B 循环：关".into()
        };
        self.sync_playback_tools_to_player();
    }

    pub fn toggle_metronome(&mut self) {
        self.metronome_enabled = !self.metronome_enabled;
        self.status_message = if self.metronome_enabled {
            "节拍器：开".into()
        } else {
            "节拍器：关".into()
        };
        self.sync_playback_tools_to_player();
    }

    pub fn set_playback_rate_ui(&mut self, rate: f32) {
        self.playback_rate = rate.clamp(0.5, 1.5);
        self.sync_playback_tools_to_player();
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
            self.sync_playback_tools_to_player();
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
