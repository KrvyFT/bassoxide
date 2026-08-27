//! Bassoxide 工程文件 `.bso`：二进制容器，内嵌谱面（bincode）与原始音频字节。
//!
//! 布局（小端）：
//! ```text
//! magic[4] = b"BSO1"
//! version: u16 = 1
//! flags: u16 = 0
//! 重复 chunk:
//!   tag[4]  (META / SONG / AUDI / PCMF)
//!   size: u64
//!   data[size]
//! ```

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use bassoxide_core::song::Song;
use bassoxide_layout::PaperSize;
use serde::{Deserialize, Serialize};

use crate::state::{AppState, CursorPosition, ScorePrefs};

pub const BSO_MAGIC: &[u8; 4] = b"BSO1";
pub const BSO_VERSION: u16 = 1;

const TAG_META: &[u8; 4] = b"META";
const TAG_SONG: &[u8; 4] = b"SONG";
const TAG_AUDI: &[u8; 4] = b"AUDI"; // 原始音频文件字节
const TAG_PCMF: &[u8; 4] = b"PCMF"; // 后备：单声道 f32 PCM

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BsoCursor {
    pub track: usize,
    pub measure: usize,
    pub beat: usize,
    pub string: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BsoScorePrefs {
    pub font_size: f32,
    pub line_spacing: f32,
    pub row_spacing: f32,
    pub measures_per_line: u8,
    pub paper_size: PaperSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BsoMeta {
    pub selected_track: usize,
    pub zoom_factor: f32,
    pub scroll_y: f32,
    pub cursor: BsoCursor,
    pub score_prefs: BsoScorePrefs,
    pub is_light_theme: bool,
    pub playback_rate: f32,
    pub loop_a: Option<f64>,
    pub loop_b: Option<f64>,
    pub loop_enabled: bool,
    pub metronome_enabled: bool,
    pub audio_sync_offset_secs: f64,
    pub audio_pixels_per_second: f32,
    pub audio_view_start_secs: f64,
}

/// 解析后的工程内容（尚未应用到 AppState 的音频解码）
#[derive(Debug, Clone)]
pub struct BsoLoaded {
    pub song: Song,
    pub meta: BsoMeta,
    /// 原始音频文件字节 + 文件名提示
    pub audio_file: Option<(String, Vec<u8>)>,
    /// 若无 AUDI 则可能有 PCM 后备
    pub pcm: Option<(u32, Vec<f32>)>,
}

fn write_chunk<W: Write>(w: &mut W, tag: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    w.write_all(tag)?;
    w.write_all(&(data.len() as u64).to_le_bytes())?;
    w.write_all(data)?;
    Ok(())
}

fn read_exact_vec(r: &mut impl Read, n: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// 从 AppState 写出 `.bso`（谱面 bincode + 音频原始字节）
pub fn save_bso(path: &Path, state: &AppState) -> Result<(), String> {
    let song = state
        .song
        .as_ref()
        .ok_or_else(|| "没有可保存的乐谱".to_string())?;

    let meta = BsoMeta {
        selected_track: state.selected_track,
        zoom_factor: state.zoom_factor,
        scroll_y: state.scroll_y,
        cursor: BsoCursor {
            track: state.cursor.track,
            measure: state.cursor.measure,
            beat: state.cursor.beat,
            string: state.cursor.string as u8,
        },
        score_prefs: BsoScorePrefs {
            font_size: state.score_prefs.font_size,
            line_spacing: state.score_prefs.line_spacing,
            row_spacing: state.score_prefs.row_spacing,
            measures_per_line: state.score_prefs.measures_per_line,
            paper_size: state.score_prefs.paper_size,
        },
        is_light_theme: state.is_light_theme,
        playback_rate: state.playback_rate,
        loop_a: state.loop_a,
        loop_b: state.loop_b,
        loop_enabled: state.loop_enabled,
        metronome_enabled: state.metronome_enabled,
        audio_sync_offset_secs: state
            .audio_track
            .as_ref()
            .map(|t| t.sync_offset_secs)
            .unwrap_or(0.0),
        audio_pixels_per_second: state
            .audio_track
            .as_ref()
            .map(|t| t.pixels_per_second)
            .unwrap_or(80.0),
        audio_view_start_secs: state
            .audio_track
            .as_ref()
            .map(|t| t.view_start_secs)
            .unwrap_or(0.0),
    };

    let meta_bytes = bincode::serialize(&meta).map_err(|e| e.to_string())?;
    let song_bytes = bincode::serialize(song).map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(64 + meta_bytes.len() + song_bytes.len());
    out.write_all(BSO_MAGIC).map_err(|e| e.to_string())?;
    out.write_all(&BSO_VERSION.to_le_bytes())
        .map_err(|e| e.to_string())?;
    out.write_all(&0u16.to_le_bytes()).map_err(|e| e.to_string())?;
    write_chunk(&mut out, TAG_META, &meta_bytes).map_err(|e| e.to_string())?;
    write_chunk(&mut out, TAG_SONG, &song_bytes).map_err(|e| e.to_string())?;

    if let Some(track) = &state.audio_track {
        if let Some(raw) = &track.source_bytes {
            let name = track
                .source_name
                .clone()
                .unwrap_or_else(|| "audio.bin".into());
            let name_bytes = name.as_bytes();
            let mut audi = Vec::with_capacity(2 + name_bytes.len() + raw.len());
            audi.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            audi.extend_from_slice(name_bytes);
            audi.extend_from_slice(raw);
            write_chunk(&mut out, TAG_AUDI, &audi).map_err(|e| e.to_string())?;
        } else {
            // 后备：写入 PCM f32
            let mut pcm = Vec::with_capacity(4 + 8 + track.samples.len() * 4);
            pcm.extend_from_slice(&track.sample_rate.to_le_bytes());
            pcm.extend_from_slice(&(track.samples.len() as u64).to_le_bytes());
            for s in track.samples.iter() {
                pcm.extend_from_slice(&s.to_le_bytes());
            }
            write_chunk(&mut out, TAG_PCMF, &pcm).map_err(|e| e.to_string())?;
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(())
}

/// 读取 `.bso` 二进制工程
pub fn load_bso(path: &Path) -> Result<BsoLoaded, String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    load_bso_bytes(&data)
}

pub fn load_bso_bytes(data: &[u8]) -> Result<BsoLoaded, String> {
    if data.len() < 8 {
        return Err("文件过短".into());
    }
    if &data[0..4] != BSO_MAGIC {
        return Err("不是有效的 .bso 文件（magic）".into());
    }
    let version = u16::from_le_bytes([data[4], data[5]]);
    if version != BSO_VERSION {
        return Err(format!("不支持的 .bso 版本: {version}"));
    }

    let mut cur = Cursor::new(&data[8..]);
    let mut meta: Option<BsoMeta> = None;
    let mut song: Option<Song> = None;
    let mut audio_file: Option<(String, Vec<u8>)> = None;
    let mut pcm: Option<(u32, Vec<f32>)> = None;

    loop {
        let mut tag = [0u8; 4];
        match cur.read_exact(&mut tag) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.to_string()),
        }
        let mut sz_buf = [0u8; 8];
        cur.read_exact(&mut sz_buf).map_err(|e| e.to_string())?;
        let size = u64::from_le_bytes(sz_buf) as usize;
        let chunk = read_exact_vec(&mut cur, size).map_err(|e| e.to_string())?;

        match &tag {
            t if t == TAG_META => {
                meta = Some(bincode::deserialize(&chunk).map_err(|e| e.to_string())?);
            }
            t if t == TAG_SONG => {
                song = Some(bincode::deserialize(&chunk).map_err(|e| e.to_string())?);
            }
            t if t == TAG_AUDI => {
                if chunk.len() < 2 {
                    return Err("AUDI 块损坏".into());
                }
                let name_len = u16::from_le_bytes([chunk[0], chunk[1]]) as usize;
                if chunk.len() < 2 + name_len {
                    return Err("AUDI 块文件名长度无效".into());
                }
                let name = String::from_utf8_lossy(&chunk[2..2 + name_len]).into_owned();
                let bytes = chunk[2 + name_len..].to_vec();
                audio_file = Some((name, bytes));
            }
            t if t == TAG_PCMF => {
                if chunk.len() < 12 {
                    return Err("PCMF 块损坏".into());
                }
                let sr = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
                let n = u64::from_le_bytes(chunk[4..12].try_into().unwrap()) as usize;
                if chunk.len() < 12 + n * 4 {
                    return Err("PCMF 采样数不匹配".into());
                }
                let mut samples = Vec::with_capacity(n);
                for i in 0..n {
                    let o = 12 + i * 4;
                    samples.push(f32::from_le_bytes(chunk[o..o + 4].try_into().unwrap()));
                }
                pcm = Some((sr, samples));
            }
            _ => {
                // 未知块跳过，便于向前兼容
            }
        }
    }

    let song = song.ok_or_else(|| "缺少 SONG 块".to_string())?;
    let meta = meta.unwrap_or(BsoMeta {
        selected_track: 0,
        zoom_factor: 1.0,
        scroll_y: 0.0,
        cursor: BsoCursor {
            track: 0,
            measure: 0,
            beat: 0,
            string: 1,
        },
        score_prefs: BsoScorePrefs {
            font_size: 13.0,
            line_spacing: 10.0,
            row_spacing: 10.0,
            measures_per_line: 4,
            paper_size: PaperSize::A4,
        },
        is_light_theme: true,
        playback_rate: 1.0,
        loop_a: None,
        loop_b: None,
        loop_enabled: false,
        metronome_enabled: false,
        audio_sync_offset_secs: 0.0,
        audio_pixels_per_second: 80.0,
        audio_view_start_secs: 0.0,
    });

    Ok(BsoLoaded {
        song,
        meta,
        audio_file,
        pcm,
    })
}

/// 将已解码/装载的工程状态应用到 AppState（不含耗时音频解码）
pub fn apply_meta_and_song(state: &mut AppState, loaded: &BsoLoaded, project_path: PathBuf) {
    state.project_path = Some(project_path);
    state.file_path = state
        .project_path
        .as_ref()
        .map(|p| p.display().to_string());
    state.score_prefs = ScorePrefs {
        font_size: loaded.meta.score_prefs.font_size,
        line_spacing: loaded.meta.score_prefs.line_spacing,
        row_spacing: loaded.meta.score_prefs.row_spacing,
        measures_per_line: loaded.meta.score_prefs.measures_per_line,
        paper_size: loaded.meta.score_prefs.paper_size,
    };
    state.selected_track = loaded.meta.selected_track;
    state.zoom_factor = loaded.meta.zoom_factor;
    state.scroll_y = loaded.meta.scroll_y;
    state.cursor = CursorPosition {
        track: loaded.meta.cursor.track,
        measure: loaded.meta.cursor.measure,
        beat: loaded.meta.cursor.beat,
        string: loaded.meta.cursor.string as usize,
    };
    state.set_light_theme(loaded.meta.is_light_theme);
    state.playback_rate = loaded.meta.playback_rate.clamp(0.5, 1.5);
    state.loop_a = loaded.meta.loop_a;
    state.loop_b = loaded.meta.loop_b;
    state.loop_enabled = loaded.meta.loop_enabled;
    state.metronome_enabled = loaded.meta.metronome_enabled;
    state.load_song(loaded.song.clone(), state.file_path.clone());
    // load_song 会重置 cursor / playback sync；再写回
    state.cursor = CursorPosition {
        track: loaded.meta.cursor.track,
        measure: loaded.meta.cursor.measure,
        beat: loaded.meta.cursor.beat,
        string: loaded.meta.cursor.string as usize,
    };
    state.selected_track = loaded.meta.selected_track;
    state.zoom_factor = loaded.meta.zoom_factor;
    state.scroll_y = loaded.meta.scroll_y;
    state.playback_rate = loaded.meta.playback_rate.clamp(0.5, 1.5);
    state.loop_a = loaded.meta.loop_a;
    state.loop_b = loaded.meta.loop_b;
    state.loop_enabled = loaded.meta.loop_enabled;
    state.metronome_enabled = loaded.meta.metronome_enabled;
    state.apply_score_prefs();
    state.sync_playback_tools_to_player();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bassoxide_core::measure::{MasterBar, Measure};
    use bassoxide_core::song::Song;
    use bassoxide_core::track::{Track, Tuning};

    fn tiny_song() -> Song {
        let mut song = Song::default();
        song.info.title = "t".into();
        song.tempo = 120;
        song.master_bars.push(MasterBar::default());
        let mut track = Track::default();
        track.tuning = Tuning::standard_guitar();
        track.measures.push(Measure::default());
        song.tracks.push(track);
        song
    }

    #[test]
    fn bso_roundtrip_song_and_pcm() {
        let song = tiny_song();
        let meta = BsoMeta {
            selected_track: 0,
            zoom_factor: 1.2,
            scroll_y: 10.0,
            cursor: BsoCursor {
                track: 0,
                measure: 0,
                beat: 0,
                string: 2,
            },
            score_prefs: BsoScorePrefs {
                font_size: 14.0,
                line_spacing: 11.0,
                row_spacing: 12.0,
                measures_per_line: 4,
                paper_size: PaperSize::A4,
            },
            is_light_theme: true,
            playback_rate: 0.8,
            loop_a: Some(1.0),
            loop_b: Some(2.0),
            loop_enabled: true,
            metronome_enabled: true,
            audio_sync_offset_secs: 0.5,
            audio_pixels_per_second: 90.0,
            audio_view_start_secs: 0.0,
        };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        let song_bytes = bincode::serialize(&song).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(BSO_MAGIC);
        out.extend_from_slice(&BSO_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        write_chunk(&mut out, TAG_META, &meta_bytes).unwrap();
        write_chunk(&mut out, TAG_SONG, &song_bytes).unwrap();
        // AUDI: fake raw bytes
        let mut audi = Vec::new();
        let name = b"x.flac";
        audi.extend_from_slice(&(name.len() as u16).to_le_bytes());
        audi.extend_from_slice(name);
        audi.extend_from_slice(b"FAKEAUDIO");
        write_chunk(&mut out, TAG_AUDI, &audi).unwrap();

        let loaded = load_bso_bytes(&out).unwrap();
        assert_eq!(loaded.song.info.title, "t");
        assert!((loaded.meta.playback_rate - 0.8).abs() < 1e-6);
        assert_eq!(loaded.meta.loop_a, Some(1.0));
        let (n, bytes) = loaded.audio_file.unwrap();
        assert_eq!(n, "x.flac");
        assert_eq!(bytes, b"FAKEAUDIO");
    }

    #[test]
    fn bso_embeds_real_flac_bytes() {
        let flac = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/click_120bpm.flac");
        if !flac.exists() {
            eprintln!("skip: missing {}", flac.display());
            return;
        }
        let raw = fs::read(&flac).unwrap();
        let song = tiny_song();
        let song_bytes = bincode::serialize(&song).unwrap();
        let meta = BsoMeta {
            selected_track: 0,
            zoom_factor: 1.0,
            scroll_y: 0.0,
            cursor: BsoCursor {
                track: 0,
                measure: 0,
                beat: 0,
                string: 1,
            },
            score_prefs: BsoScorePrefs {
                font_size: 13.0,
                line_spacing: 10.0,
                row_spacing: 10.0,
                measures_per_line: 4,
                paper_size: PaperSize::A4,
            },
            is_light_theme: true,
            playback_rate: 1.0,
            loop_a: None,
            loop_b: None,
            loop_enabled: false,
            metronome_enabled: false,
            audio_sync_offset_secs: 0.25,
            audio_pixels_per_second: 80.0,
            audio_view_start_secs: 0.0,
        };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(BSO_MAGIC);
        out.extend_from_slice(&BSO_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        write_chunk(&mut out, TAG_META, &meta_bytes).unwrap();
        write_chunk(&mut out, TAG_SONG, &song_bytes).unwrap();
        let name = b"click_120bpm.flac";
        let mut audi = Vec::with_capacity(2 + name.len() + raw.len());
        audi.extend_from_slice(&(name.len() as u16).to_le_bytes());
        audi.extend_from_slice(name);
        audi.extend_from_slice(&raw);
        write_chunk(&mut out, TAG_AUDI, &audi).unwrap();

        let loaded = load_bso_bytes(&out).unwrap();
        let (n, bytes) = loaded.audio_file.unwrap();
        assert_eq!(n, "click_120bpm.flac");
        assert_eq!(bytes, raw);
        assert!((loaded.meta.audio_sync_offset_secs - 0.25).abs() < 1e-9);
        // 魔数仍为 FLAC
        assert_eq!(&bytes[0..4], b"fLaC");
    }

    #[test]
    fn save_bso_roundtrip_via_app_state() {
        use crate::state::AppState;
        use crate::ui::audio_track::AudioTrack;
        use std::sync::Arc;

        let flac = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/click_120bpm.flac");
        if !flac.exists() {
            eprintln!("skip: missing {}", flac.display());
            return;
        }
        let raw = fs::read(&flac).unwrap();
        let mut state = AppState::default();
        state.load_song(tiny_song(), Some("demo.gp5".into()));
        let track = AudioTrack {
            path: "click_120bpm.flac".into(),
            samples: Arc::new(vec![0.0; 100]),
            sample_rate: 44100,
            duration_secs: 100.0 / 44100.0,
            peaks: vec![0.1],
            analysis: bassoxide_audio::BeatAnalysis {
                bpm: 120.0,
                beat_times: vec![],
                measure_times: vec![],
                beats_per_bar: 4,
            },
            sync_offset_secs: 0.1,
            pixels_per_second: 80.0,
            view_start_secs: 0.0,
            source_bytes: Some(raw.clone()),
            source_name: Some("click_120bpm.flac".into()),
        };
        state.audio_track = Some(track);
        state.playback_rate = 0.9;
        state.metronome_enabled = true;

        let dir = std::env::temp_dir().join("bassoxide_bso_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("roundtrip.bso");
        save_bso(&path, &state).expect("save");
        let loaded = load_bso(&path).expect("load");
        assert_eq!(loaded.song.info.title, "t");
        assert!((loaded.meta.playback_rate - 0.9).abs() < 1e-6);
        assert!(loaded.meta.metronome_enabled);
        let (name, bytes) = loaded.audio_file.expect("audi");
        assert_eq!(name, "click_120bpm.flac");
        assert_eq!(bytes, raw);
        assert_eq!(&bytes[0..4], b"fLaC");
        let size = fs::metadata(&path).unwrap().len();
        assert!(size > raw.len() as u64);
    }
}
