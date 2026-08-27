//! PCM 文件回放（非 MIDI）。
//!
//! 有声卡时：音频回调推进谱面时间并出声（避免 UI 帧率导致卡顿）。
//! 无声卡时：由 UI `tick` 推进播放头。
//!
//! 支持：练习变速（playback rate）、A-B 定点循环、合成节拍器点击。

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

/// 节拍器调度（谱面秒）
#[derive(Debug, Clone, Default)]
struct MetroSchedule {
    beat_times: Vec<f64>,
    /// 小节起点（用于强拍）
    measure_times: Vec<f64>,
}

struct CallbackState {
    samples: Mutex<Arc<Vec<f32>>>,
    source_sr: Mutex<u32>,
    /// 输出设备采样率（回调推进时间用）
    device_sr: AtomicU32,
    score_secs: AtomicU64,
    sync_offset: AtomicU64,
    playing: AtomicBool,
    /// 练习变速（f64 bits），默认 1.0，夹紧 0.5..=1.5
    playback_rate: AtomicU64,
    loop_enabled: AtomicBool,
    loop_a: AtomicU64,
    loop_b: AtomicU64,
    metronome_enabled: AtomicBool,
    metro: Mutex<MetroSchedule>,
    /// 节拍器点击包络相位（样本计数，>0 表示正在发声）
    click_samples_left: AtomicU32,
    click_is_accent: AtomicBool,
    /// 下一拍在 beat_times 中的查找游标
    metro_cursor: AtomicU32,
}

