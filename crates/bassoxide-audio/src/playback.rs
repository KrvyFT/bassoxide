//! PCM 文件回放（非 MIDI）。按「谱面时间」推进，用 sync_offset 对齐音频。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
                tracing::warn!("音频设备不可用 ({e})；使用软件时钟播放器");
                Ok(Self::new_silent())
            }
        }
    }

    fn new_silent() -> Self {
        let state = Arc::new(CallbackState {
            samples: Mutex::new(Arc::new(Vec::new())),
            source_sr: Mutex::new(44100),
            score_secs: AtomicU64::new(0f64.to_bits()),
            sync_offset: AtomicU64::new(0f64.to_bits()),
            playing: AtomicBool::new(false),
        });
        start_soft_clock(state.clone());
        Self {
            state,
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
        let device_sr = stream_config.sample_rate.0;
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

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| fill_output(data, channels, device_sr, &state_cb),
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let state_cb = state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        let mut tmp = vec![0.0f32; data.len()];
                        fill_output(&mut tmp, channels, device_sr, &state_cb);
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
}

fn start_soft_clock(state: Arc<CallbackState>) {
    thread::spawn(move || {
        let mut last = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(16));
            if !state.playing.load(Ordering::Relaxed) {
                last = Instant::now();
                continue;
            }
            let now = Instant::now();
            let dt = now.duration_since(last).as_secs_f64();
            last = now;
            let score = f64::from_bits(state.score_secs.load(Ordering::Relaxed)) + dt;
            state.score_secs.store(score.to_bits(), Ordering::Relaxed);

            let samples = state.samples.lock().unwrap().clone();
            let sr = *state.source_sr.lock().unwrap();
            if !samples.is_empty() && sr > 0 {
                let sync = f64::from_bits(state.sync_offset.load(Ordering::Relaxed));
                let audio_t = score - sync;
                let dur = samples.len() as f64 / f64::from(sr);
                if audio_t >= dur {
                    state.playing.store(false, Ordering::Relaxed);
                }
            }
        }
    });
}

fn fill_output(data: &mut [f32], channels: usize, device_sr: u32, state: &CallbackState) {
    data.fill(0.0);
    if !state.playing.load(Ordering::Relaxed) {
        return;
    }

    let samples = state.samples.lock().unwrap().clone();
    let source_sr = *state.source_sr.lock().unwrap();
    if samples.is_empty() || source_sr == 0 || device_sr == 0 {
        return;
    }

    let sync = f64::from_bits(state.sync_offset.load(Ordering::Relaxed));
    let mut score = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
    let dt = 1.0 / f64::from(device_sr);
    let duration = samples.len() as f64 / f64::from(source_sr);

    for frame in data.chunks_mut(channels) {
        let audio_t = score - sync;
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
        } else if audio_t >= duration {
            state.playing.store(false, Ordering::Relaxed);
        }
        for ch in frame.iter_mut() {
            *ch = sample;
        }
        score += dt;
    }
    state.score_secs.store(score.to_bits(), Ordering::Relaxed);
}
