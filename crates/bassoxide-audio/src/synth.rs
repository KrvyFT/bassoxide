//! 基于 rustysynth 的 MIDI 合成器

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

use crate::error::{AudioError, Result};

/// 内置乐队音源路径（GeneralUser GS，覆盖吉他/贝斯/键盘/鼓）
pub const DEFAULT_SOUNDFONT: &str = "assets/Bassoxide_Band.sf2";

/// 候选路径：工作区相对路径 → FluidR3 系统包
fn resolve_soundfont_path() -> Result<String> {
    let candidates = [
        DEFAULT_SOUNDFONT,
        "/usr/share/sounds/sf2/FluidR3_GM.sf2",
        "/usr/share/sounds/sf2/default-GM.sf2",
    ];
    for path in candidates {
        if Path::new(path).is_file() {
            return Ok(path.to_string());
        }
    }
    Err(AudioError::SoundFontError(format!(
        "未找到音源文件。请运行安装脚本下载 {DEFAULT_SOUNDFONT}，或安装 fluid-soundfont-gm"
    )))
}

pub struct Synth {
    pub synth: Arc<Mutex<Synthesizer>>,
    pub soundfont: RwLock<Arc<SoundFont>>,
    pub sample_rate: i32,
}

impl Synth {
    pub fn new(sample_rate: i32) -> Result<Self> {
        let sf2_path = resolve_soundfont_path()?;
        let mut sf2_file = File::open(&sf2_path)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to open {sf2_path}: {e}")))?;

        let soundfont = Arc::new(
            SoundFont::new(&mut sf2_file)
                .map_err(|e| AudioError::SoundFontError(format!("Failed to parse SoundFont: {e}")))?,
        );

        let settings = SynthesizerSettings::new(sample_rate);
        let synthesizer = Synthesizer::new(&soundfont, &settings)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to create synthesizer: {e}")))?;

        info!("Loaded SoundFont: {sf2_path}");
        Ok(Self {
            synth: Arc::new(Mutex::new(synthesizer)),
            soundfont: RwLock::new(soundfont),
            sample_rate,
        })
    }

    pub fn load_soundfont(&self, path: &str) -> Result<()> {
        let mut sf2_file = File::open(path)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to open {path}: {e}")))?;

        let soundfont = Arc::new(
            SoundFont::new(&mut sf2_file)
                .map_err(|e| AudioError::SoundFontError(format!("Failed to parse SoundFont: {e}")))?,
        );

        let settings = SynthesizerSettings::new(self.sample_rate);
        let new_synth = Synthesizer::new(&soundfont, &settings)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to create synthesizer: {e}")))?;

        if let Ok(mut s) = self.synth.lock() {
            *s = new_synth;
        }
        if let Ok(mut sf) = self.soundfont.write() {
            *sf = soundfont;
        }
        info!("Successfully loaded new SoundFont: {}", path);
        Ok(())
    }

    /// 获取当前加载的 SoundFont 中的所有预设 (bank, patch, name)
    pub fn get_presets(&self) -> Vec<(i32, i32, String)> {
        if let Ok(sf) = self.soundfont.read() {
            sf.get_presets()
                .iter()
                .map(|p| {
                    (
                        p.get_bank_number(),
                        p.get_patch_number(),
                        p.get_name().to_string(),
                    )
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// 发送 Note On
    pub fn note_on(&self, channel: i32, key: i32, velocity: i32) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.note_on(channel, key, velocity);
        }
    }

    /// 发送 Note Off
    pub fn note_off(&self, channel: i32, key: i32) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.note_off(channel, key);
        }
    }

    /// 发送 Program Change (音色切换)
    pub fn program_change(&self, channel: i32, program: i32) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.process_midi_message(channel, 0xC0, program, 0);
        }
    }

    /// 重置合成器
    pub fn reset(&self) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.reset();
        }
    }

    /// 渲染音频块
    pub fn render(&self, left: &mut [f32], right: &mut [f32]) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.render(left, right);
        }
    }
}
