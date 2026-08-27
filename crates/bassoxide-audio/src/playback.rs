//! PCM 文件回放（非 MIDI）。谱面时间由 UI 帧时钟推进，音频回调只读位置取采样。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use crate::error::{AudioError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

struct CallbackState {
    samples: Mutex<Arc<Vec<f32>>>,
    source_sr: Mutex<u32>,
    score_secs: AtomicU64,
    sync_offset: AtomicU64,
    playing: AtomicBool,
}

/// 音频轨播放器
pub struct AudioPlayer {
    state: Arc<CallbackState>,
    _stream: Option<Stream>,
    status: Mutex<PlaybackStatus>,
    paused_score: Mutex<f64>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        match Self::try_new_with_device() {
            Ok(p) => Ok(p),
            Err(e) => {
                tracing::warn!("音频设备不可用 ({e})；仅使用 UI 时钟推进播放头");
                Ok(Self::new_silent())
            }
        }
    }

    fn new_silent() -> Self {
        Self {
            state: Arc::new(CallbackState {
                samples: Mutex::new(Arc::new(Vec::new())),
                source_sr: Mutex::new(44100),
                score_secs: AtomicU64::new(0f64.to_bits()),
                sync_offset: AtomicU64::new(0f64.to_bits()),
                playing: AtomicBool::new(false),
            }),
            _stream: None,
            status: Mutex::new(PlaybackStatus::Stopped),
            paused_score: Mutex::new(0.0),
        }
    }

    fn try_new_with_device() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceError("无输出设备".into()))?;
        let config = device
            .default_output_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;
        let sample_format = config.sample_format();
        let stream_config: StreamConfig = config.into();
        let channels = stream_config.channels as usize;

        let state = Arc::new(CallbackState {
            samples: Mutex::new(Arc::new(Vec::new())),
            source_sr: Mutex::new(44100),
            score_secs: AtomicU64::new(0f64.to_bits()),
            sync_offset: AtomicU64::new(0f64.to_bits()),
            playing: AtomicBool::new(false),
        });

        let state_cb = state.clone();
        let err_fn = |e| tracing::error!("音频流错误: {e}");

        // 回调只负责出声，不推进时间（时间由 UI tick 统一推进）
        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| fill_output(data, channels, &state_cb),
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let state_cb = state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        let mut tmp = vec![0.0f32; data.len()];
                        fill_output(&mut tmp, channels, &state_cb);
                        for (o, s) in data.iter_mut().zip(tmp.iter()) {
                            *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(AudioError::DeviceError(format!(
                    "不支持的采样格式: {other:?}"
                )));
            }
        }
        .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        Ok(Self {
            state,
            _stream: Some(stream),
            status: Mutex::new(PlaybackStatus::Stopped),
            paused_score: Mutex::new(0.0),
        })
    }

    pub fn set_audio(&self, samples: Arc<Vec<f32>>, sample_rate: u32) {
        *self.state.samples.lock().unwrap() = samples;
        *self.state.source_sr.lock().unwrap() = sample_rate.max(1);
        self.stop();
    }

    pub fn clear_audio(&self) {
        *self.state.samples.lock().unwrap() = Arc::new(Vec::new());
        self.stop();
    }

    pub fn set_sync_offset(&self, offset_secs: f64) {
        self.state
            .sync_offset
            .store(offset_secs.to_bits(), Ordering::Relaxed);
    }

    pub fn sync_offset(&self) -> f64 {
        f64::from_bits(self.state.sync_offset.load(Ordering::Relaxed))
    }

    pub fn status(&self) -> PlaybackStatus {
        *self.status.lock().unwrap()
    }

    pub fn score_position_secs(&self) -> f64 {
        f64::from_bits(self.state.score_secs.load(Ordering::Relaxed))
    }

    pub fn play(&self) {
        let status = *self.status.lock().unwrap();
        match status {
            PlaybackStatus::Playing => {}
            PlaybackStatus::Paused => {
                let t = *self.paused_score.lock().unwrap();
                self.state.score_secs.store(t.to_bits(), Ordering::Relaxed);
                self.state.playing.store(true, Ordering::Relaxed);
                *self.status.lock().unwrap() = PlaybackStatus::Playing;
            }
            PlaybackStatus::Stopped => {
                self.state
                    .score_secs
                    .store(0f64.to_bits(), Ordering::Relaxed);
                self.state.playing.store(true, Ordering::Relaxed);
                *self.status.lock().unwrap() = PlaybackStatus::Playing;
            }
        }
    }

    pub fn pause(&self) {
        if *self.status.lock().unwrap() == PlaybackStatus::Playing {
            self.state.playing.store(false, Ordering::Relaxed);
            *self.paused_score.lock().unwrap() = self.score_position_secs();
            *self.status.lock().unwrap() = PlaybackStatus::Paused;
        }
    }

    pub fn stop(&self) {
        self.state.playing.store(false, Ordering::Relaxed);
        self.state
            .score_secs
            .store(0f64.to_bits(), Ordering::Relaxed);
        *self.paused_score.lock().unwrap() = 0.0;
        *self.status.lock().unwrap() = PlaybackStatus::Stopped;
    }

    pub fn seek_score_secs(&self, secs: f64) {
        let secs = secs.max(0.0);
        self.state.score_secs.store(secs.to_bits(), Ordering::Relaxed);
        *self.paused_score.lock().unwrap() = secs;
    }

    /// 由 UI 每帧调用：推进谱面时间；到音频末尾自动停止。
    pub fn tick(&self, dt_secs: f64) {
        if *self.status.lock().unwrap() != PlaybackStatus::Playing {
            return;
        }
        if !self.state.playing.load(Ordering::Relaxed) {
            // 回调侧可能已停；同步状态
            *self.status.lock().unwrap() = PlaybackStatus::Stopped;
            return;
        }
        let dt = dt_secs.clamp(0.0, 0.1);
        let score = self.score_position_secs() + dt;
        self.state.score_secs.store(score.to_bits(), Ordering::Relaxed);

        let samples = self.state.samples.lock().unwrap().clone();
        let sr = *self.state.source_sr.lock().unwrap();
        if samples.is_empty() || sr == 0 {
            return;
        }
        let sync = self.sync_offset();
        let dur = samples.len() as f64 / f64::from(sr);
        if score - sync >= dur {
            self.stop();
        }
    }
}