fn default_callback_state(device_sr: u32) -> CallbackState {
    CallbackState {
        samples: Mutex::new(Arc::new(Vec::new())),
        source_sr: Mutex::new(44100),
        device_sr: AtomicU32::new(device_sr),
        score_secs: AtomicU64::new(0f64.to_bits()),
        sync_offset: AtomicU64::new(0f64.to_bits()),
        playing: AtomicBool::new(false),
        playback_rate: AtomicU64::new(1.0f64.to_bits()),
        loop_enabled: AtomicBool::new(false),
        loop_a: AtomicU64::new(0f64.to_bits()),
        loop_b: AtomicU64::new(0f64.to_bits()),
        metronome_enabled: AtomicBool::new(false),
        metro: Mutex::new(MetroSchedule::default()),
        click_samples_left: AtomicU32::new(0),
        click_is_accent: AtomicBool::new(false),
        metro_cursor: AtomicU32::new(0),
    }
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
            state: Arc::new(default_callback_state(44100)),
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

        let state = Arc::new(default_callback_state(device_sr));

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

    /// 练习变速 0.5..=1.5
    pub fn set_playback_rate(&self, rate: f64) {
        let rate = rate.clamp(0.5, 1.5);
        self.state
            .playback_rate
            .store(rate.to_bits(), Ordering::Relaxed);
    }

    pub fn playback_rate(&self) -> f64 {
        f64::from_bits(self.state.playback_rate.load(Ordering::Relaxed))
    }

    /// 设置 A-B 循环（谱面秒）。`a >= b` 时视为无效区间，不启用 wrap。
    pub fn set_loop(&self, a: f64, b: f64, enabled: bool) {
        self.state.loop_a.store(a.max(0.0).to_bits(), Ordering::Relaxed);
        self.state.loop_b.store(b.max(0.0).to_bits(), Ordering::Relaxed);
        self.state.loop_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn loop_enabled(&self) -> bool {
        self.state.loop_enabled.load(Ordering::Relaxed)
    }

    pub fn set_metronome(&self, enabled: bool) {
        self.state
            .metronome_enabled
            .store(enabled, Ordering::Relaxed);
        if !enabled {
            self.state.click_samples_left.store(0, Ordering::Relaxed);
        }
    }

    pub fn metronome_enabled(&self) -> bool {
        self.state.metronome_enabled.load(Ordering::Relaxed)
    }

    /// 更新节拍器调度（谱面秒拍点 + 小节线）
    pub fn set_metronome_schedule(&self, beat_times: Vec<f64>, measure_times: Vec<f64>) {
        let mut metro = self.state.metro.lock().unwrap();
        metro.beat_times = beat_times;
        metro.measure_times = measure_times;
        self.state.metro_cursor.store(0, Ordering::Relaxed);
    }

    pub fn play(&self) {
        let status = *self.status.lock().unwrap();
        match status {
            PlaybackStatus::Playing => {}
            PlaybackStatus::Paused => {
                let t = *self.paused_score.lock().unwrap();
                self.state.score_secs.store(t.to_bits(), Ordering::Relaxed);
                self.resync_metro_cursor(t);
                self.state.playing.store(true, Ordering::Relaxed);
                *self.status.lock().unwrap() = PlaybackStatus::Playing;
            }
            PlaybackStatus::Stopped => {
                let start = self.loop_start_on_play();
                self.state
                    .score_secs
                    .store(start.to_bits(), Ordering::Relaxed);
                self.resync_metro_cursor(start);
                self.state.playing.store(true, Ordering::Relaxed);
                *self.status.lock().unwrap() = PlaybackStatus::Playing;
            }
        }
    }

    fn loop_start_on_play(&self) -> f64 {
        if self.state.loop_enabled.load(Ordering::Relaxed) {
            let a = f64::from_bits(self.state.loop_a.load(Ordering::Relaxed));
            let b = f64::from_bits(self.state.loop_b.load(Ordering::Relaxed));
            if a < b {
                return a;
            }
        }
        0.0
    }

    fn resync_metro_cursor(&self, score: f64) {
        let metro = self.state.metro.lock().unwrap();
        let idx = metro
            .beat_times
            .iter()
            .position(|&t| t >= score - 1e-9)
            .unwrap_or(metro.beat_times.len()) as u32;
        self.state.metro_cursor.store(idx, Ordering::Relaxed);
        self.state.click_samples_left.store(0, Ordering::Relaxed);
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
        self.state.click_samples_left.store(0, Ordering::Relaxed);
        self.state.metro_cursor.store(0, Ordering::Relaxed);
        *self.status.lock().unwrap() = PlaybackStatus::Stopped;
    }

    pub fn seek_score_secs(&self, secs: f64) {
        let secs = secs.max(0.0);
        self.state.score_secs.store(secs.to_bits(), Ordering::Relaxed);
        *self.paused_score.lock().unwrap() = secs;
        self.resync_metro_cursor(secs);
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
            self.check_eof();
            return;
        }

        let dt = dt_secs.clamp(0.0, 0.1);
        let rate = self.playback_rate();
        let mut score = self.score_position_secs() + dt * rate;
        score = apply_loop_wrap(&self.state, score);
        // silent 模式也触发节拍器游标推进（无声音）
        advance_metro_silent(&self.state, self.score_position_secs(), score);
        self.state.score_secs.store(score.to_bits(), Ordering::Relaxed);
        self.check_eof();
    }

    fn check_eof(&self) {
        // 循环开启时不因 EOF 停
        if self.state.loop_enabled.load(Ordering::Relaxed) {
            let a = f64::from_bits(self.state.loop_a.load(Ordering::Relaxed));
            let b = f64::from_bits(self.state.loop_b.load(Ordering::Relaxed));
            if a < b {
                return;
            }
        }
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

fn apply_loop_wrap(state: &CallbackState, mut score: f64) -> f64 {
    if !state.loop_enabled.load(Ordering::Relaxed) {
        return score;
    }
    let a = f64::from_bits(state.loop_a.load(Ordering::Relaxed));
    let b = f64::from_bits(state.loop_b.load(Ordering::Relaxed));
    if a < b && score >= b {
        score = a;
        // 循环后重同步节拍游标
        if let Ok(metro) = state.metro.lock() {
            let idx = metro
                .beat_times
                .iter()
                .position(|&t| t >= a - 1e-9)
                .unwrap_or(0) as u32;
            state.metro_cursor.store(idx, Ordering::Relaxed);
        }
    }
    score
}

fn advance_metro_silent(state: &CallbackState, prev: f64, next: f64) {
    if !state.metronome_enabled.load(Ordering::Relaxed) {
        return;
    }
    let Ok(metro) = state.metro.lock() else {
        return;
    };
    let mut cursor = state.metro_cursor.load(Ordering::Relaxed) as usize;
    while cursor < metro.beat_times.len() {
        let bt = metro.beat_times[cursor];
        if bt < prev - 1e-9 {
            cursor += 1;
            continue;
        }
        if bt > next + 1e-9 {
            break;
        }
        // 触发（silent：只推进游标）
        cursor += 1;
    }
    state.metro_cursor.store(cursor as u32, Ordering::Relaxed);
}

fn is_measure_start(measure_times: &[f64], beat_t: f64) -> bool {
    measure_times
        .iter()
        .any(|&mt| (mt - beat_t).abs() < 1e-3)
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

/// 合成点击：短衰减正弦
fn click_sample(phase: u32, total: u32, accent: bool, device_sr: u32) -> f32 {
    if total == 0 || phase >= total {
        return 0.0;
    }
    let t = phase as f32 / device_sr as f32;
    let freq = if accent { 1200.0 } else { 900.0 };
    let amp = if accent { 0.35 } else { 0.22 };
    let env = 1.0 - (phase as f32 / total as f32);
    (t * freq * std::f32::consts::TAU).sin() * amp * env * env
}

fn fill_output(data: &mut [f32], channels: usize, state: &CallbackState) {
    data.fill(0.0);
    if !state.playing.load(Ordering::Relaxed) {
        return;
    }

    let samples = state.samples.lock().unwrap().clone();
    let source_sr = *state.source_sr.lock().unwrap();
    let device_sr = state.device_sr.load(Ordering::Relaxed).max(1);
    let rate = f64::from_bits(state.playback_rate.load(Ordering::Relaxed)).clamp(0.5, 1.5);
    let sync = f64::from_bits(state.sync_offset.load(Ordering::Relaxed));
    let mut score = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
    let dt = (1.0 / f64::from(device_sr)) * rate;
    let duration = if source_sr > 0 && !samples.is_empty() {
        samples.len() as f64 / f64::from(source_sr)
    } else {
        f64::INFINITY
    };

    let metro_on = state.metronome_enabled.load(Ordering::Relaxed);
    let metro = state.metro.lock().unwrap();
    let mut metro_cursor = state.metro_cursor.load(Ordering::Relaxed) as usize;
    let click_len = (device_sr as f32 * 0.025).round() as u32; // ~25ms

    let frames = data.len() / channels.max(1);
    for fi in 0..frames {
        let audio_t = score - sync;
        let mut sample = if samples.is_empty() || source_sr == 0 {
            0.0
        } else {
            sample_at(&samples, source_sr, audio_t)
        };

        // 节拍器：跨过拍点则启动点击包络
        if metro_on {
            while metro_cursor < metro.beat_times.len() {
                let bt = metro.beat_times[metro_cursor];
                if bt <= score + 1e-9 {
                    let accent = is_measure_start(&metro.measure_times, bt);
                    state.click_is_accent.store(accent, Ordering::Relaxed);
                    state.click_samples_left.store(click_len, Ordering::Relaxed);
                    metro_cursor += 1;
                } else {
                    break;
                }
            }
            let left = state.click_samples_left.load(Ordering::Relaxed);
            if left > 0 {
                let phase = click_len.saturating_sub(left);
                let accent = state.click_is_accent.load(Ordering::Relaxed);
                sample += click_sample(phase, click_len, accent, device_sr);
                state
                    .click_samples_left
                    .store(left.saturating_sub(1), Ordering::Relaxed);
            }
        }

        let base = fi * channels;
        for ch in 0..channels {
            data[base + ch] = sample.clamp(-1.0, 1.0);
        }

        let prev = score;
        score += dt;
        score = apply_loop_wrap_inline(state, score, &metro, &mut metro_cursor);

        // EOF（无有效循环时）
        if audio_t >= duration && !samples.is_empty() {
            let looping = state.loop_enabled.load(Ordering::Relaxed);
            let a = f64::from_bits(state.loop_a.load(Ordering::Relaxed));
            let b = f64::from_bits(state.loop_b.load(Ordering::Relaxed));
            if !(looping && a < b) {
                state.playing.store(false, Ordering::Relaxed);
                let _ = prev;
                break;
            }
        }
    }

    state.metro_cursor.store(metro_cursor as u32, Ordering::Relaxed);
    state.score_secs.store(score.to_bits(), Ordering::SeqCst);
}

fn apply_loop_wrap_inline(
    state: &CallbackState,
    mut score: f64,
    metro: &MetroSchedule,
    metro_cursor: &mut usize,
) -> f64 {
    if !state.loop_enabled.load(Ordering::Relaxed) {
        return score;
    }
    let a = f64::from_bits(state.loop_a.load(Ordering::Relaxed));
    let b = f64::from_bits(state.loop_b.load(Ordering::Relaxed));
    if a < b && score >= b {
        score = a;
        *metro_cursor = metro
            .beat_times
            .iter()
            .position(|&t| t >= a - 1e-9)
            .unwrap_or(0);
    }
    score
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
    fn playback_rate_scales_tick() {
        let player = AudioPlayer::new_silent();
        player.set_audio(Arc::new(vec![0.1_f32; 44100 * 10]), 44100);
        player.set_playback_rate(2.0); // clamp to 1.5
        assert!((player.playback_rate() - 1.5).abs() < 1e-9);
        player.set_playback_rate(0.5);
        player.play();
        // tick 内部 dt clamp 到 0.1，再乘 rate → 0.05
        player.tick(0.2);
        let t = player.score_position_secs();
        assert!((t - 0.05).abs() < 1e-6, "t={t}");
    }

    #[test]
    fn loop_wraps_score_to_a() {
        let player = AudioPlayer::new_silent();
        player.set_audio(Arc::new(vec![0.1_f32; 44100 * 20]), 44100);
        player.set_loop(1.0, 1.2, true);
        player.seek_score_secs(1.15);
        player.play();
        // 以 rate=1 推进超过 B
        player.tick(0.1);
        let t = player.score_position_secs();
        assert!(t < 1.2, "should wrap, t={t}");
        assert!(t >= 1.0 && t < 1.15, "t={t}");
    }

    #[test]
    fn fill_output_advances_score() {
        let state = default_callback_state(44100);
        *state.samples.lock().unwrap() = Arc::new(vec![0.5_f32; 44100]);
        *state.source_sr.lock().unwrap() = 44100;
        state.playing.store(true, Ordering::Relaxed);
        let mut buf = vec![0.0f32; 441]; // 10ms @ 44.1k
        fill_output(&mut buf, 1, &state);
        let t = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
        assert!((t - 0.01).abs() < 1e-4, "t={t}");
        assert!(buf.iter().any(|s| *s > 0.0));
    }

    #[test]
    fn fill_output_mixes_metronome_click() {
        let state = default_callback_state(44100);
        // 无 PCM，仅节拍器
        state.playing.store(true, Ordering::Relaxed);
        state.metronome_enabled.store(true, Ordering::Relaxed);
        {
            let mut m = state.metro.lock().unwrap();
            m.beat_times = vec![0.0, 0.5, 1.0];
            m.measure_times = vec![0.0, 1.0];
        }
        let mut buf = vec![0.0f32; 441]; // 10ms — 应触发 t=0 强拍
        fill_output(&mut buf, 1, &state);
        assert!(
            buf.iter().any(|s| s.abs() > 0.01),
            "expected metronome click energy"
        );
    }

    #[test]
    fn fill_output_loop_wrap() {
        let state = default_callback_state(44100);
        *state.samples.lock().unwrap() = Arc::new(vec![0.25_f32; 44100 * 5]);
        *state.source_sr.lock().unwrap() = 44100;
        state.playing.store(true, Ordering::Relaxed);
        state.loop_enabled.store(true, Ordering::Relaxed);
        state.loop_a.store(0.5f64.to_bits(), Ordering::Relaxed);
        state.loop_b.store(0.52f64.to_bits(), Ordering::Relaxed);
        state
            .score_secs
            .store(0.515f64.to_bits(), Ordering::Relaxed);
        let mut buf = vec![0.0f32; 882]; // 20ms @ 44.1k — 足够越过 B
        fill_output(&mut buf, 1, &state);
        let t = f64::from_bits(state.score_secs.load(Ordering::Relaxed));
        assert!(t < 0.52, "t={t}");
        assert!(t >= 0.5, "t={t}");
    }
}
