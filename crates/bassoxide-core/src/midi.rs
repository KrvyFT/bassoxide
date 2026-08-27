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
        self.channel % 16 == PERCUSSION_CHANNEL
    }

    /// GP 64 槽通道表下标：4 个端口 × 16 通道。
    ///
    /// `port` 在 GP 文件中通常为 1-based；`channel` 为 0-based（已减 1）。
    /// 若 `channel` 已是 16–63 的表下标，则直接使用。
    pub fn table_index(port: u8, channel: u8) -> usize {
        let ch = usize::from(channel);
        if ch >= MIDI_CHANNEL_COUNT {
            MIDI_CHANNEL_COUNT - 1
        } else if ch >= 16 {
            ch
        } else {
            let port_idx = usize::from(port.saturating_sub(1)).min(3);
            port_idx * 16 + ch
        }
    }
}

/// GM 鼓通道固定编号
pub const PERCUSSION_CHANNEL: u8 = 9;

/// 默认 MIDI 通道数
pub const MIDI_CHANNEL_COUNT: usize = 64;
