//! 音频引擎骨架。
//!
//! Phase 1 仅提供空壳，Phase 2 将接入 cpal + rustysynth。

pub mod error;
pub mod playback;
pub mod synth;

pub use playback::{AudioEngine, PlaybackStatus};
pub use error::{AudioError, Result};