fn fill_output(data: &mut [f32], channels: usize, state: &CallbackState) {
    data.fill(0.0);
    if !state.playing.load(Ordering::Relaxed) {
        return;
    }

    let samples = state.samples.lock().unwrap().clone();
    let source_sr = *state.source_sr.lock().unwrap();
    if samples.is_empty() || source_sr == 0 {
        return;
    }

    let sync = f64::from_bits(state.sync_offset.load(Ordering::Relaxed));
    let score = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
    let duration = samples.len() as f64 / f64::from(source_sr);
    let audio_t = score - sync;

    // 整块缓冲用同一时刻近似（精细相位由 UI tick 高频刷新）
    let mut sample = 0.0_f32;
    if audio_t >= 0.0 && audio_t < duration {
        let idx_f = audio_t * f64::from(source_sr);
        let idx = idx_f.floor() as usize;
        if idx + 1 < samples.len() {
            let frac = (idx_f - idx as f64) as f32;
            sample = samples[idx] * (1.0 - frac) + samples[idx + 1] * frac;
        } else if idx < samples.len() {
            sample = samples[idx];
        }
    }

    for frame in data.chunks_mut(channels) {
        for ch in frame.iter_mut() {
            *ch = sample;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_while_playing() {
        let player = AudioPlayer::new_silent();
        let samples = Arc::new(vec![0.1_f32; 44100 * 4]); // 4s
        player.set_audio(samples, 44100);
        player.play();
        assert_eq!(player.status(), PlaybackStatus::Playing);
        // tick 会把单帧 dt 钳制到 0.1，避免掉帧跳表
        player.tick(0.5);
        let t = player.score_position_secs();
        assert!((t - 0.1).abs() < 1e-6, "t={t}");
        player.tick(0.05);
        assert!((player.score_position_secs() - 0.15).abs() < 1e-6);
    }
}
