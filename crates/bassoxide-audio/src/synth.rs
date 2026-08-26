//! 基于 rustysynth 的 MIDI 合成器

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::error::{AudioError, Result};

pub struct Synth {
    pub synth: Arc<Mutex<Synthesizer>>,
}

impl Synth {
    pub fn new(sample_rate: i32) -> Result<Self> {
        // 尝试加载 assets/Orchestra_HQ.sf2
        let sf2_path = "assets/Orchestra_HQ.sf2";
        
        let mut sf2_file = File::open(sf2_path)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to open {sf2_path}: {e}")))?;
            
        let soundfont = Arc::new(SoundFont::new(&mut sf2_file)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to parse SoundFont: {e}")))?);
            
        let settings = SynthesizerSettings::new(sample_rate);
        let synthesizer = Synthesizer::new(&soundfont, &settings)
            .map_err(|e| AudioError::SoundFontError(format!("Failed to create synthesizer: {e}")))?;
            
        info!("SoundFont loaded successfully, sample_rate={}", sample_rate);
            
        Ok(Self {
            synth: Arc::new(Mutex::new(synthesizer)),
        })
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
