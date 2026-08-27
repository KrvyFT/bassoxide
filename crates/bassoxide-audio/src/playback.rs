//! 音频回放控制与 cpal 接口

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info};

use crate::error::{AudioError, Result};
use crate::synth::Synth;
use bassoxide_core::song::Song;

/// MIDI 事件
#[derive(Debug, Clone)]
pub enum MidiEvent {
    NoteOn { tick: u32, channel: i32, key: i32, velocity: i32 },
    NoteOff { tick: u32, channel: i32, key: i32 },
    ProgramChange { tick: u32, channel: i32, program: i32 },
    BankSelect { tick: u32, channel: i32, bank: i32 },
}

impl MidiEvent {
    pub fn tick(&self) -> u32 {
        match self {
            MidiEvent::NoteOn { tick, .. } => *tick,
            MidiEvent::NoteOff { tick, .. } => *tick,
            MidiEvent::ProgramChange { tick, .. } => *tick,
            MidiEvent::BankSelect { tick, .. } => *tick,
        }
    }
}

/// 播放器引擎
pub struct AudioEngine {
    _stream: Option<Stream>,
    pub synth: Arc<Synth>,
    play_state: Arc<Mutex<PlayState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

struct PlayState {
    status: PlaybackStatus,
    current_tick: u32,
    bpm: u16,
    events: Vec<MidiEvent>,
    event_idx: usize,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| AudioError::DeviceError("No output device available".to_string()))?;
            
        let mut supported_configs_range = device.supported_output_configs()
            .map_err(|e| AudioError::DeviceError(format!("Error while querying configs: {e}")))?;
            
        let config_range = supported_configs_range.next()
            .ok_or_else(|| AudioError::DeviceError("No supported config".to_string()))?;
            
        let target_sr = cpal::SampleRate(44100);
        let config = if config_range.min_sample_rate() <= target_sr && target_sr <= config_range.max_sample_rate() {
            config_range.with_sample_rate(target_sr)
        } else {
            let max_allowed = config_range.max_sample_rate().0.min(192000);
            config_range.with_sample_rate(cpal::SampleRate(max_allowed))
        }.config();
            
        let sample_rate = config.sample_rate.0 as i32;
        let synth = Arc::new(Synth::new(sample_rate)?);
        
        let play_state = Arc::new(Mutex::new(PlayState {
            status: PlaybackStatus::Stopped,
            current_tick: 0,
            bpm: 120,
            events: Vec::new(),
            event_idx: 0,
        }));

        let stream = Self::start_stream(&device, &config, synth.clone())?;
        stream.play().map_err(|e| AudioError::DeviceError(format!("Failed to play stream: {e}")))?;

        // 启动音序器后台线程
        Self::start_sequencer(play_state.clone(), synth.clone());

