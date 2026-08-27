//! Guitar Pro 7/8 (.gp) 解析器

use bassoxide_core::beat::{Beat, Voice};
use bassoxide_core::measure::{MasterBar, Measure};
use bassoxide_core::note::Note;
use bassoxide_core::song::Song;
use bassoxide_core::track::{Track, Tuning};
use bassoxide_core::types::{Duration, NoteValue, TimeSignature};
use roxmltree::{Document, Node};
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
    /// GPIF 0-based 弦索引（0 = 最低音弦）
    string: usize,
    fret: usize,
    /// GPIF `<Midi>` 音高（若有则优先使用）
    midi: Option<u8>,
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
                    // MasterBar 内嵌 Tempo（部分导出）
                    if let Some(tempo_node) = node.descendants().find(|n| n.has_tag_name("Tempo")) {
                        if let Some(text) = tempo_node.text() {
                            if let Ok(bpm) = text.trim().split_whitespace().next().unwrap_or("").parse::<u16>() {
                                if bpm > 0 {
                                    mb.tempo = Some(bpm);
                                }
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
                                    "Midi" => {
                                        if let Some(inner) =
                                            p.descendants().find(|n| n.has_tag_name("Number"))
                                        {
                                            note.midi = inner.text().and_then(|t| t.trim().parse().ok());
                                        } else if let Some(t) = p.text() {
                                            note.midi = t.trim().parse().ok();
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

    // 2. 提取 Score/全局信息（Title/Artist 多为 CDATA，不能只用 .text()）
    if let Some(score_node) = doc.descendants().find(|n| n.has_tag_name("Score")) {
        song.info.title = xml_child_text(score_node, "Title")
            .unwrap_or_else(|| "Unknown Title".to_string());
        song.info.artist = xml_child_text(score_node, "Artist").unwrap_or_default();
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
    if let Some(tracks_node) = doc.descendants().find(|n| {
        n.has_tag_name("Tracks") && n.children().any(|c| c.has_tag_name("Track"))
    }) {
        for track_node in tracks_node.children().filter(|n| n.has_tag_name("Track")) {
            if let Some(id) = track_node.attribute("id") {
                tracks_attr_map.insert(id.to_string(), track_node);
            }
        }
    }

    for t_id in &track_ids {
        let mut track = Track::default();
        if let Some(track_node) = tracks_attr_map.get(t_id) {
            // 只用 Track 直接子节点 Name/ShortName，避免吃到 Sounds 内嵌 Name
            track.name = xml_child_text(*track_node, "Name")
                .or_else(|| xml_child_text(*track_node, "ShortName"))
                .unwrap_or_else(|| format!("Track {}", t_id));

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
                    // GPIF <Pitches> 为低音弦→高音弦；应用内约定弦 1 = 最高音弦（谱面最上方）
                    let high_to_low: Vec<u8> = pitches.into_iter().rev().collect();
                    track.tuning = Tuning {
                        name: "Custom".to_string(),
                        strings: high_to_low
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

            apply_gp7_track_sound(*track_node, &mut track);
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

                                            // GPIF String：0 = 最低音弦；应用弦号：1 = 最高音弦
                                            let n_str = track_objects[track_idx]
                                                .tuning
                                                .string_count()
                                                .max(1);
                                            let our_string = (n_str as u8)
                                                .saturating_sub(gp7_note.string as u8)
                                                .clamp(1, n_str as u8);
                                            let fret = gp7_note.fret as i8;

                                            let mut note = Note {
                                                string: our_string,
                                                fret,
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

                                            note.midi_note = gp7_note.midi.unwrap_or_else(|| {
                                                track_objects[track_idx]
                                                    .tuning
                                                    .midi_note(note.string, note.fret)
                                                    .unwrap_or(0)
                                            });
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

    // 从 MasterTrack Automations 读取 Tempo（GPIF: <Value>120 2</Value>）
    apply_gp7_tempo_automations(&doc, &mut song);

    // 若仍无速度：取首个 MasterBar.tempo，否则保持默认
    if song.tempo == 120 {
        if let Some(bpm) = song.master_bars.iter().find_map(|mb| mb.tempo) {
            if bpm > 0 {
                song.tempo = bpm;
            }
        }
    }

    song.tracks = track_objects;

    Ok(song)
}

/// 解析 MasterTrack 速度自动化，写入 `song.tempo` 与对应小节的 `MasterBar.tempo`
fn apply_gp7_tempo_automations(doc: &Document, song: &mut Song) {
    let Some(mt) = doc.descendants().find(|n| n.has_tag_name("MasterTrack")) else {
        return;
    };
    let Some(autos) = mt.children().find(|n| n.has_tag_name("Automations")) else {
        return;
    };

    let mut first_bpm: Option<u16> = None;
    for auto in autos.children().filter(|n| n.has_tag_name("Automation")) {
        let is_tempo = auto
            .children()
            .find(|n| n.has_tag_name("Type"))
            .and_then(|n| n.text())
            .map(|t| t.trim() == "Tempo")
            .unwrap_or(false);
        if !is_tempo {
            continue;
        }
        let bpm = auto
            .children()
            .find(|n| n.has_tag_name("Value"))
            .and_then(|n| n.text())
            .and_then(|t| t.trim().split_whitespace().next()?.parse::<u16>().ok())
            .filter(|b| *b > 0);
        let Some(bpm) = bpm else {
            continue;
        };
        let bar = auto
            .children()
            .find(|n| n.has_tag_name("Bar"))
            .and_then(|n| n.text())
            .and_then(|t| t.trim().parse::<usize>().ok())
            .unwrap_or(0);

        if first_bpm.is_none() {
            first_bpm = Some(bpm);
        }
        if let Some(mb) = song.master_bars.get_mut(bar) {
            mb.tempo = Some(bpm);
        }
    }
    if let Some(bpm) = first_bpm {
        song.tempo = bpm;
    }
}

fn xml_child_u8(node: Node<'_, '_>, tag: &str) -> Option<u8> {
    xml_child_text(node, tag).and_then(|t| t.parse().ok())
}

/// 合并元素下全部文本/CDATA 并 trim。
/// GPIF 常写成 `<Name>\n<![CDATA[Lead Guitar]]>\n</Name>`，`.text()` 只会拿到首个空白文本节点。
fn xml_text_content(node: Node<'_, '_>) -> String {
    let mut out = String::new();
    for child in node.children() {
        if let Some(t) = child.text() {
            out.push_str(t);
        }
    }
    out.trim().to_string()
}

fn xml_child_text(node: Node<'_, '_>, tag: &str) -> Option<String> {
    let child = node.children().find(|n| n.has_tag_name(tag))?;
    let text = xml_text_content(child);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// 从 Track 的 Sounds/Sound/MIDI 与 MidiConnection 写入 GM program / bank / channel。
fn apply_gp7_track_sound(track_node: Node<'_, '_>, track: &mut Track) {
    if let Some(sounds) = track_node.children().find(|n| n.has_tag_name("Sounds")) {
        if let Some(sound) = sounds.children().find(|n| n.has_tag_name("Sound")) {
            if let Some(midi) = sound.children().find(|n| n.has_tag_name("MIDI")) {
                if let Some(program) = xml_child_u8(midi, "Program") {
                    track.midi_program = program;
                }
                if let Some(msb) = xml_child_u8(midi, "MSB") {
                    track.midi_bank = msb;
                }
            }
        }
    }

    if let Some(conn) = track_node.children().find(|n| n.has_tag_name("MidiConnection")) {
        if let Some(port) = xml_child_u8(conn, "Port") {
            track.midi_port = port;
        }
        if let Some(ch) = xml_child_u8(conn, "PrimaryChannel") {
            track.midi_channel = ch;
            if ch == 9 {
                track.is_percussion = true;
            }
        }
    }

    track.sync_instrument_type();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpif_midi_programs_from_file() {
        let xml_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scratch/score.gpif");
        let Ok(xml) = std::fs::read_to_string(xml_path) else {
            return;
        };
        let song = parse_score_gpif(&xml).expect("Failed to parse GPIF");
        assert!(!song.tracks.is_empty());
        assert_eq!(song.tempo, 200, "应从 MasterTrack Tempo 自动化读取 BPM");
        assert_eq!(song.display_tempo(), 200);
        assert_eq!(song.info.title.trim(), "メタンハイドレート");
        assert_eq!(song.info.artist.trim(), "文藝天国");
        assert_eq!(song.tracks[0].name, "Lead Guitar");
        if song.tracks.len() >= 5 {
            assert_eq!(song.tracks[1].name, "Rhythm Guitar");
            assert_eq!(song.tracks[2].name, "Electric Bass (finger)");
            assert_eq!(song.tracks[3].name, "Drums");
            assert_eq!(song.tracks[4].name, "Vocals");
        }
        if song.tracks.len() >= 3 {
            assert_eq!(song.tracks[0].midi_program, 30, "Lead Guitar 应为 Distortion Guitar");
            assert_eq!(song.tracks[2].midi_program, 33, "贝斯轨应使用文件内 GM 33");
        }
        if let Some(drums) = song.tracks.iter().find(|t| t.is_percussion) {
            assert_eq!(drums.midi_program, 0);
            assert_eq!(drums.midi_channel, 9);
        }
    }

    #[test]
    fn test_gpif_string_order_and_midi_pitch() {
        let xml_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scratch/score.gpif");
        let Ok(xml) = std::fs::read_to_string(xml_path) else {
            return;
        };
        let song = parse_score_gpif(&xml).expect("Failed to parse GPIF");
        let lead = &song.tracks[0];
        // 弦 1 = 最高音 E4(64)，弦 6 = 最低音 E2(40)
        assert_eq!(lead.tuning.strings[0].number, 1);
        assert_eq!(lead.tuning.strings[0].tuning, 64);
        assert_eq!(lead.tuning.strings.last().unwrap().tuning, 40);

        // 找到 Fret=6 且 Midi=70 的音符（GPIF Note0: String5→弦1）
        let mut found = None;
        for m in &lead.measures {
            for v in &m.voices {
                for b in &v.beats {
                    for n in &b.notes {
                        if n.fret == 6 && n.midi_note == 70 {
                            found = Some(n.clone());
                            break;
                        }
                    }
                }
            }
        }
        let n0 = found.expect("应找到 Midi=70 Fret=6 的音符");
        assert_eq!(n0.string, 1, "高音弦应映射到弦 1（谱面最上）");
        assert_eq!(lead.tuning.midi_note(n0.string, n0.fret), Some(70));

        // 低音弦空弦：GPIF String=0 Fret=0 Midi=40 → 弦 6
        let mut low = None;
        for m in &lead.measures {
            for v in &m.voices {
                for b in &v.beats {
                    for n in &b.notes {
                        if n.fret == 0 && n.midi_note == 40 {
                            low = Some(n.clone());
                            break;
                        }
                    }
                }
            }
        }
        let low = low.expect("应找到低音 E2 空弦");
        assert_eq!(low.string, 6);
    }

    #[test]
    fn test_xml_text_content_joins_cdata_after_whitespace() {
        let xml = r#"<?xml version="1.0"?><Root><Name>
        <![CDATA[Lead Guitar]]>
      </Name><Empty>
        <![CDATA[]]>
      </Empty></Root>"#;
        let doc = Document::parse(xml).unwrap();
        let name = doc.descendants().find(|n| n.has_tag_name("Name")).unwrap();
        // 合并全部文本节点并 trim，兼容仅空白 + CDATA 的 GPIF 写法
        assert_eq!(xml_text_content(name), "Lead Guitar");
        let empty = doc.descendants().find(|n| n.has_tag_name("Empty")).unwrap();
        assert_eq!(xml_text_content(empty), "");
        assert!(xml_child_text(doc.root_element(), "Name").as_deref() == Some("Lead Guitar"));
        assert!(xml_child_text(doc.root_element(), "Empty").is_none());
    }
}
