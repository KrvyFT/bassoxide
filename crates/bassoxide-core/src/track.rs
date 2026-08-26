//! 轨道数据模型。
//!
//! `Track` 代表一个乐器轨道（如吉他、贝斯、鼓）。
//! 包含调弦信息、MIDI 通道配置和所有小节的音符数据。

use serde::{Deserialize, Serialize};

use crate::measure::Measure;
use crate::types::{Clef, Color, InstrumentType, MidiNote};

/// 吉他弦的调弦信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuitarString {
    /// 弦号 (1-based, 1 = 最高音弦)
    pub number: u8,
    /// 空弦 MIDI 音高
    pub tuning: MidiNote,
}

/// 预设调弦方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tuning {
    /// 调弦名称 (如 "Standard E", "Drop D")
    pub name: String,
    /// 各弦调弦值
    pub strings: Vec<GuitarString>,
}

impl Tuning {
    /// 标准吉他调弦 (E2-E4: MIDI 40,45,50,55,59,64)
    pub fn standard_guitar() -> Self {
        Self {
            name: "Standard E".to_string(),
            strings: vec![
                GuitarString { number: 1, tuning: 64 }, // E4
                GuitarString { number: 2, tuning: 59 }, // B3
                GuitarString { number: 3, tuning: 55 }, // G3
                GuitarString { number: 4, tuning: 50 }, // D3
                GuitarString { number: 5, tuning: 45 }, // A2
                GuitarString { number: 6, tuning: 40 }, // E2
            ],
        }
    }

    /// 标准贝斯调弦 (E1-G2: MIDI 28,33,38,43)
    pub fn standard_bass() -> Self {
        Self {
            name: "Standard Bass".to_string(),
            strings: vec![
                GuitarString { number: 1, tuning: 43 }, // G2
                GuitarString { number: 2, tuning: 38 }, // D2
                GuitarString { number: 3, tuning: 33 }, // A1
                GuitarString { number: 4, tuning: 28 }, // E1
            ],
        }
    }

    /// 弦数
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    /// 根据弦号和品格计算 MIDI 音高
    pub fn midi_note(&self, string: u8, fret: i8) -> Option<MidiNote> {
        self.strings
            .iter()
            .find(|s| s.number == string)
            .map(|s| (s.tuning as i16 + fret as i16).clamp(0, 127) as MidiNote)
    }
}

/// 乐器轨道
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// 轨道编号 (1-based)
    pub number: u8,
    /// 轨道名称
    pub name: String,
    /// 乐器种类
    pub instrument_type: InstrumentType,
    /// 弦/调弦信息
    pub tuning: Tuning,
    /// MIDI 通道 (0-based)
    pub midi_channel: u8,
    /// MIDI 端口 (0-based)
    pub midi_port: u8,
    /// MIDI 音色编号 (General MIDI program, 0-127)
    pub midi_program: u8,
    /// 变调夹位置 (0 = 无变调夹)
    pub capo: u8,
    /// 品格数
    pub fret_count: u8,
    /// 谱号
    pub clef: Clef,
    /// 显示颜色
    pub color: Color,
    /// 音量 (0–127)
    pub volume: u8,
    /// 声相 (0–127, 64 = 居中)
    pub pan: u8,
    /// 是否静音
    pub is_muted: bool,
    /// 是否 Solo
    pub is_solo: bool,
    /// 是否为鼓轨道
    pub is_percussion: bool,
    /// 各小节数据
    pub measures: Vec<Measure>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            number: 1,
            name: "Track 1".to_string(),
            instrument_type: InstrumentType::AcousticGuitar,
            tuning: Tuning::standard_guitar(),
            midi_channel: 0,
            midi_port: 0,
            midi_program: 25, // Steel Guitar (GM)
            capo: 0,
            fret_count: 24,
            clef: Clef::Treble,
            color: Color::rgb(255, 0, 0),
            volume: 100,
            pan: 64,
            is_muted: false,
            is_solo: false,
            is_percussion: false,
            measures: Vec::new(),
        }
    }
}

impl Track {
    /// 弦数
    pub fn string_count(&self) -> usize {
        self.tuning.string_count()
    }
}
