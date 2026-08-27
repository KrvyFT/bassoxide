//! 轨道数据模型。
//!
//! `Track` 代表一个乐器轨道（如吉他、贝斯、鼓）。
//! 包含调弦信息、MIDI 通道配置和所有小节的音符数据。

use serde::{Deserialize, Serialize};

use crate::measure::Measure;
use crate::midi::MidiChannel;
use crate::types::{Clef, Color, InstrumentType, MidiNote};

/// 吉他弦的调弦信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuitarString {
    /// 弦号 (1-based, 1 = 最高音弦)
    pub number: u8,
    /// 空弦 MIDI 音高
    pub tuning: MidiNote,
}

/// 预设调弦方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tuning {
    /// 调弦名称 (如 "Standard E", "Drop D")
    pub name: String,
    /// 各弦调弦值
    pub strings: Vec<GuitarString>,
}

impl Tuning {
    /// 标准吉他调弦 (E2-E4: MIDI 40,45,50,55,59,64)
    pub fn standard_guitar() -> Self {
        Self {
            name: "Standard E".to_string(),
            strings: vec![
                GuitarString { number: 1, tuning: 64 }, // E4
                GuitarString { number: 2, tuning: 59 }, // B3
                GuitarString { number: 3, tuning: 55 }, // G3
                GuitarString { number: 4, tuning: 50 }, // D3
                GuitarString { number: 5, tuning: 45 }, // A2
                GuitarString { number: 6, tuning: 40 }, // E2
            ],
        }
    }

    /// 标准贝斯调弦 (E1-G2: MIDI 28,33,38,43)
    pub fn standard_bass() -> Self {
        Self {
            name: "Standard Bass".to_string(),
            strings: vec![
                GuitarString { number: 1, tuning: 43 }, // G2
                GuitarString { number: 2, tuning: 38 }, // D2
                GuitarString { number: 3, tuning: 33 }, // A1
                GuitarString { number: 4, tuning: 28 }, // E1
            ],
        }
    }

    /// 弦数
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    /// 根据弦号和品格计算 MIDI 音高
    pub fn midi_note(&self, string: u8, fret: i8) -> Option<MidiNote> {
        self.strings
            .iter()
            .find(|s| s.number == string)
            .map(|s| (s.tuning as i16 + fret as i16).clamp(0, 127) as MidiNote)
    }

    /// 在给定品格上限内，为音高找最佳指法（优先低品格，其次靠近 prefer_string）
    pub fn best_fingering(
        &self,
        pitch: MidiNote,
        max_fret: u8,
        prefer_string: Option<u8>,
    ) -> Option<(u8, i8)> {
        let max_fret = max_fret as i16;
        let mut best: Option<(u8, i8, i16, i16)> = None; // string, fret, fret_cost, string_dist
        for s in &self.strings {
            let fret = pitch as i16 - s.tuning as i16;
            if fret < 0 || fret > max_fret {
                continue;
            }
            let string_dist = prefer_string
                .map(|p| (p as i16 - s.number as i16).abs())
                .unwrap_or(0);
            let cand = (s.number, fret as i8, fret, string_dist);
            let better = match best {
                None => true,
                Some((_, _, bf, bd)) => fret < bf || (fret == bf && string_dist < bd),
            };
            if better {
                best = Some(cand);
            }
        }
        best.map(|(n, f, _, _)| (n, f))
    }

    /// 调整弦数；新增弦按相邻弦向下约纯四度延伸，超出部分截断
    pub fn resize_strings(&mut self, count: usize) {
        let count = count.clamp(1, 8);
        if count == self.strings.len() {
            self.renumber_strings();
            return;
        }
        if count < self.strings.len() {
            self.strings.truncate(count);
        } else {
            while self.strings.len() < count {
                let next_num = (self.strings.len() + 1) as u8;
                let prev = self.strings.last().map(|s| s.tuning).unwrap_or(40);
                let tuning = prev.saturating_sub(5).max(12);
                self.strings.push(GuitarString {
                    number: next_num,
                    tuning,
                });
            }
        }
        self.renumber_strings();
        self.name = format!("{}-string", count);
    }

    fn renumber_strings(&mut self) {
        for (i, s) in self.strings.iter_mut().enumerate() {
            s.number = (i + 1) as u8;
        }
    }
}

/// MIDI 音高 → 科学音高名（如 E2、C#4）
pub fn midi_note_name(midi: MidiNote) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let name = NAMES[(midi % 12) as usize];
    let octave = (midi as i16 / 12) - 1;
    format!("{name}{octave}")
}

/// 轨道谱面显示配置（五线谱与六线谱可同时开启）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffDisplay {
    /// 五线谱
    pub show_standard: bool,
    /// 六线谱（Tab，弦数由调弦决定）
    pub show_tab: bool,
    /// Tab 弦数（与 `Track.tuning` 保持同步）
    pub tab_strings: u8,
}

impl Default for StaffDisplay {
    fn default() -> Self {
        Self {
            show_standard: true,
            show_tab: false,
            tab_strings: 6,
        }
    }
}

