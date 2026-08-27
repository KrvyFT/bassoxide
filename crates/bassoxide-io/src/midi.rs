//! 标准 MIDI (.mid) 解析器

use bassoxide_core::beat::Beat;
use bassoxide_core::measure::{MasterBar, Measure};
use bassoxide_core::note::{Note, NoteType};
use bassoxide_core::song::{Song, SongInfo};
use bassoxide_core::track::Track;
use bassoxide_core::types::{Duration, NoteValue, TimeSignature};
use midly::{Smf, TrackEventKind};
use std::collections::BTreeMap;

use crate::error::{IoError, Result};

/// 解析 MIDI 文件
pub fn parse_midi(data: &[u8]) -> Result<Song> {
    let smf = Smf::parse(data)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid MIDI file: {e}")))?;

    let mut song = Song::default();
    song.info = SongInfo {
        title: "Imported MIDI".to_string(),
        ..Default::default()
    };
    
    // 解析 PPQ (Pulses Per Quarter)
    let ppq = match smf.header.timing {
        midly::Timing::Metrical(ticks_per_beat) => ticks_per_beat.as_int() as u32,
        _ => 480, // Fallback default
    };

    // 这里我们做最简单的单轨/多轨映射：不处理复杂的节奏切分，只按四分音符进行基本量化
    // 这只是一个基础骨架实现，对于真实世界的复杂 MIDI 需要高级 Quantizer。
    
    // 我们假设 4/4 拍
    let mut master_bar = MasterBar::default();
    master_bar.time_signature = TimeSignature {
        numerator: 4,
        denominator: NoteValue::Quarter,
    };
    song.master_bars.push(master_bar);
    
    let ticks_per_measure = ppq * 4;

    for (track_idx, smf_track) in smf.tracks.iter().enumerate() {
        // 跳过空轨道
        if smf_track.is_empty() { continue; }

        let mut current_tick = 0u32;
        let mut active_notes: BTreeMap<u32, Vec<(u8, u8, u32)>> = BTreeMap::new();
        let mut note_ons: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
        let mut midi_program = 0u8;
        let mut midi_bank = 0u8;
        let mut midi_channel = 0u8;

        for event in smf_track {
            current_tick += event.delta.as_int() as u32;

            match event.kind {
                TrackEventKind::Midi { channel, message } => {
                    midi_channel = channel.as_int();
                    match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        let k = key.as_int();
                        let v = vel.as_int();
                        if v > 0 {
                            note_ons.insert(k, (current_tick, v));
                        } else if let Some((start, velocity)) = note_ons.remove(&k) {
                            let duration = current_tick - start;
                            active_notes.entry(start).or_default().push((k, velocity, duration));
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        let k = key.as_int();
                        if let Some((start, velocity)) = note_ons.remove(&k) {
                            let duration = current_tick - start;
                            active_notes.entry(start).or_default().push((k, velocity, duration));
                        }
                    }
                    midly::MidiMessage::ProgramChange { program } => {
                        midi_program = program.as_int();
                    }
                    midly::MidiMessage::Controller { controller, value } => {
                        if controller.as_int() == 0 {
                            midi_bank = value.as_int();
                        }
                    }
                    _ => {}
                    }
                }
                _ => {}
            }
        }
        
        if active_notes.is_empty() {
            continue;
        }
        
        let mut track = Track::default();
        track.name = format!("Track {}", track_idx + 1);
        track.midi_program = midi_program;
        track.midi_bank = midi_bank;
        track.midi_channel = midi_channel;
        if midi_channel == 9 {
            track.is_percussion = true;
        }
        track.sync_instrument_type();
        
        let mut current_measure_idx = 0;
        let mut current_measure = Measure::default();
        
        for (start_tick, notes) in active_notes {
            let measure_idx = (start_tick / ticks_per_measure) as usize;
            
            // 补齐前面的空小节
            while current_measure_idx < measure_idx {
                track.measures.push(current_measure);
                current_measure = Measure::default();
                current_measure_idx += 1;
                
                // 确保 master_bars 足够长
                if song.master_bars.len() <= current_measure_idx {
                    song.master_bars.push(song.master_bars[0].clone());
                }
            }
            
            // 简单的时值量化（这里统一作为四分音符显示，真实场景需根据 duration 映射）
            let duration = Duration { value: NoteValue::Quarter, ..Default::default() };
            
            let mut beat = Beat {
                duration,
                start_tick,
                ..Default::default()
            };
            
            for (pitch, vel, _dur) in notes {
                // 简单的指法分配：尽量在 E (6) 弦到 e (1) 弦上找个位置
                let (string, fret) = map_midi_to_guitar(pitch);
                
                beat.notes.push(Note {
                    string,
                    fret,
                    midi_note: pitch,
                    velocity: vel,
                    note_type: NoteType::Normal,
                    ..Default::default()
                });
            }
            
            if !beat.notes.is_empty() {
                current_measure.voices[0].beats.push(beat);
            }
        }
        
        track.measures.push(current_measure);
        song.tracks.push(track);
    }
    
    // 如果没有提取到任何轨道，至少给个空轨道
    if song.tracks.is_empty() {
        let mut track = Track::default();
        track.measures.push(Measure::default());
        song.tracks.push(track);
    }

    Ok(song)
}

/// 将 MIDI 音高简单映射到标准吉他调弦的弦和品格
fn map_midi_to_guitar(pitch: u8) -> (u8, i8) {
    // 标准调弦 E2=40, A2=45, D3=50, G3=55, B3=59, E4=64
    let tunings = [64, 59, 55, 50, 45, 40]; // 1弦到6弦
    
    for (i, &open_pitch) in tunings.iter().enumerate() {
        if pitch >= open_pitch && pitch <= open_pitch + 24 {
            return (i as u8 + 1, (pitch - open_pitch) as i8);
        }
    }
    
    // 如果超出了范围，降级到最接近的弦
    if pitch < 40 {
        (6, (pitch as i8 - 40)) // 可能产生负数品格（需特殊调弦才能弹）
    } else {
        (1, (pitch as i8 - 64))
    }
}
