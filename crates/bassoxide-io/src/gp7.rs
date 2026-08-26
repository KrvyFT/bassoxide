//! Guitar Pro 7/8 (.gp) 解析器

use bassoxide_core::beat::{Beat, Voice};
use bassoxide_core::measure::{MasterBar, Measure};
use bassoxide_core::note::Note;
use bassoxide_core::song::{Song, SongInfo};
use bassoxide_core::track::{Track, Tuning};
use bassoxide_core::types::{Duration, NoteValue, TimeSignature};
use roxmltree::Document;
use std::collections::HashMap;
use std::io::Read;

use crate::error::{IoError, Result};

/// 内部 GP7 结构体
#[derive(Debug, Default)]
struct Gp7MasterBar {
    time_signature: TimeSignature,
    tempo: Option<u16>,
}

#[derive(Debug, Default)]
struct Gp7Bar {
    voices: Vec<String>, // voice IDs
}

#[derive(Debug, Default)]
struct Gp7Voice {
    beats: Vec<String>, // beat IDs
}

#[derive(Debug, Default)]
struct Gp7Beat {
    notes: Vec<String>, // note IDs
    rhythm_ref: String, // rhythm ID
}

#[derive(Debug, Default)]
struct Gp7Note {
    string: usize,
    fret: usize,
    is_tie: bool,
    is_dead: bool,
}

#[derive(Debug, Default)]
struct Gp7Rhythm {
    primary: i32,
    dot: bool,
}

/// 解析 GP7/GP8 文件
pub fn parse_gp7(data: &[u8]) -> Result<Song> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid ZIP/GP7 file: {}", e)))?;

    let mut score_gpif_content = String::new();
    let mut found = false;

    for i in 0..archive.len() {
        if let Ok(mut file) = archive.by_index(i) {
            if file.name() == "Content/score.gpif" || file.name() == "score.gpif" {
                file.read_to_string(&mut score_gpif_content)?;
                found = true;
                break;
            }
        }
    }

    if !found {
        return Err(IoError::UnsupportedFormat(
            "Missing score.gpif in GP archive".to_string(),
        ));
    }

    parse_score_gpif(&score_gpif_content)
}