impl StaffDisplay {
    /// 按轨道乐器与弦数给出合理默认显示
    pub fn default_for(midi_program: u8, string_count: usize, is_percussion: bool) -> Self {
        if is_percussion {
            return Self {
                show_standard: true,
                show_tab: false,
                tab_strings: string_count.max(1).min(8) as u8,
            };
        }
        let is_guitar_bass = (24..=39).contains(&midi_program) && string_count > 0;
        if is_guitar_bass {
            Self {
                show_standard: false,
                show_tab: true,
                tab_strings: string_count.clamp(1, 8) as u8,
            }
        } else {
            Self::default()
        }
    }

    /// 启用六线谱（Tab）
    pub fn enable_tab(&mut self, string_count: u8) {
        self.show_tab = true;
        self.tab_strings = string_count.clamp(1, 8);
    }

    /// 关闭 Tab
    pub fn disable_tab(&mut self) {
        self.show_tab = false;
    }
}

/// 乐器轨道
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// 轨道编号 (1-based)
    pub number: u8,
    /// 轨道名称
    pub name: String,
    /// 乐器种类
    pub instrument_type: InstrumentType,
    /// 弦/调弦信息
    pub tuning: Tuning,
    /// MIDI 通道 (0-based)
    pub midi_channel: u8,
    /// MIDI 端口 (0-based)
    pub midi_port: u8,
    /// MIDI 音色编号 (General MIDI program, 0-127)
    pub midi_program: u8,
    /// MIDI 音色库编号 (Bank, 通常 0 为默认, 128 为打击乐)
    pub midi_bank: u8,
    /// 变调夹位置 (0 = 无变调夹)
    pub capo: u8,
    /// 品格数
    pub fret_count: u8,
    /// 谱号
    pub clef: Clef,
    /// 显示颜色
    pub color: Color,
    /// 音量 (0–127)
    pub volume: u8,
    /// 声相 (0–127, 64 = 居中)
    pub pan: u8,
    /// 是否静音
    pub is_muted: bool,
    /// 是否 Solo
    pub is_solo: bool,
    /// 是否为鼓轨道
    pub is_percussion: bool,
    /// 谱面显示配置
    pub staff_display: StaffDisplay,
    /// 各小节数据
    pub measures: Vec<Measure>,
}

impl Default for Track {
    fn default() -> Self {
        let midi_program = 25; // Steel Guitar (GM)
        let tuning = Tuning::standard_guitar();
        let staff_display =
            StaffDisplay::default_for(midi_program, tuning.string_count(), false);
        Self {
            number: 1,
            name: "Track 1".to_string(),
            instrument_type: InstrumentType::AcousticGuitar,
            tuning,
            midi_channel: 0,
            midi_port: 0,
            midi_program,
            midi_bank: 0,
            capo: 0,
            fret_count: 24,
            clef: Clef::Treble,
            color: Color::rgb(255, 0, 0),
            volume: 100,
            pan: 64,
            is_muted: false,
            is_solo: false,
            is_percussion: false,
            staff_display,
            measures: Vec::new(),
        }
    }
}

impl Track {
    /// 弦数
    pub fn string_count(&self) -> usize {
        self.tuning.string_count()
    }

    /// 用 GP MIDI 通道表条目回填 GM 音色，不根据轨道名猜测。
    pub fn apply_midi_channel(&mut self, channel: &MidiChannel) {
        self.midi_channel = channel.channel % 16;
        self.midi_program = channel.instrument;
        self.volume = channel.volume;
        self.pan = channel.balance;
        if channel.is_percussion() {
            self.is_percussion = true;
        }
        self.sync_instrument_type();
        self.ensure_staff_display();
    }

    /// 按当前 GM program / 打击乐标志同步 `instrument_type`，不改音色号。
    pub fn sync_instrument_type(&mut self) {
        self.instrument_type = InstrumentType::from_gm(self.midi_program, self.is_percussion);
    }

    /// 若尚未按乐器初始化过合理谱面，则写入默认配置
    pub fn ensure_staff_display(&mut self) {
        // 加载后始终按当前乐器校正一次默认组合，避免旧默认值卡住
        self.staff_display = StaffDisplay::default_for(
            self.midi_program,
            self.string_count(),
            self.is_percussion,
        );
    }

    /// 同步 Tab 弦数显示与当前调弦
    pub fn sync_tab_string_count(&mut self) {
        self.staff_display.tab_strings = self.string_count().clamp(1, 8) as u8;
    }

    /// 启用六线谱并按当前（或指定）弦数同步调弦显示
    pub fn enable_tab(&mut self) {
        self.sync_tab_string_count();
        self.staff_display.enable_tab(self.staff_display.tab_strings);
    }

    /// 调整弦数：保留音高地重映射谱面音符，并更新 Tab 显示
    pub fn set_string_count(&mut self, count: usize) {
        let count = count.clamp(1, 8);
        if count == self.string_count() {
            self.sync_tab_string_count();
            self.staff_display.show_tab = true;
            return;
        }
        let old = self.tuning.clone();
        self.tuning.resize_strings(count);
        self.remap_notes_preserving_pitch(&old);
        self.sync_tab_string_count();
        self.staff_display.show_tab = true;
    }

