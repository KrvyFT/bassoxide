//! Beat（拍）和 Voice（声部）数据模型。
//!
//! 一个 `Beat` 包含多个同时发声的 `Note`（构成和弦），
//! 并持有自身的时值和拍级效果。
//! 一个 `Voice` 是 `Beat` 的有序序列，每小节最多有 4 个 Voice。

use serde::{Deserialize, Serialize};

use crate::chord::ChordDiagram;
use crate::effects::BeatEffect;
use crate::note::Note;
use crate::types::Duration;

/// 单拍（可以是单音也可以是和弦）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Beat {
    /// 时值
    pub duration: Duration,
    /// 这一拍包含的音符（多个 = 和弦）
    pub notes: Vec<Note>,
    /// 是否为休止符（notes 为空时通常是休止）
    pub is_rest: bool,
    /// 拍级效果
    pub effects: Vec<BeatEffect>,
    /// 附加的和弦图
    pub chord: Option<ChordDiagram>,
    /// 文本标注
    pub text: Option<String>,
    /// 起始 tick 位置（排版时计算）
    pub start_tick: u32,
}


impl Beat {
    /// 是否为空拍（休止或无音符）
    pub fn is_empty(&self) -> bool {
        self.is_rest || self.notes.is_empty()
    }

    /// 这一拍的 tick 时值
    pub fn ticks(&self) -> u32 {
        self.duration.ticks()
    }

    /// 获取指定弦上的音符
    pub fn note_on_string(&self, string: u8) -> Option<&Note> {
        self.notes.iter().find(|n| n.string == string)
    }

    /// 获取指定弦上的音符（可变）
    pub fn note_on_string_mut(&mut self, string: u8) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.string == string)
    }

    /// 是否有和弦图
    pub fn has_chord(&self) -> bool {
        self.chord.is_some()
    }
}

/// 声部 — 一个小节中的一条旋律线
///
/// GP 格式支持每小节最多 4 个声部，
/// 通常只有 Voice 0 被使用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Voice {
    /// 这个声部中的所有拍
    pub beats: Vec<Beat>,
}

impl Voice {
    /// 是否为空声部
    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    /// 这个声部的总 tick 数
    pub fn total_ticks(&self) -> u32 {
        self.beats.iter().map(|b| b.ticks()).sum()
    }
}
