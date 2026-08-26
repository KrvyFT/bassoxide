//! 自动化数据（速度变化、音量变化等随时间轴的参数曲线）。

use serde::{Deserialize, Serialize};

/// 速度变化事件
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    /// BPM 值
    pub bpm: u16,
    /// 是否渐变到此速度（而非突变）
    pub is_gradual: bool,
}

impl Default for TempoChange {
    fn default() -> Self {
        Self {
            bpm: 120,
            is_gradual: false,
        }
    }
}
