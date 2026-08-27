//! 能量包络 + 自相关 BPM 估计，生成拍点与小节线。

use bassoxide_core::song::Song;
use bassoxide_core::types::Duration;

/// 节拍分析结果（相对音频文件起点，秒）
#[derive(Debug, Clone)]
pub struct BeatAnalysis {
    /// 估算 BPM
    pub bpm: f64,
    /// 拍点时间（秒）
    pub beat_times: Vec<f64>,
    /// 小节线时间（秒）—— 每 `beats_per_bar` 拍一条
    pub measure_times: Vec<f64>,
    pub beats_per_bar: u8,
}

/// 谱面时间轴（相对乐谱起点，秒）
#[derive(Debug, Clone, Default)]
pub struct ScoreTimeline {
    pub beat_times: Vec<f64>,
    pub measure_times: Vec<f64>,
    pub duration_secs: f64,
}

/// 根据乐谱拍号/速度构建固定节拍轴
pub fn score_timeline(song: &Song) -> ScoreTimeline {
    let mut beat_times = Vec::new();
    let mut measure_times = vec![0.0];
    let mut t = 0.0_f64;
    let mut bpm = f64::from(song.tempo.max(1));

    for mb in &song.master_bars {
        if let Some(tempo) = mb.tempo {
            bpm = f64::from(tempo.max(1));
        }
        let ts = mb.time_signature;
        let beat_ticks = Duration {
            value: ts.denominator,
            ..Default::default()
        }
        .ticks();
        let beat_secs = (f64::from(beat_ticks) / 960.0) * (60.0 / bpm);
        for b in 0..ts.numerator {
            beat_times.push(t + f64::from(b) * beat_secs);
        }
        t += f64::from(ts.measure_ticks()) / 960.0 * (60.0 / bpm);
        measure_times.push(t);
    }

    ScoreTimeline {
        beat_times,
        measure_times,
        duration_secs: t,
    }
}

/// 谱面时间 → 小节索引与小节内归一化位置（0..1）
pub fn measure_at_score_secs(timeline: &ScoreTimeline, secs: f64) -> (usize, f64) {
    let secs = secs.max(0.0);
    let times = &timeline.measure_times;
    if times.len() < 2 {
        return (0, 0.0);
    }
    for i in 0..times.len() - 1 {
        let a = times[i];
        let b = times[i + 1];
        if secs < b || i + 1 == times.len() - 1 {
            let span = (b - a).max(1e-9);
            let frac = ((secs - a) / span).clamp(0.0, 1.0);
            return (i, frac);
        }
    }
    (times.len().saturating_sub(2), 1.0)
}

/// 点击/插值：小节 + 小节内比例 → 谱面秒
pub fn score_secs_in_measure(timeline: &ScoreTimeline, measure: usize, frac: f64) -> f64 {
    let times = &timeline.measure_times;
    if times.is_empty() {
        return 0.0;
    }
    let i = measure.min(times.len().saturating_sub(2));
    let a = times[i];
    let b = times.get(i + 1).copied().unwrap_or(timeline.duration_secs);
    a + (b - a) * frac.clamp(0.0, 1.0)
}

/// 吸附到最近谱面拍点（阈值内）
pub fn snap_to_nearest_beat(timeline: &ScoreTimeline, secs: f64, max_dist: f64) -> f64 {
    let mut best = secs;
    let mut best_d = max_dist;
    for &t in &timeline.beat_times {
        let d = (t - secs).abs();
        if d < best_d {
            best_d = d;
            best = t;
        }
    }
    for &t in &timeline.measure_times {
        let d = (t - secs).abs();
        if d < best_d {
            best_d = d;
            best = t;
        }
    }
    best
}

