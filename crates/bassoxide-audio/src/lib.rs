//! 音频文件解码、节拍检测与 PCM 回放（不再合成 MIDI 音符）。

pub mod beat;
pub mod decode;
pub mod error;
pub mod playback;

pub use beat::{
    analyze_beats, compute_peaks, default_beats_per_bar, measure_at_score_secs,
    score_secs_in_measure, score_timeline, snap_to_nearest_beat, BeatAnalysis, ScoreTimeline,
};
pub use decode::{decode_file, DecodedAudio};
pub use error::{AudioError, Result};
pub use playback::{AudioPlayer, PlaybackStatus};
