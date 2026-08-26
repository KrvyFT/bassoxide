//! Song 顶层数据模型 — 整个乐谱文件的根容器。

use serde::{Deserialize, Serialize};

use crate::lyrics::Lyrics;
use crate::measure::MasterBar;
use crate::midi::MidiChannel;
use crate::track::Track;

/// 乐谱文件元信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SongInfo {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub album: String,
    /// 作词者
    pub words: String,
    /// 作曲者
    pub music: String,
    pub copyright: String,
    /// 制谱者
    pub tab_author: String,
    /// 说明/备注
    pub instructions: String,
    /// 多行注释
    pub comments: Vec<String>,
}

/// 页面设置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageSetup {
    /// 页面宽度 (mm)
    pub page_width: f32,
    /// 页面高度 (mm)
    pub page_height: f32,
    /// 上边距 (mm)
    pub margin_top: f32,
    /// 下边距 (mm)
    pub margin_bottom: f32,
    /// 左边距 (mm)
    pub margin_left: f32,
    /// 右边距 (mm)
    pub margin_right: f32,
    /// 谱表间距
    pub score_size: f32,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            page_width: 210.0,
            page_height: 297.0,
            margin_top: 10.0,
            margin_bottom: 15.0,
            margin_left: 10.0,
            margin_right: 10.0,
            score_size: 1.0,
        }
    }
}

/// 乐谱顶层结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// 文件格式版本 (如 "FICHIER GUITAR PRO v5.10")
    pub version: String,
    /// 元信息
    pub info: SongInfo,
    /// 初始速度 (BPM)
    pub tempo: u16,
    /// 全局小节信息
    pub master_bars: Vec<MasterBar>,
    /// 各乐器轨道
    pub tracks: Vec<Track>,
    /// MIDI 通道配置
    pub midi_channels: Vec<MidiChannel>,
    /// 歌词
    pub lyrics: Lyrics,
    /// 页面设置
    pub page_setup: PageSetup,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            version: String::new(),
            info: SongInfo::default(),
            tempo: 120,
            master_bars: Vec::new(),
            tracks: Vec::new(),
            midi_channels: Vec::new(),
            lyrics: Lyrics::default(),
            page_setup: PageSetup::default(),
        }
    }
}

impl Song {
    /// 小节数
    pub fn measure_count(&self) -> usize {
        self.master_bars.len()
    }

    /// 轨道数
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// 获取指定小节的全局信息
    pub fn master_bar(&self, index: usize) -> Option<&MasterBar> {
        self.master_bars.get(index)
    }

    /// 获取指定轨道
    pub fn track(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    /// 根据轨道名称自动配置所有轨道的乐器类型和音色
    pub fn auto_configure_instruments(&mut self) {
        for track in &mut self.tracks {
            track.auto_configure_instrument();
        }
    }
}
