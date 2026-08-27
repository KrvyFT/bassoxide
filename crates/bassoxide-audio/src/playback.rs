//! PCM 文件回放（非 MIDI）。
//!
//! 有声卡时：音频回调推进谱面时间并出声（避免 UI 帧率导致卡顿）。
//! 无声卡时：由 UI `tick` 推进播放头。

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
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
    /// 输出设备采样率（回调推进时间用）
    device_sr: AtomicU32,
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
    /// 无输出设备时为 true，需 UI tick 推进
    silent: bool,
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
                device_sr: AtomicU32::new(44100),
                score_secs: AtomicU64::new(0f64.to_bits()),
                sync_offset: AtomicU64::new(0f64.to_bits()),
                playing: AtomicBool::new(false),
            }),
            _stream: None,
            status: Mutex::new(PlaybackStatus::Stopped),
            paused_score: Mutex::new(0.0),
            silent: true,
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
        let device_sr = stream_config.sample_rate.0;

        let state = Arc::new(CallbackState {
            samples: Mutex::new(Arc::new(Vec::new())),
            source_sr: Mutex::new(44100),
            device_sr: AtomicU32::new(device_sr),
            score_secs: AtomicU64::new(0f64.to_bits()),
            sync_offset: AtomicU64::new(0f64.to_bits()),
            playing: AtomicBool::new(false),
        });

        let state_cb = state.clone();
        let err_fn = |e| tracing::error!("音频流错误: {e}");

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
            silent: false,
        })
    }

    pub fn is_silent(&self) -> bool {
        self.silent
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

    pub fn toggle_play_pause(&self) {
        if self.status() == PlaybackStatus::Playing {
            self.pause();
        } else {
            self.play();
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

    /// 无声卡模式下由 UI 每帧调用推进时间；有声卡时回调已推进，此处只同步结束状态。
    pub fn tick(&self, dt_secs: f64) {
        if *self.status.lock().unwrap() != PlaybackStatus::Playing {
            return;
        }
        if !self.state.playing.load(Ordering::Relaxed) {
            *self.status.lock().unwrap() = PlaybackStatus::Stopped;
            return;
        }

        if !self.silent {
            // 时钟由音频回调推进；检测是否已到末尾
            self.check_eof();
            return;
        }

        let dt = dt_secs.clamp(0.0, 0.1);
        let score = self.score_position_secs() + dt;
        self.state.score_secs.store(score.to_bits(), Ordering::Relaxed);
        self.check_eof();
    }

    fn check_eof(&self) {
        let samples = self.state.samples.lock().unwrap().clone();
        let sr = *self.state.source_sr.lock().unwrap();
        if samples.is_empty() || sr == 0 {
            return;
        }
        let sync = self.sync_offset();
        let dur = samples.len() as f64 / f64::from(sr);
        if self.score_position_secs() - sync >= dur {
            self.stop();
        }
    }
}

fn sample_at(samples: &[f32], source_sr: u32, audio_t: f64) -> f32 {
    if audio_t < 0.0 || samples.is_empty() || source_sr == 0 {
        return 0.0;
    }
    let duration = samples.len() as f64 / f64::from(source_sr);
    if audio_t >= duration {
        return 0.0;
    }
    let idx_f = audio_t * f64::from(source_sr);
    let idx = idx_f.floor() as usize;
    if idx + 1 < samples.len() {
        let frac = (idx_f - idx as f64) as f32;
        samples[idx] * (1.0 - frac) + samples[idx + 1] * frac
    } else if idx < samples.len() {
        samples[idx]
    } else {
        0.0
    }
}

fn fill_output(data: &mut [f32], channels: usize, state: &CallbackState) {
    data.fill(0.0);
    if !state.playing.load(Ordering::Relaxed) {
        return;
    }

    let samples = state.samples.lock().unwrap().clone();
    let source_sr = *state.source_sr.lock().unwrap();
    let device_sr = state.device_sr.load(Ordering::Relaxed).max(1);
    if samples.is_empty() || source_sr == 0 {
        return;
    }

    let sync = f64::from_bits(state.sync_offset.load(Ordering::Relaxed));
    let mut score = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
    let dt = 1.0 / f64::from(device_sr);
    let duration = samples.len() as f64 / f64::from(source_sr);

    for frame in data.chunks_mut(channels) {
        let audio_t = score - sync;
        let sample = sample_at(&samples, source_sr, audio_t);
        for ch in frame.iter_mut() {
            *ch = sample;
        }
        score += dt;
        if audio_t >= duration {
            state.playing.store(false, Ordering::Relaxed);
            break;
        }
    }

    state.score_secs.store(score.to_bits(), Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_while_playing_silent() {
        let player = AudioPlayer::new_silent();
        let samples = Arc::new(vec![0.1_f32; 44100 * 4]);
        player.set_audio(samples, 44100);
        player.play();
        assert_eq!(player.status(), PlaybackStatus::Playing);
        player.tick(0.5);
        let t = player.score_position_secs();
        assert!((t - 0.1).abs() < 1e-6, "t={t}");
        player.tick(0.05);
        assert!((player.score_position_secs() - 0.15).abs() < 1e-6);
    }

    #[test]
    fn fill_output_advances_score() {
        let state = CallbackState {
            samples: Mutex::new(Arc::new(vec![0.5_f32; 44100])),
            source_sr: Mutex::new(44100),
            device_sr: AtomicU32::new(44100),
            score_secs: AtomicU64::new(0f64.to_bits()),
            sync_offset: AtomicU64::new(0f64.to_bits()),
            playing: AtomicBool::new(true),
        };
        let mut buf = vec![0.0f32; 441]; // 10ms @ 44.1k
        fill_output(&mut buf, 1, &state);
        let t = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
        assert!((t - 0.01).abs() < 1e-4, "t={t}");
        assert!(buf.iter().any(|s| *s > 0.0));
    }
}
