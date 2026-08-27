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
    pub is_dead: bool,
    pub has_vibrato: bool,
    pub has_bend: bool,
    pub has_slide: bool,
    pub has_harmonic: bool,
    pub has_palm_mute: bool,
    pub has_let_ring: bool,
    pub hammer_pull: bool, // 泛指击勾弦
}

#[derive(Debug)]
struct Gp7Rhythm {
    value: NoteValue,
    dots: u8,
    tuplet_num: u8,
    tuplet_den: u8,
}

impl Default for Gp7Rhythm {
    fn default() -> Self {
        Self {
            value: NoteValue::Quarter,
            dots: 0,
            tuplet_num: 1,
            tuplet_den: 1,
        }
    }
}

/// 解析 GPIF 的 NoteValue 字符串为 NoteValue
fn parse_gp_note_value(s: &str) -> NoteValue {
    match s.trim() {
        "Whole" => NoteValue::Whole,
        "Half" => NoteValue::Half,
        "Quarter" => NoteValue::Quarter,
        "Eighth" => NoteValue::Eighth,
        "16th" | "Sixteenth" => NoteValue::Sixteenth,
        "32nd" | "ThirtySecond" => NoteValue::ThirtySecond,
        "64th" | "SixtyFourth" => NoteValue::SixtyFourth,
        _ => NoteValue::Quarter,
    }
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
    let doc =
        Document::parse(xml).map_err(|e| IoError::ParseError(format!("XML Parse Error: {}", e)))?;

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
                    if id.is_empty() {
                        continue;
                    }
                    master_bars_map.insert(id, mb);
                }
                "Bar" => {
                    if id.is_empty() {
                        continue;
                    }
                    let mut bar = Gp7Bar::default();
                    if let Some(voices_node) = node.descendants().find(|n| n.has_tag_name("Voices"))
                    {
                        if let Some(text) = voices_node.text() {
                            bar.voices = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    bars_map.insert(id, bar);
                }
                "Voice" => {
                    if id.is_empty() {
                        continue;
                    }
                    let mut voice = Gp7Voice::default();
                    if let Some(beats_node) = node.descendants().find(|n| n.has_tag_name("Beats")) {
                        if let Some(text) = beats_node.text() {
                            voice.beats = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    voices_map.insert(id, voice);
                }
                "Beat" => {
                    if id.is_empty() {
                        continue;
                    }
                    let mut beat = Gp7Beat::default();
                    if let Some(notes_node) = node.descendants().find(|n| n.has_tag_name("Notes")) {
                        if let Some(text) = notes_node.text() {
                            beat.notes = text.split_whitespace().map(|s| s.to_string()).collect();
                        }
                    }
                    if let Some(rhythm_node) = node.descendants().find(|n| n.has_tag_name("Rhythm"))
                    {
                        beat.rhythm_ref = rhythm_node.attribute("ref").unwrap_or("").to_string();
                    }
                    beats_map.insert(id, beat);
                }
                "Note" => {
                    if id.is_empty() {
                        continue;
                    }
                    let mut note = Gp7Note::default();
                    if let Some(props) = node.descendants().find(|n| n.has_tag_name("Properties")) {
                        for p in props.children().filter(|n| n.has_tag_name("Property")) {
                            if let Some(prop_name) = p.attribute("name") {
                                match prop_name {
                                    "String" => {
                                        if let Some(inner) = p.descendants().find(|n| n.has_tag_name("String")) {
                                            note.string = inner.text().unwrap_or("0").parse().unwrap_or(0);
                                        }
                                    }
                                    "Fret" => {
                                        if let Some(inner) = p.descendants().find(|n| n.has_tag_name("Fret")) {
                                            note.fret = inner.text().unwrap_or("0").parse().unwrap_or(0);
                                        }
                                    }
                                    "Tie" => note.is_tie = true,
                                    "Muted" => note.is_dead = true,
                                    "Vibrato" => note.has_vibrato = true,
                                    "Bend" => note.has_bend = true,
                                    "Slide" => note.has_slide = true,
                                    "HarmonicType" | "HarmonicFret" => note.has_harmonic = true,
                                    "PalmMute" => note.has_palm_mute = true,
                                    "LetRing" => note.has_let_ring = true,
                                    "Hammer" | "PullOff" | "Legato" => note.hammer_pull = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                    notes_map.insert(id, note);
                }
                "Rhythm" => {
                    if id.is_empty() {
                        continue;
                    }
                    let mut rhythm = Gp7Rhythm::default();
                    if let Some(nv) = node
                        .descendants()
                        .find(|n| n.has_tag_name("NoteValue"))
                        .and_then(|n| n.text())
                    {
                        rhythm.value = parse_gp_note_value(nv);
                    }
                    if let Some(dot) = node.descendants().find(|n| n.has_tag_name("AugmentationDot"))
                    {
                        rhythm.dots = dot
                            .attribute("count")
                            .and_then(|c| c.parse().ok())
                            .unwrap_or(1);
                    }
                    if let Some(tp) = node.descendants().find(|n| n.has_tag_name("PrimaryTuplet")) {
                        rhythm.tuplet_num = tp
                            .attribute("num")
                            .and_then(|c| c.parse().ok())
                            .unwrap_or(1);
                        rhythm.tuplet_den = tp
                            .attribute("den")
                            .and_then(|c| c.parse().ok())
                            .unwrap_or(1);
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

    // 获取轨道 ID 顺序
    let mut track_ids = Vec::new();
    if let Some(mt_node) = doc.descendants().find(|n| n.has_tag_name("MasterTrack")) {
        if let Some(tracks_node) = mt_node.descendants().find(|n| n.has_tag_name("Tracks")) {
            if let Some(text) = tracks_node.text() {
                track_ids = text.split_whitespace().map(|s| s.to_string()).collect();
            }
        }
    }

    // 提前创建 Track 对象并存储到 HashMap 或数组，保证顺序
    let mut track_objects: Vec<Track> = Vec::new();

    // 首先从 XML 的 <Tracks> 中解析各个轨道的属性
    let mut tracks_attr_map = HashMap::new();
    if let Some(tracks_node) = doc.descendants().find(|n| n.has_tag_name("Tracks")) {
        for track_node in tracks_node.children().filter(|n| n.has_tag_name("Track")) {
            if let Some(id) = track_node.attribute("id") {
                tracks_attr_map.insert(id.to_string(), track_node);
            }
        }
    }

    for t_id in &track_ids {
        let mut track = Track::default();
        if let Some(track_node) = tracks_attr_map.get(t_id) {
            track.name = track_node
                .descendants()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("Track")
                .to_string();

            if let Some(tuning_node) = track_node.descendants().find(|n| n.has_tag_name("Tuning")) {
                let mut pitches = Vec::new();
                if let Some(text) = tuning_node
                    .descendants()
                    .find(|n| n.has_tag_name("Pitches"))
                    .and_then(|n| n.text())
                {
                    for p in text.split_whitespace() {
                        if let Ok(midi) = p.parse::<u8>() {
                            pitches.push(midi);
                        }
                    }
                }
                if !pitches.is_empty() {
                    track.tuning = Tuning {
                        name: "Custom".to_string(),
                        strings: pitches
                            .into_iter()
                            .enumerate()
                            .map(|(i, tuning)| bassoxide_core::track::GuitarString {
                                number: (i + 1) as u8,
                                tuning,
                            })
                            .collect(),
                    };
                }
            }
        }
        track_objects.push(track);
    }

    // 3. 提取 MasterBars 并组装每个 Track 的 Measure
    if let Some(master_bars_node) = doc.descendants().find(|n| n.has_tag_name("MasterBars")) {
        for mb_node in master_bars_node
            .children()
            .filter(|n| n.has_tag_name("MasterBar"))
        {
            // MasterBar 本身的属性 (拍号等)
            let mut mb = MasterBar::default();
            if let Some(id) = mb_node.attribute("id") {
                if let Some(gp7_mb) = master_bars_map.get(id) {
                    mb.time_signature = gp7_mb.time_signature;
                    mb.tempo = gp7_mb.tempo;
                }
            }
            song.master_bars.push(mb);

            // 该 MasterBar 对应的各轨道 Bar ID 列表
            let mut bar_ids = Vec::new();
            if let Some(bars_node) = mb_node.descendants().find(|n| n.has_tag_name("Bars")) {
                if let Some(text) = bars_node.text() {
                    bar_ids = text.split_whitespace().map(|s| s.to_string()).collect();
                }
            }

            // 为每个轨道添加当前小节 (Measure)
            for (track_idx, bar_id) in bar_ids.into_iter().enumerate() {
                if track_idx >= track_objects.len() {
                    break;
                }

                let mut measure = Measure::default();
                if let Some(gp7_bar) = bars_map.get(&bar_id) {
                    for (v_idx, voice_id) in gp7_bar
                        .voices
                        .iter()
                        .enumerate()
                        .take(bassoxide_core::measure::MAX_VOICES)
                    {
                        let mut voice = Voice::default();
                        if let Some(gp7_voice) = voices_map.get(voice_id) {
                            for beat_id in &gp7_voice.beats {
                                let mut beat = Beat::default();
                                if let Some(gp7_beat) = beats_map.get(beat_id) {
                                    if let Some(gp7_rhythm) = rhythms_map.get(&gp7_beat.rhythm_ref)
                                    {
                                        beat.duration = Duration {
                                            value: gp7_rhythm.value,
                                            dotted: gp7_rhythm.dots >= 1,
                                            double_dotted: gp7_rhythm.dots >= 2,
                                            tuplet_numerator: gp7_rhythm.tuplet_num.max(1),
                                            tuplet_denominator: gp7_rhythm.tuplet_den.max(1),
                                        };
                                    }

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
                                                velocity: 95,
                                                note_type,
                                                effects: Vec::new(),
                                                left_fingering: None,
                                                right_fingering: None,
                                                midi_note: 0,
                                            };

                                            use bassoxide_core::effects::*;
                                            if gp7_note.has_vibrato {
                                                note.effects.push(NoteEffect::Vibrato(VibratoType::Finger, VibratoSpeed::Medium));
                                            }
                                            if gp7_note.has_bend {
                                                note.effects.push(NoteEffect::Bend(BendEffect {
                                                    bend_type: BendType::Bend,
                                                    points: vec![
                                                        BendPoint { position: 0, value: 0, vibrato: false },
                                                        BendPoint { position: 6, value: 4, vibrato: false }, // 默认推全音 (4 * 25 cents)
                                                        BendPoint { position: 12, value: 4, vibrato: false },
                                                    ],
                                                }));
                                            }
                                            if gp7_note.has_slide {
                                                note.effects.push(NoteEffect::Slide(vec![SlideType::LegatoSlide]));
                                            }
                                            if gp7_note.has_harmonic {
                                                note.effects.push(NoteEffect::Harmonic(HarmonicEffect {
                                                    harmonic_type: HarmonicType::Natural,
                                                    fret_offset: None,
                                                }));
                                            }
                                            if gp7_note.has_palm_mute {
                                                note.effects.push(NoteEffect::PalmMute);
                                            }
                                            if gp7_note.has_let_ring {
                                                note.effects.push(NoteEffect::LetRing);
                                            }
                                            if gp7_note.hammer_pull {
                                                note.effects.push(NoteEffect::HammerOnPullOff(HammerOnPullOff::HammerOn)); // 统配为 HammerOn
                                            }

                                            note.midi_note = track_objects[track_idx]
                                                .tuning
                                                .midi_note(note.string, note.fret)
                                                .unwrap_or(0);
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
                track_objects[track_idx].measures.push(measure);
            }
        }
    }

    // 如果没有 MasterBar (容错)
    if song.master_bars.is_empty() {
        song.master_bars.push(MasterBar::default());
    }

    song.tracks = track_objects;

    Ok(song)
}
