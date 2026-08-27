//! 多格式音频解码（WAV / FLAC / MP3 / OGG 等）。

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{AudioError, Result};

/// 解码后的单声道 PCM
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    /// 归一化到 [-1, 1] 的单声道采样
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl DecodedAudio {
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / f64::from(self.sample_rate)
    }
}

/// 从路径解码为单声道 f32 PCM
pub fn decode_file(path: &Path) -> Result<DecodedAudio> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioError::DecodeError(e.to_string()))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError::DecodeError("未找到可解码音轨".into()))?
        .clone();

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| AudioError::DecodeError("缺少采样率".into()))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::DecodeError(e.to_string()))?;

    let mut mono = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::ResetRequired) => continue,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::IoError(_)) => break,
            Err(e) => return Err(AudioError::DecodeError(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => append_mono(&decoded, &mut mono),
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return Err(AudioError::DecodeError(e.to_string())),
        }
    }

    if mono.is_empty() {
        return Err(AudioError::DecodeError("音频为空".into()));
    }

    Ok(DecodedAudio {
        samples: mono,
        sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn decodes_flac_click_track() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/click_120bpm.flac");
        if !path.exists() {
            eprintln!("skip: missing {}", path.display());
            return;
        }
        let audio = decode_file(&path).expect("decode flac");
        assert!(audio.sample_rate >= 16000);
        assert!(audio.duration_secs() > 10.0);
    }

    #[test]
    fn decodes_mp3_click_track() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/click_120bpm.mp3");
        if !path.exists() {
            eprintln!("skip: missing {}", path.display());
            return;
        }
        let audio = decode_file(&path).expect("decode mp3");
        assert!(audio.duration_secs() > 10.0);
        let analysis = crate::beat::analyze_beats(&audio.samples, audio.sample_rate, 4, Some(120.0));
        assert!(
            (analysis.bpm - 120.0).abs() < 12.0,
            "bpm={}",
            analysis.bpm
        );
        assert!(analysis.measure_times.len() >= 4);
    }
}

fn append_mono(decoded: &AudioBufferRef<'_>, out: &mut Vec<f32>) {
    // 先转为平面 f32，再混缩，避免 symphonia 临时 planes 生命周期问题
    let planes_f32: Vec<Vec<f32>> = match decoded {
        AudioBufferRef::F32(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.to_vec())
            .collect(),
        AudioBufferRef::U8(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| (f32::from(s) - 128.0) / 128.0).collect())
            .collect(),
        AudioBufferRef::U16(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| {
                ch.iter()
                    .map(|&s| (f32::from(s) - 32768.0) / 32768.0)
                    .collect()
            })
            .collect(),
        AudioBufferRef::U24(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| {
                ch.iter()
                    .map(|s| s.inner() as f32 / 8_388_608.0 - 1.0)
                    .collect()
            })
            .collect(),
        AudioBufferRef::U32(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| (s as f32) / 2_147_483_648.0 - 1.0).collect())
            .collect(),
        AudioBufferRef::S8(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| f32::from(s) / 128.0).collect())
            .collect(),
        AudioBufferRef::S16(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| f32::from(s) / 32768.0).collect())
            .collect(),
        AudioBufferRef::S24(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|s| s.inner() as f32 / 8_388_608.0).collect())
            .collect(),
        AudioBufferRef::S32(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| (s as f32) / 2_147_483_648.0).collect())
            .collect(),
        AudioBufferRef::F64(buf) => buf
            .planes()
            .planes()
            .iter()
            .map(|ch| ch.iter().map(|&s| s as f32).collect())
            .collect(),
    };

    if planes_f32.is_empty() {
        return;
    }
    let frames = planes_f32[0].len();
    let n = planes_f32.len() as f32;
    out.reserve(frames);
    for i in 0..frames {
        let mut sum = 0.0_f32;
        for ch in &planes_f32 {
            sum += ch[i];
        }
        out.push(sum / n);
    }
}
