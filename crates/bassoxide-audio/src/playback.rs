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
pub struct MidiEvent {
    pub tick: u32,
    pub is_note_on: bool,
    pub channel: i32,
    pub key: i32,
    pub velocity: i32,
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
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| AudioError::DeviceError("No output device available".to_string()))?;
            
        let mut supported_configs_range = device.supported_output_configs()
            .map_err(|e| AudioError::DeviceError(format!("Error while querying configs: {e}")))?;
            
        let config = supported_configs_range.next()
            .ok_or_else(|| AudioError::DeviceError("No supported config".to_string()))?
            .with_max_sample_rate()
            .config();
            
        let sample_rate = config.sample_rate.0 as i32;
        let synth = Arc::new(Synth::new(sample_rate)?);
        
        let play_state = Arc::new(Mutex::new(PlayState {
            status: PlaybackStatus::Stopped,
            current_tick: 0,
            bpm: 120,
            events: Vec::new(),
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
    
    /// 编译 Song 到事件流
    fn compile_song(song: &Song) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        // 假设 PPQ = 960 (根据 core/types::Duration::ticks 计算)
        
        for track in &song.tracks {
            let channel = track.midi_channel as i32;
            let mut current_tick = 0;
            
            for (m_idx, measure) in track.measures.iter().enumerate() {
                let mut measure_tick = 0;
                
                // 暂时只处理 Voice 0
                for beat in &measure.voices[0].beats {
                    if !beat.is_empty() {
                        for note in &beat.notes {
                            if note.is_dead() { continue; }
                            
                            // Note On
                            events.push(MidiEvent {
                                tick: current_tick + measure_tick,
                                is_note_on: true,
                                channel,
                                key: note.midi_note as i32,
                                velocity: note.velocity as i32,
                            });
                            
                            // Note Off
                            events.push(MidiEvent {
                                tick: current_tick + measure_tick + beat.ticks(),
                                is_note_on: false,
                                channel,
                                key: note.midi_note as i32,
                                velocity: 0,
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
        
        events.sort_by_key(|e| e.tick);
        events
    }
    
    /// 启动后台 sequencer 线程
    fn start_sequencer(state: Arc<Mutex<PlayState>>, synth: Arc<Synth>) {
        thread::spawn(move || {
            let mut last_tick = 0;
            let mut last_time = Instant::now();
            let mut event_idx = 0;
            
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
                while event_idx < st.events.len() && st.events[event_idx].tick <= current {
                    let ev = &st.events[event_idx];
                    if ev.is_note_on {
                        synth.note_on(ev.channel, ev.key, ev.velocity);
                    } else {
                        synth.note_off(ev.channel, ev.key);
                    }
                    event_idx += 1;
                }
                
                // 播放结束
                if event_idx >= st.events.len() {
                    st.status = PlaybackStatus::Stopped;
                    st.current_tick = 0;
                    event_idx = 0;
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
        }
        state.status = PlaybackStatus::Playing;
    }
    
    pub fn pause(&self) {
        let mut state = self.play_state.lock().unwrap();
        state.status = PlaybackStatus::Paused;
    }
    
    pub fn stop(&self) {
        let mut state = self.play_state.lock().unwrap();
        state.status = PlaybackStatus::Stopped;
        state.current_tick = 0;
        self.synth.reset();
    }
    
    pub fn status(&self) -> PlaybackStatus {
        self.play_state.lock().unwrap().status
    }
}