    /// 修改某一弦空弦音高，并按旧音高把音符映射到新指法
    pub fn set_string_open_pitch(&mut self, string_number: u8, midi: MidiNote) {
        let old = self.tuning.clone();
        if let Some(s) = self
            .tuning
            .strings
            .iter_mut()
            .find(|s| s.number == string_number)
        {
            if s.tuning == midi {
                return;
            }
            s.tuning = midi;
        } else {
            return;
        }
        self.remap_notes_preserving_pitch(&old);
    }

    /// 将轨道上所有音符按「旧调弦算出的音高」映射到新调弦的弦位
    pub fn remap_notes_preserving_pitch(&mut self, old_tuning: &Tuning) {
        let max_fret = self.fret_count.max(24);
        let new_tuning = self.tuning.clone();
        for measure in &mut self.measures {
            for voice in &mut measure.voices {
                for beat in &mut voice.beats {
                    for note in &mut beat.notes {
                        let pitch = if note.midi_note > 0 {
                            note.midi_note
                        } else {
                            old_tuning
                                .midi_note(note.string, note.fret)
                                .unwrap_or(note.midi_note)
                        };
                        if note.is_dead() && note.fret < 0 {
                            // 死音：尽量保留原弦，否则落到最近弦
                            let s = note
                                .string
                                .min(new_tuning.string_count() as u8)
                                .max(1);
                            note.string = s;
                            continue;
                        }
                        if let Some((s, f)) =
                            new_tuning.best_fingering(pitch, max_fret, Some(note.string))
                        {
                            note.string = s;
                            note.fret = f;
                            note.midi_note = pitch;
                        } else if let Some((s, f)) =
                            new_tuning.best_fingering(pitch, 24, Some(note.string))
                        {
                            note.string = s;
                            note.fret = f;
                            note.midi_note = pitch;
                        }
                    }
                }
            }
        }
    }

    /// 用户切换 Tab 弦数时同步调弦并重映射音符（兼容旧调用）
    pub fn apply_tab_string_count(&mut self, strings: u8) {
        self.set_string_count(strings as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beat::{Beat, Voice};
    use crate::measure::Measure;
    use crate::midi::MidiChannel;
    use crate::note::Note;

    #[test]
    fn apply_midi_channel_keeps_file_program() {
        let mut track = Track {
            name: "Bass".to_string(),
            midi_program: 25,
            ..Track::default()
        };
        let ch = MidiChannel {
            channel: 2,
            instrument: 27,
            volume: 110,
            balance: 40,
            ..MidiChannel::default()
        };
        track.apply_midi_channel(&ch);
        assert_eq!(track.midi_program, 27, "不得按轨道名改写成 Fingered Bass");
        assert_eq!(track.midi_channel, 2);
        assert_eq!(track.volume, 110);
        assert_eq!(track.instrument_type, InstrumentType::ElectricGuitar);
    }

    #[test]
    fn staff_display_defaults_for_guitar_and_piano() {
        let guitar = StaffDisplay::default_for(27, 6, false);
        assert!(guitar.show_tab);
        assert_eq!(guitar.tab_strings, 6);
        assert!(!guitar.show_standard);

        let bass = StaffDisplay::default_for(33, 4, false);
        assert!(bass.show_tab);
        assert_eq!(bass.tab_strings, 4);

        let piano = StaffDisplay::default_for(0, 0, false);
        assert!(piano.show_standard);
        assert!(!piano.show_tab);
    }

    #[test]
    fn set_string_count_remaps_notes_by_pitch() {
        let mut track = Track::default();
        track.measures.push(Measure {
            voices: std::array::from_fn(|_| Voice::default()),
            ..Default::default()
        });
        // 1 弦空弦 E4 (64)
        track.measures[0].voices[0].beats.push(Beat {
            notes: vec![Note {
                string: 1,
                fret: 0,
                midi_note: 64,
                ..Default::default()
            }],
            ..Default::default()
        });
        // 改为 4 弦贝斯调弦后，E4 应落到更高品格的某弦
        track.tuning = Tuning::standard_bass();
        track.staff_display.tab_strings = 4;
        let old = Tuning::standard_guitar();
        track.remap_notes_preserving_pitch(&old);
        let n = &track.measures[0].voices[0].beats[0].notes[0];
        assert_eq!(n.midi_note, 64);
        assert_eq!(
            track.tuning.midi_note(n.string, n.fret),
            Some(64),
            "指法应仍发出原音高"
        );
    }

    #[test]
    fn apply_tab_string_count_switches_tuning_size() {
        let mut track = Track::default();
        track.apply_tab_string_count(4);
        assert!(track.staff_display.show_tab);
        assert_eq!(track.staff_display.tab_strings, 4);
        assert_eq!(track.string_count(), 4);

        track.apply_tab_string_count(6);
        assert_eq!(track.staff_display.tab_strings, 6);
        assert_eq!(track.string_count(), 6);
    }

    #[test]
    fn midi_note_name_formats() {
        assert_eq!(midi_note_name(40), "E2");
        assert_eq!(midi_note_name(64), "E4");
        assert_eq!(midi_note_name(60), "C4");
    }
}
