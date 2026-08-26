//! 音频引擎骨架。
//!
//! Phase 1 仅提供空壳，Phase 2 将接入 cpal + rustysynth。

pub struct AudioEngine;

impl AudioEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