fn parse_score_gpif(xml: &str) -> Result<Song> {
    let doc = Document::parse(xml)
        .map_err(|e| IoError::ParseError(format!("XML Parse Error: {}", e)))?;

    let mut master_bars_map = HashMap::new();
    let mut bars_map = HashMap::new();
    let mut voices_map = HashMap::new();
    let mut beats_map = HashMap::new();
    let mut notes_map = HashMap::new();
    let mut rhythms_map = HashMap::new();

    // 1. 第一遍扫描：建立所有 ID -> 实体 的映射表
    for node in doc.descendants() {
        if node.is_element() {
            let id = node.attribute("id").unwrap_or("").to_string();
            
            match node.tag_name().name() {
                "MasterBar" => {
                    let mut mb = Gp7MasterBar::default();
                    if let Some(time) = node.descendants().find(|n| n.has_tag_name("Time")) {
                        // 简单解析如 4/4
                        let text = time.text().unwrap_or("4/4");
                        let parts: Vec<&str> = text.split('/').collect();
                        if parts.len() == 2 {
                            if let Ok(num) = parts[0].parse::<u8>() {
                                mb.time_signature.numerator = num;
                            }
                            if let Ok(den) = parts[1].parse::<i32>() {
                                mb.time_signature.denominator = match den {
                                    1 => NoteValue::Whole,
                                    2 => NoteValue::Half,
                                    8 => NoteValue::Eighth,
                                    16 => NoteValue::Sixteenth,
                                    32 => NoteValue::ThirtySecond,
                                    64 => NoteValue::SixtyFourth,
                                    _ => NoteValue::Quarter, // default 4
                                };
                            }
                        }
                    }
                    if id.is_empty() { continue; }
                    master_bars_map.insert(id, mb);
                }
                "Bar" => {
                    if id.is_empty() { continue; }
                    let mut bar = Gp7Bar::default();
                    if let Some(voices_node) = node.descendants().find(|n| n.has_tag_name("Voices")) {
                        if let Some(text) = voices_node.text() {
                            bar.voices = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    bars_map.insert(id, bar);
                }
                "Voice" => {
                    if id.is_empty() { continue; }
                    let mut voice = Gp7Voice::default();
                    if let Some(beats_node) = node.descendants().find(|n| n.has_tag_name("Beats")) {
                        if let Some(text) = beats_node.text() {
                            voice.beats = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    voices_map.insert(id, voice);
                }
                "Beat" => {
                    if id.is_empty() { continue; }
                    let mut beat = Gp7Beat::default();
                    if let Some(notes_node) = node.descendants().find(|n| n.has_tag_name("Notes")) {
                        if let Some(text) = notes_node.text() {
                            beat.notes = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    if let Some(rhythm_node) = node.descendants().find(|n| n.has_tag_name("Rhythm")) {
                        beat.rhythm_ref = rhythm_node.attribute("ref").unwrap_or("").to_string();
                    }
                    beats_map.insert(id, beat);
                }
                "Note" => {
                    if id.is_empty() { continue; }
                    let mut note = Gp7Note::default();
                    if let Some(props) = node.descendants().find(|n| n.has_tag_name("Properties")) {
                        for p in props.children().filter(|n| n.is_element()) {
                            match p.tag_name().name() {
                                "String" => note.string = p.text().unwrap_or("0").parse().unwrap_or(0),
                                "Fret" => note.fret = p.text().unwrap_or("0").parse().unwrap_or(0),
                                "Tie" => note.is_tie = true,
                                "Muted" => note.is_dead = true, // maybe different in gp7
                                _ => {}
                            }
                        }
                    }
                    notes_map.insert(id, note);
                }
                "Rhythm" => {
                    if id.is_empty() { continue; }
                    let mut rhythm = Gp7Rhythm::default();
                    if let Some(p) = node.descendants().find(|n| n.has_tag_name("Primary")) {
                        rhythm.primary = p.text().unwrap_or("4").parse().unwrap_or(4);
                    }
                    if node.descendants().any(|n| n.has_tag_name("Dot")) {
                        rhythm.dot = true;
                    }
                    rhythms_map.insert(id, rhythm);
                }
                _ => {}
            }
        }
    }

    let mut song = Song::default();

    // 2. 提取 Score/全局信息
    if let Some(score_node) = doc.descendants().find(|n| n.has_tag_name("Score")) {
        song.info.title = score_node
            .descendants()
            .find(|n| n.has_tag_name("Title"))
            .and_then(|n| n.text())
            .unwrap_or("Unknown Title")
            .to_string();

        song.info.artist = score_node
            .descendants()
            .find(|n| n.has_tag_name("Artist"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();
    }

    // GP7 将 MasterBars 按顺序放在 <MasterBars> 节点下，提取它们
    if let Some(master_bars_node) = doc.descendants().find(|n| n.has_tag_name("MasterBars")) {
        // 如果这里只有一系列 ID (在 <MasterBars> 标签的内容中)
        let mut mb_ids = Vec::new();
        if let Some(text) = master_bars_node.text() {
             mb_ids = text.split_whitespace().map(|s| s.to_string()).collect();
        }
        
        // 如果它包含子节点 <MasterBar> (由于我们第一次扫描已经按 id 解析了，但需要保持顺序)
        if mb_ids.is_empty() {
             for mb_node in master_bars_node.children().filter(|n| n.has_tag_name("MasterBar")) {
                 if let Some(id) = mb_node.attribute("id") {
                     mb_ids.push(id.to_string());
                 }
             }
        }

        for id in mb_ids {
            if let Some(gp7_mb) = master_bars_map.get(&id) {
                let mut mb = MasterBar::default();
                mb.time_signature = gp7_mb.time_signature;
                mb.tempo = gp7_mb.tempo;
                song.master_bars.push(mb);
            }
        }
    }
    
    // Fallback
    if song.master_bars.is_empty() {
        song.master_bars.push(MasterBar::default());
    }

    // 3. 提取 Tracks 并还原数据树
    if let Some(tracks_node) = doc.descendants().find(|n| n.has_tag_name("Tracks")) {
        for track_node in tracks_node.children().filter(|n| n.has_tag_name("Track")) {
            let mut track = Track::default();
            
            // Track 名称
            track.name = track_node
                .descendants()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("Track")
                .to_string();
            
            // 解析弦数和定弦
            if let Some(tuning_node) = track_node.descendants().find(|n| n.has_tag_name("Tuning")) {
                let mut pitches = Vec::new();
                if let Some(text) = tuning_node.descendants().find(|n| n.has_tag_name("Pitches")).and_then(|n| n.text()) {
                    for p in text.split_whitespace() {
                        if let Ok(midi) = p.parse::<u8>() {
                            pitches.push(midi);
                        }
                    }
                }
                if !pitches.is_empty() {
                    track.tuning = Tuning {
                        name: "Custom".to_string(),
                        strings: pitches.into_iter().enumerate().map(|(i, tuning)| bassoxide_core::track::GuitarString {
                            number: (i + 1) as u8,
                            tuning,
                        }).collect(),
                    };
                }
            }
            
            // 解析 Bars
            let mut bar_ids = Vec::new();
            if let Some(bars_node) = track_node.descendants().find(|n| n.has_tag_name("Bars")) {
                if let Some(text) = bars_node.text() {
                    bar_ids = text.split_whitespace().map(|s| s.to_string()).collect();
                }
            }

            // 构建 Measure 列表
            for bar_id in bar_ids {
                let mut measure = Measure::default();
                
                if let Some(gp7_bar) = bars_map.get(&bar_id) {
                    for (v_idx, voice_id) in gp7_bar.voices.iter().enumerate().take(bassoxide_core::measure::MAX_VOICES) {
                        let mut voice = Voice::default();
                        
                        if let Some(gp7_voice) = voices_map.get(voice_id) {
                            for beat_id in &gp7_voice.beats {
                                let mut beat = Beat::default();
                                
                                if let Some(gp7_beat) = beats_map.get(beat_id) {
                                    // 处理 Rhythm (时值)
                                    if let Some(gp7_rhythm) = rhythms_map.get(&gp7_beat.rhythm_ref) {
                                        let val = match gp7_rhythm.primary {
                                            1 => NoteValue::Whole,
                                            2 => NoteValue::Half,
                                            4 => NoteValue::Quarter,
                                            8 => NoteValue::Eighth,
                                            16 => NoteValue::Sixteenth,
                                            32 => NoteValue::ThirtySecond,
                                            64 => NoteValue::SixtyFourth,
                                            _ => NoteValue::Quarter,
                                        };
                                        beat.duration = Duration {
                                            value: val,
                                            dotted: gp7_rhythm.dot,
                                            ..Default::default()
                                        };
                                    }
                                    
                                    // 处理 Notes
                                    for note_id in &gp7_beat.notes {
                                        if let Some(gp7_note) = notes_map.get(note_id) {
                                            let note_type = if gp7_note.is_tie {
                                                bassoxide_core::note::NoteType::Tie
                                            } else if gp7_note.is_dead {
                                                bassoxide_core::note::NoteType::Dead
                                            } else {
                                                bassoxide_core::note::NoteType::Normal
                                            };
                                            
                                            let mut note = Note {
                                                string: gp7_note.string.max(1) as u8,
                                                fret: gp7_note.fret as i8,
                                                velocity: 100, // 默认力度
                                                note_type,
                                                effects: Vec::new(),
                                                left_fingering: None,
                                                right_fingering: None,
                                                midi_note: 0,
                                            };
                                            
                                            // 计算 MIDI note
                                            note.midi_note = track.tuning.midi_note(note.string, note.fret).unwrap_or(0);
                                            beat.notes.push(note);
                                        }
                                    }
                                }
                                voice.beats.push(beat);
                            }
                        }
                        measure.voices[v_idx] = voice;
                    }
                }
                track.measures.push(measure);
            }
            song.tracks.push(track);
        }
    }

    Ok(song)
}