/// 对单声道 PCM 做节拍检测
pub fn analyze_beats(
    samples: &[f32],
    sample_rate: u32,
    beats_per_bar: u8,
    hint_bpm: Option<f64>,
) -> BeatAnalysis {
    let beats_per_bar = beats_per_bar.max(1);
    if samples.is_empty() || sample_rate == 0 {
        return BeatAnalysis {
            bpm: hint_bpm.unwrap_or(120.0),
            beat_times: Vec::new(),
            measure_times: vec![0.0],
            beats_per_bar,
        };
    }

    let hop = (sample_rate / 100).max(1) as usize; // ~10 ms
    let onset = onset_envelope(samples, hop);
    let fps = f64::from(sample_rate) / hop as f64;

    let (bpm, phase_frames) = estimate_bpm_and_phase(&onset, fps, hint_bpm);
    let beat_period = 60.0 / bpm;
    let duration = samples.len() as f64 / f64::from(sample_rate);
    let phase_secs = phase_frames as f64 / fps;

    let mut beat_times = Vec::new();
    let mut t = phase_secs;
    // 向前补齐到 0 附近
    while t > 0.0 {
        t -= beat_period;
    }
    if t < -beat_period * 0.5 {
        t += beat_period;
    }
    while t < duration {
        if t >= -1e-6 {
            beat_times.push(t.max(0.0));
        }
        t += beat_period;
    }

    let mut measure_times = Vec::new();
    for (i, &bt) in beat_times.iter().enumerate() {
        if i % usize::from(beats_per_bar) == 0 {
            measure_times.push(bt);
        }
    }
    if measure_times.is_empty() {
        measure_times.push(0.0);
    }

    BeatAnalysis {
        bpm,
        beat_times,
        measure_times,
        beats_per_bar,
    }
}

/// 从乐谱取默认每小节拍数（首个 MasterBar，否则 4）
pub fn default_beats_per_bar(song: Option<&Song>) -> u8 {
    song.and_then(|s| s.master_bars.first())
        .map(|m| m.time_signature.numerator)
        .unwrap_or(4)
        .max(1)
}

fn onset_envelope(samples: &[f32], hop: usize) -> Vec<f32> {
    let mut env = Vec::with_capacity(samples.len() / hop + 1);
    let mut prev = 0.0_f32;
    let mut i = 0;
    while i < samples.len() {
        let end = (i + hop).min(samples.len());
        let mut energy = 0.0_f32;
        for &s in &samples[i..end] {
            energy += s * s;
        }
        energy = (energy / (end - i) as f32).sqrt();
        let flux = (energy - prev).max(0.0);
        env.push(flux);
        prev = energy;
        i += hop;
    }
    // 简单平滑
    if env.len() >= 3 {
        let mut smooth = env.clone();
        for i in 1..env.len() - 1 {
            smooth[i] = (env[i - 1] + env[i] * 2.0 + env[i + 1]) * 0.25;
        }
        return smooth;
    }
    env
}

