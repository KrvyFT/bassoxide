//! 音符数据模型。
//!
//! `Note` 是数据层次的最底层，代表一个具体的音符。
//! 在和弦中，一个 `Beat` 包含多个同时发声的 `Note`。

use serde::{Deserialize, Serialize};

use crate::effects::NoteEffect;
use crate::types::{Fret, MidiNote, StringNumber, Velocity};

/// 音符类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NoteType {
    /// 正常音符
    #[default]
    Normal,
    /// 延音 (Tie)：与前一个同弦音符连接
    Tie,
    /// 死音 (Dead Note / Muted)
    Dead,
    /// 休止符位置的占位（通常不直接使用）
    Rest,
}

/// 单个音符
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// 所在弦号 (1-based)
    pub string: StringNumber,
    /// 品格 (-1 = 死音, 0 = 空弦, 1+ = 正常品格)
    pub fret: Fret,
    /// 音符类型
    pub note_type: NoteType,
    /// MIDI 力度 (1–127)
    pub velocity: Velocity,
    /// 计算后的 MIDI 音高（由 string tuning + fret 得出）
    pub midi_note: MidiNote,
    /// 附加的演奏效果列表
    pub effects: Vec<NoteEffect>,
    /// 左手指法
    pub left_fingering: Option<crate::effects::Fingering>,
    /// 右手指法
    pub right_fingering: Option<crate::effects::Fingering>,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            string: 1,
            fret: 0,
            note_type: NoteType::Normal,
            velocity: 95,
            midi_note: 0,
            effects: Vec::new(),
            left_fingering: None,
            right_fingering: None,
        }
    }
}

impl Note {
    /// 是否为死音
    pub fn is_dead(&self) -> bool {
        self.note_type == NoteType::Dead || self.fret < 0
    }

    /// 是否为延音
    pub fn is_tie(&self) -> bool {
        self.note_type == NoteType::Tie
    }

    /// 是否附加了指定效果类型
    pub fn has_effect<F>(&self, predicate: F) -> bool
    where
        F: Fn(&NoteEffect) -> bool,
    {
        self.effects.iter().any(predicate)
    }

    /// 是否有推弦效果
    pub fn has_bend(&self) -> bool {
        self.has_effect(|e| matches!(e, NoteEffect::Bend(_)))
    }

    /// 是否有滑音效果
    pub fn has_slide(&self) -> bool {
        self.has_effect(|e| matches!(e, NoteEffect::Slide(_)))
    }

    /// 是否有泛音效果
    pub fn has_harmonic(&self) -> bool {
        self.has_effect(|e| matches!(e, NoteEffect::Harmonic(_)))
    }

    /// 是否有闷音 (Palm Mute)
    pub fn has_palm_mute(&self) -> bool {
        self.has_effect(|e| matches!(e, NoteEffect::PalmMute))
    }

    /// 是否有 Let Ring
    pub fn has_let_ring(&self) -> bool {
        self.has_effect(|e| matches!(e, NoteEffect::LetRing))
    }
}
