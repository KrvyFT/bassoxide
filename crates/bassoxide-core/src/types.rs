//! 公共类型定义：枚举、常量和基础结构体。
//!
//! 这些类型被 `bassoxide-core` 的所有模块共享，
//! 也是上层 crate（io, layout, render）的基础依赖。

use serde::{Deserialize, Serialize};

// ── 音高 ──

/// MIDI 音高值 (0–127)
pub type MidiNote = u8;
/// MIDI 力度 (0–127)
pub type Velocity = u8;
/// 品格编号 (0 = 空弦, -1 = 死音)
pub type Fret = i8;
/// 弦编号 (1-based, 1 = 最高音弦)
pub type StringNumber = u8;

/// 音名
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PitchClass {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

/// 变音记号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Accidental {
    DoubleFlat,
    Flat,
    #[default]
    Natural,
    Sharp,
    DoubleSharp,
}

/// 谱号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Clef {
    #[default]
    Treble,
    Bass,
    Alto,
    Tenor,
    /// 六线谱专用标记
    Tab,
}

// ── 时值 ──

/// 音符时值基础值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
}

impl NoteValue {
    /// 返回以全音符为 1.0 的相对时值
    pub fn relative_duration(self) -> f64 {
        match self {
            Self::Whole => 1.0,
            Self::Half => 0.5,
            Self::Quarter => 0.25,
            Self::Eighth => 0.125,
            Self::Sixteenth => 0.0625,
            Self::ThirtySecond => 0.03125,
            Self::SixtyFourth => 0.015625,
        }
    }

    /// 从 GP5 格式的编码值 (-2..4) 解析
    pub fn from_gp_value(value: i8) -> Option<Self> {
        match value {
            -2 => Some(Self::Whole),
            -1 => Some(Self::Half),
            0 => Some(Self::Quarter),
            1 => Some(Self::Eighth),
            2 => Some(Self::Sixteenth),
            3 => Some(Self::ThirtySecond),
            4 => Some(Self::SixtyFourth),
            _ => None,
        }
    }

    /// 转为 GP5 格式的编码值
    pub fn to_gp_value(self) -> i8 {
        match self {
            Self::Whole => -2,
            Self::Half => -1,
            Self::Quarter => 0,
            Self::Eighth => 1,
            Self::Sixteenth => 2,
            Self::ThirtySecond => 3,
            Self::SixtyFourth => 4,
        }
    }
}

/// 音符时值（含附点和连音）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Duration {
    pub value: NoteValue,
    pub dotted: bool,
    pub double_dotted: bool,
    /// 连音符：n 连音（如三连音 = 3）
    pub tuplet_numerator: u8,
    pub tuplet_denominator: u8,
}

impl Default for Duration {
    fn default() -> Self {
        Self {
            value: NoteValue::Quarter,
            dotted: false,
            double_dotted: false,
            tuplet_numerator: 1,
            tuplet_denominator: 1,
        }
    }
}

impl Duration {
    /// 计算实际 tick 数（基于 960 ticks/quarter）
    pub fn ticks(&self) -> u32 {
        let base = match self.value {
            NoteValue::Whole => 3840,
            NoteValue::Half => 1920,
            NoteValue::Quarter => 960,
            NoteValue::Eighth => 480,
            NoteValue::Sixteenth => 240,
            NoteValue::ThirtySecond => 120,
            NoteValue::SixtyFourth => 60,
        };

        let dotted = if self.double_dotted {
            base + base / 2 + base / 4
        } else if self.dotted {
            base + base / 2
        } else {
            base
        };

        if self.tuplet_denominator > 0 && self.tuplet_numerator > 0 {
            dotted * u32::from(self.tuplet_denominator) / u32::from(self.tuplet_numerator)
        } else {
            dotted
        }
    }
}

// ── 拍号 & 调号 ──

/// 拍号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: NoteValue,
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: NoteValue::Quarter,
        }
    }
}

impl TimeSignature {
    /// 一小节的总 tick 数
    pub fn measure_ticks(&self) -> u32 {
        let beat_ticks = Duration {
            value: self.denominator,
            ..Default::default()
        }
        .ticks();
        beat_ticks * u32::from(self.numerator)
    }
}

/// 调号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeySignature {
    /// 正数 = 升号数, 负数 = 降号数 (-7..7)
    pub key: i8,
    pub is_minor: bool,
}

// ── 力度 ──

/// 力度标记（ppp → fff）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum DynamicValue {
    PPP,
    PP,
    P,
    MP,
    #[default]
    MF,
    F,
    FF,
    FFF,
}

impl DynamicValue {
    /// 转为 MIDI velocity
    pub fn velocity(self) -> Velocity {
        match self {
            Self::PPP => 15,
            Self::PP => 31,
            Self::P => 47,
            Self::MP => 63,
            Self::MF => 79,
            Self::F => 95,
            Self::FF => 111,
            Self::FFF => 127,
        }
    }
}

// ── 乐器 ──

/// 乐器种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InstrumentType {
    #[default]
    AcousticGuitar,
    ElectricGuitar,
    ClassicalGuitar,
    Bass,
    Drums,
    Piano,
    Strings,
    Voice,
    Other,
}

/// 反复 / 排练标记方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatType {
    /// 反复开始
    Open,
    /// 反复结束（含次数）
    Close(u8),
    /// 反复开始+结束
    OpenClose(u8),
}

/// 小节线类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BarLineType {
    #[default]
    Normal,
    Double,
    Final,
    RepeatOpen,
    RepeatClose,
    RepeatBoth,
}

/// 渐强/渐弱标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hairpin {
    Crescendo,
    Decrescendo,
}

/// 颜色 (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// 页面方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// 节奏型斜线标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashType {
    Normal,
    /// 带节奏的斜线
    Rhythmic,
}

/// 符干方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StemDirection {
    Up,
    Down,
    Auto,
}

/// Tuplet 连音比例常用值
impl Duration {
    pub fn triplet(value: NoteValue) -> Self {
        Self {
            value,
            dotted: false,
            double_dotted: false,
            tuplet_numerator: 3,
            tuplet_denominator: 2,
        }
    }
}
