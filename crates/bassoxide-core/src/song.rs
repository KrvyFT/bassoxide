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

    /// 打开文件后：用文件内 MIDI 通道表（若有）分配 GM 音色；否则只同步乐器种类。
    pub fn apply_file_instruments(&mut self) {
        let channels = self.midi_channels.clone();
        for track in &mut self.tracks {
            if !channels.is_empty() {
                let idx = crate::midi::MidiChannel::table_index(track.midi_port, track.midi_channel);
                if let Some(ch) = channels.get(idx) {
                    track.apply_midi_channel(ch);
                    continue;
                }
            }
            track.sync_instrument_type();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::MidiChannel;
    use crate::track::Track;
    use crate::types::InstrumentType;

    #[test]
    fn apply_file_instruments_uses_channel_table_not_track_name() {
        let mut song = Song::default();
        song.midi_channels = vec![MidiChannel {
            channel: 0,
            instrument: 30,
            volume: 96,
            ..MidiChannel::default()
        }];
        song.tracks.push(Track {
            name: "Bass".to_string(),
            midi_port: 1,
            midi_channel: 0,
            midi_program: 25,
            ..Track::default()
        });
        song.apply_file_instruments();
        assert_eq!(song.tracks[0].midi_program, 30);
        assert_eq!(song.tracks[0].instrument_type, InstrumentType::ElectricGuitar);
        assert_eq!(song.tracks[0].volume, 96);
    }
}
