//! MIDI 通道映射。

use serde::{Deserialize, Serialize};

/// MIDI 通道配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiChannel {
    /// 通道号 (0-15)
    pub channel: u8,
    /// 效果通道号 (用于效果器处理)
    pub effect_channel: u8,
    /// 乐器音色 (GM program, 0-127)
    pub instrument: u8,
    /// 音量 (0-127)
    pub volume: u8,
    /// 平衡 (0-127, 64=中)
    pub balance: u8,
    /// 合唱效果深度 (0-127)
    pub chorus: u8,
    /// 混响深度 (0-127)
    pub reverb: u8,
    /// 相位效果 (0-127)
    pub phaser: u8,
    /// 颤音深度 (0-127)
    pub tremolo: u8,
}

impl Default for MidiChannel {
    fn default() -> Self {
        Self {
            channel: 0,
            effect_channel: 0,
            instrument: 25,
            volume: 100,
            balance: 64,
            chorus: 0,
            reverb: 0,
            phaser: 0,
            tremolo: 0,
        }
    }
}

impl MidiChannel {
    /// MIDI 通道 10 (index 9) 是鼓轨道
    pub fn is_percussion(&self) -> bool {
        self.channel == 9
    }
}

/// GM 鼓通道固定编号
pub const PERCUSSION_CHANNEL: u8 = 9;

/// 默认 MIDI 通道数
pub const MIDI_CHANNEL_COUNT: usize = 64;