        Ok(Self {
            _stream: Some(stream),
            synth,
            play_state,
        })
    }
    pub fn load_soundfont(&self, path: &str) -> Result<()> {
        self.synth.load_soundfont(path)
    }

    pub fn get_presets(&self) -> Vec<(i32, i32, String)> {
        self.synth.get_presets()
    }

    fn start_stream(
        device: &cpal::Device,
        config: &StreamConfig,
        synth: Arc<Synth>,
    ) -> Result<Stream> {
        let channels = config.channels as usize;
        let mut left_buf = vec![0.0f32; 1024];
        let mut right_buf = vec![0.0f32; 1024];

        let err_fn = |err| error!("Audio stream error: {}", err);

        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frame_count = data.len() / channels;
                if left_buf.len() < frame_count {
                    left_buf.resize(frame_count, 0.0);
                    right_buf.resize(frame_count, 0.0);
                }
                
                synth.render(&mut left_buf[..frame_count], &mut right_buf[..frame_count]);
                
                for (i, frame) in data.chunks_mut(channels).enumerate() {
                    frame[0] = left_buf[i];
                    if channels > 1 {
                        frame[1] = right_buf[i];
                    }
                }
            },
            err_fn,
            None,
        ).map_err(|e| AudioError::DeviceError(format!("Failed to build stream: {e}")))?;

        Ok(stream)
    }
    
    fn compile_song(song: &Song) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let has_solo = song.tracks.iter().any(|t| t.is_solo);
        
        for (track_idx, track) in song.tracks.iter().enumerate() {
            if has_solo && !track.is_solo {
                continue;
            }
            if !has_solo && track.is_muted {
                continue;
            }
            let channel = Self::synth_channel(track, track_idx);
            
            // 为该轨道压入初始音色切换事件（包含 Bank 和 Program）
            events.push(MidiEvent::BankSelect {
                tick: 0,
                channel,
                bank: track.midi_bank as i32,
            });
            events.push(MidiEvent::ProgramChange {
                tick: 0,
                channel,
                program: track.midi_program as i32,
            });
            
            let mut current_tick = 0;
            
            for (m_idx, measure) in track.measures.iter().enumerate() {
                let mut measure_tick = 0;
                
                // 暂时只处理 Voice 0
                for beat in &measure.voices[0].beats {
                    if !beat.is_empty() {
                        for note in &beat.notes {
                            if note.is_dead() { continue; }
                            
                            // Note On
                            events.push(MidiEvent::NoteOn {
                                tick: current_tick + measure_tick,
                                channel,
                                key: note.midi_note as i32,
                                velocity: note.velocity as i32,
                            });
                            
                            // Note Off
                            events.push(MidiEvent::NoteOff {
                                tick: current_tick + measure_tick + beat.ticks(),
                                channel,
                                key: note.midi_note as i32,
                            });
                        }
                    }
                    measure_tick += beat.ticks();
                }
                
                // 小节长度，获取拍号
                if let Some(master) = song.master_bar(m_idx) {
                    current_tick += master.time_signature.measure_ticks();
                } else {
                    current_tick += 960 * 4; // 降级 4/4
                }
            }
        }
        
        events.sort_by_key(|e| e.tick());
        events
    }

    fn synth_channel(track: &bassoxide_core::track::Track, track_idx: usize) -> i32 {
        if track.is_percussion {
            return i32::from(bassoxide_core::midi::PERCUSSION_CHANNEL);
        }
        // 每条旋律轨独立通道，避开 GM 鼓通道 9；音色号仍来自文件。
        let mut ch = (track_idx as i32) % 15;
        if ch >= i32::from(bassoxide_core::midi::PERCUSSION_CHANNEL) {
            ch += 1;
        }
        ch
    }
    
    /// 启动后台 sequencer 线程
    fn start_sequencer(state: Arc<Mutex<PlayState>>, synth: Arc<Synth>) {
        thread::spawn(move || {
            let mut last_time = Instant::now();
            
            loop {
                thread::sleep(Duration::from_millis(10));
                
                let mut st = state.lock().unwrap();
                if st.status != PlaybackStatus::Playing {
                    last_time = Instant::now();
                    continue;
                }
                
                // 计算流逝的 tick (假设 ppq=960)
                let now = Instant::now();
                let elapsed = now.duration_since(last_time);
                last_time = now;
                
                let bps = st.bpm as f32 / 60.0;
                let ticks_per_sec = bps * 960.0;
                let delta_ticks = (elapsed.as_secs_f32() * ticks_per_sec) as u32;
                
                st.current_tick += delta_ticks;
                let current = st.current_tick;
                
                // 发送落入当前时间窗口的事件
                while st.event_idx < st.events.len() && st.events[st.event_idx].tick() <= current {
                    match &st.events[st.event_idx] {
                        MidiEvent::NoteOn { channel, key, velocity, .. } => {
                            synth.note_on(*channel, *key, *velocity);
                        }
                        MidiEvent::NoteOff { channel, key, .. } => {
                            synth.note_off(*channel, *key);
                        }
                        MidiEvent::BankSelect { channel, bank, .. } => {
                            tracing::info!("Sending BankSelect: channel={}, bank={}", channel, bank);
                            if let Ok(mut s) = synth.synth.lock() {
                                s.process_midi_message(*channel, 0xB0, 0x00, *bank);
                            }
                        }
                        MidiEvent::ProgramChange { channel, program, .. } => {
                            tracing::info!("Sending ProgramChange: channel={}, program={}", channel, program);
                            synth.program_change(*channel, *program);
                        }
                    }
                    st.event_idx += 1;
                }
                
                // 播放结束
                if st.event_idx >= st.events.len() {
                    st.status = PlaybackStatus::Stopped;
                    st.current_tick = 0;
                    st.event_idx = 0;
                }
            }
        });
    }

    pub fn play(&self, song: &Song) {
        let mut state = self.play_state.lock().unwrap();
        if state.status == PlaybackStatus::Stopped {
            state.events = Self::compile_song(song);
            state.bpm = song.tempo;
            state.current_tick = 0;
            state.event_idx = 0;
        }
        state.status = PlaybackStatus::Playing;
    }
    
    pub fn pause(&self) {
        let mut state = self.play_state.lock().unwrap();
        state.status = PlaybackStatus::Paused;
        self.synth.reset();
    }
    
    pub fn stop(&self) {
        let mut state = self.play_state.lock().unwrap();
        state.status = PlaybackStatus::Stopped;
        state.current_tick = 0;
        self.synth.reset();
    }
    
    pub fn reload_song(&self, song: &Song) {
        let mut state = self.play_state.lock().unwrap();
        let was_playing = state.status == PlaybackStatus::Playing;
        
        state.events = Self::compile_song(song);
        
        // 当切换独奏静音时，如果当前正在播放，需要让 event_idx 移动到 current_tick 处
        if was_playing {
            self.synth.reset();
            let cur = state.current_tick;
            
            // 将所有在当前时间点之前的配置事件立即发出，确保重置后的合成器能恢复正确的音色
            for e in &state.events {
                if e.tick() > cur {
                    break;
                }
                match e {
                    MidiEvent::BankSelect { channel, bank, .. } => {
                        if let Ok(mut s) = self.synth.synth.lock() {
                            s.process_midi_message(*channel, 0xB0, 0x00, *bank);
                        }
                    }
                    MidiEvent::ProgramChange { channel, program, .. } => {
                        self.synth.program_change(*channel, *program);
                    }
                    _ => {} // NoteOn/NoteOff 不补发
                }
            }
            
            state.event_idx = state.events.iter().position(|e| e.tick() > cur).unwrap_or(state.events.len());
        } else {
            state.event_idx = 0;
        }
    }
    
    pub fn status(&self) -> PlaybackStatus {
        self.play_state.lock().unwrap().status
    }
}