fn estimate_bpm_and_phase(onset: &[f32], fps: f64, hint_bpm: Option<f64>) -> (f64, usize) {
    if onset.len() < 8 || fps <= 0.0 {
        return (hint_bpm.unwrap_or(120.0), 0);
    }

    let min_bpm = 70.0_f64;
    let max_bpm = 180.0_f64;
    let min_lag = ((60.0 / max_bpm) * fps).round() as usize;
    let max_lag = ((60.0 / min_bpm) * fps).round() as usize;
    let max_lag = max_lag.min(onset.len() / 2).max(min_lag + 1);

    let mut best_lag = ((60.0 / hint_bpm.unwrap_or(120.0)) * fps).round() as usize;
    best_lag = best_lag.clamp(min_lag, max_lag);
    let mut best_score = f32::MIN;

    for lag in min_lag..=max_lag {
        let mut sum = 0.0_f32;
        let mut n = 0u32;
        for i in lag..onset.len() {
            sum += onset[i] * onset[i - lag];
            n += 1;
        }
        if n == 0 {
            continue;
        }
        let mut score = sum / n as f32;
        // 偏向提示 BPM
        if let Some(hint) = hint_bpm {
            let bpm = 60.0 * fps / lag as f64;
            let diff = (bpm - hint).abs() / hint;
            score *= 1.0 + (1.0 - diff.min(1.0)) as f32 * 0.35;
        }
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    let bpm = 60.0 * fps / best_lag as f64;

    // 相位：在一个周期内找 onset 累加最大的起点
    let mut best_phase = 0usize;
    let mut best_phase_score = f32::MIN;
    for phase in 0..best_lag {
        let mut score = 0.0_f32;
        let mut t = phase;
        while t < onset.len() {
            score += onset[t];
            t += best_lag;
        }
        if score > best_phase_score {
            best_phase_score = score;
            best_phase = phase;
        }
    }

    (bpm.clamp(min_bpm, max_bpm), best_phase)
}

/// 为 UI 生成固定数量的波形峰值（0..1）
pub fn compute_peaks(samples: &[f32], buckets: usize) -> Vec<f32> {
    if samples.is_empty() || buckets == 0 {
        return vec![0.0; buckets.max(1)];
    }
    let mut peaks = vec![0.0_f32; buckets];
    let bucket_len = samples.len() as f64 / buckets as f64;
    for (i, peak) in peaks.iter_mut().enumerate() {
        let start = (i as f64 * bucket_len) as usize;
        let end = (((i + 1) as f64 * bucket_len) as usize).min(samples.len());
        let mut max_amp = 0.0_f32;
        for &s in &samples[start..end] {
            max_amp = max_amp.max(s.abs());
        }
        *peak = max_amp;
    }
    let max = peaks.iter().cloned().fold(0.0_f32, f32::max).max(1e-6);
    for p in &mut peaks {
        *p /= max;
    }
    peaks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click_track(sr: u32, bpm: f64, bars: u32, bpb: u8) -> Vec<f32> {
        let duration = bars as f64 * f64::from(bpb) * 60.0 / bpm;
        let n = (duration * f64::from(sr)) as usize;
        let mut samples = vec![0.0; n];
        let period = (60.0 / bpm * f64::from(sr)) as usize;
        let mut t = 0usize;
        while t < n {
            for i in 0..((sr as usize) / 200).min(n - t) {
                let env = 1.0 - i as f32 / 200.0;
                samples[t + i] = env * if t % (period * usize::from(bpb)) == 0 {
                    1.0
                } else {
                    0.6
                };
            }
            t += period;
        }
        samples
    }

    #[test]
    fn detects_bpm_near_120() {
        let sr = 22050;
        let samples = click_track(sr, 120.0, 8, 4);
        let analysis = analyze_beats(&samples, sr, 4, Some(120.0));
        assert!(
            (analysis.bpm - 120.0).abs() < 8.0,
            "bpm={}",
            analysis.bpm
        );
        assert!(!analysis.measure_times.is_empty());
        assert!(analysis.beat_times.len() >= 16);
    }

    #[test]
    fn peaks_normalized() {
        let peaks = compute_peaks(&[0.0, 0.5, -1.0, 0.25], 4);
        assert_eq!(peaks.len(), 4);
        assert!((peaks.iter().cloned().fold(0.0_f32, f32::max) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn score_timeline_measure_lookup_and_snap() {
        let mut song = bassoxide_core::song::Song::default();
        song.tempo = 120;
        song.master_bars.push(Default::default());
        song.master_bars.push(Default::default());
        let tl = score_timeline(&song);
        assert!(tl.duration_secs > 0.0);
        assert!(tl.measure_times.len() >= 3);
        let mid = score_secs_in_measure(&tl, 0, 0.5);
        assert!(mid > 0.0 && mid < tl.measure_times[1]);
        let (m, frac) = measure_at_score_secs(&tl, mid);
        assert_eq!(m, 0);
        assert!((frac - 0.5).abs() < 1e-6);
        let snapped = snap_to_nearest_beat(&tl, mid, 1.0);
        assert!(tl.beat_times.iter().any(|t| (*t - snapped).abs() < 1e-9));
    }
}
