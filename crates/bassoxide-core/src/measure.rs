//! 小节数据模型。
//!
//! - `MasterBar`：全局小节信息（拍号、调号、速度），所有轨道共享。
//! - `Measure`：某一轨道在某一小节中的具体音符数据。

use serde::{Deserialize, Serialize};

use crate::beat::Voice;
use crate::types::{BarLineType, KeySignature, RepeatType, TimeSignature};

/// 全局小节信息 — 所有轨道在此小节共享
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterBar {
    /// 拍号（当此小节拍号改变时设置）
    pub time_signature: TimeSignature,
    /// 调号
    pub key_signature: KeySignature,
    /// 速度 (BPM)，None 表示继承前一小节
    pub tempo: Option<u16>,
    /// 反复标记
    pub repeat: Option<RepeatType>,
    /// 小节线类型
    pub bar_line_start: BarLineType,
    pub bar_line_end: BarLineType,
    /// 排练标记 (如 "A", "Chorus")
    pub marker: Option<Marker>,
    /// 是否为不完全小节 (anacrusis / pickup bar)
    pub is_anacrusis: bool,
    /// 段落跳转 (Coda, Segno, Fine, D.C., D.S.)
    pub directions: Vec<Direction>,
    /// 替代结尾编号 (如 [1. ] [2. ])
    pub alternate_endings: u8,
}

impl Default for MasterBar {
    fn default() -> Self {
        Self {
            time_signature: TimeSignature::default(),
            key_signature: KeySignature::default(),
            tempo: None,
            repeat: None,
            bar_line_start: BarLineType::Normal,
            bar_line_end: BarLineType::Normal,
            marker: None,
            is_anacrusis: false,
            directions: Vec::new(),
            alternate_endings: 0,
        }
    }
}

/// 排练标记
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub color: crate::types::Color,
}

/// 段落跳转标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Coda,
    DoubleCoda,
    Segno,
    SegnoSegno,
    Fine,
    DaCapo,
    DaCapoAlCoda,
    DaCapoAlDoubleCoda,
    DaCapoAlFine,
    DalSegno,
    DalSegnoAlCoda,
    DalSegnoAlDoubleCoda,
    DalSegnoAlFine,
    DalSegnoSegno,
    DalSegnoSegnoAlCoda,
    DalSegnoSegnoAlDoubleCoda,
    DalSegnoSegnoAlFine,
}

/// 单轨道在一个小节中的数据
///
/// 每个 `Measure` 最多包含 4 个 `Voice`（声部）。
/// 大多数情况下只使用 Voice 0。
pub const MAX_VOICES: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measure {
    /// 声部（最多 4 个）
    pub voices: [Voice; MAX_VOICES],
    /// 是否有行尾换行标记
    pub line_break: bool,
    /// 谱号变更（仅在小节开头生效）
    pub clef: Option<crate::types::Clef>,
}

impl Default for Measure {
    fn default() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::default()),
            line_break: false,
            clef: None,
        }
    }
}

impl Measure {
    /// 获取主声部（Voice 0）
    pub fn primary_voice(&self) -> &Voice {
        &self.voices[0]
    }

    /// 获取主声部（可变）
    pub fn primary_voice_mut(&mut self) -> &mut Voice {
        &mut self.voices[0]
    }

    /// 是否有任何非空声部
    pub fn has_content(&self) -> bool {
        self.voices.iter().any(|v| !v.is_empty())
    }
}

/// 小节时值校验结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureDurationStatus {
    /// 与拍号一致
    Ok,
    /// 总 ticks 少于拍号
    Under { expected: u32, actual: u32 },
    /// 总 ticks 多于拍号
    Over { expected: u32, actual: u32 },
}

impl MeasureDurationStatus {
    pub fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// actual - expected（多则为正，少则为负）
    pub fn delta_ticks(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::Under { expected, actual } => actual as i32 - expected as i32,
            Self::Over { expected, actual } => actual as i32 - expected as i32,
        }
    }
}

/// 比较声部总时值与拍号期望 ticks
pub fn check_voice_duration(voice: &Voice, expected_ticks: u32) -> MeasureDurationStatus {
    let actual = voice.total_ticks();
    if actual == expected_ticks {
        MeasureDurationStatus::Ok
    } else if actual < expected_ticks {
        MeasureDurationStatus::Under {
            expected: expected_ticks,
            actual,
        }
    } else {
        MeasureDurationStatus::Over {
            expected: expected_ticks,
            actual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beat::Beat;
    use crate::types::{Duration, NoteValue, TimeSignature};

    #[test]
    fn duration_ok_under_over() {
        let expected = TimeSignature::default().measure_ticks(); // 3840
        let mut voice = Voice::default();
        for _ in 0..4 {
            voice.beats.push(Beat {
                duration: Duration {
                    value: NoteValue::Quarter,
                    ..Duration::default()
                },
                ..Beat::default()
            });
        }
        assert_eq!(check_voice_duration(&voice, expected), MeasureDurationStatus::Ok);

        voice.beats.pop();
        assert!(matches!(
            check_voice_duration(&voice, expected),
            MeasureDurationStatus::Under { .. }
        ));

        voice.beats.push(Beat {
            duration: Duration {
                value: NoteValue::Whole,
                ..Duration::default()
            },
            ..Beat::default()
        });
        assert!(matches!(
            check_voice_duration(&voice, expected),
            MeasureDurationStatus::Over { .. }
        ));
    }
}
