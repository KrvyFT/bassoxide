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
}

/// 轨道谱面显示配置（可多选；四线谱与六线谱互斥）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffDisplay {
    /// 五线谱
    pub show_standard: bool,
    /// Tab（四线/六线）
    pub show_tab: bool,
    /// Tab 弦数：4 或 6
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
                tab_strings: 6,
            };
        }
        let is_guitar_bass = (24..=39).contains(&midi_program) && string_count > 0;
        if is_guitar_bass {
            let tab_strings = if string_count <= 4 { 4 } else { 6 };
            Self {
                show_standard: false,
                show_tab: true,
                tab_strings,
            }
        } else {
            Self::default()
        }
    }

    /// 启用四线谱（与六线互斥）
    pub fn enable_four_string_tab(&mut self) {
        self.show_tab = true;
        self.tab_strings = 4;
    }

    /// 启用六线谱（与四线互斥）
    pub fn enable_six_string_tab(&mut self) {
        self.show_tab = true;
        self.tab_strings = 6;
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

    /// 用户在弹窗中切换四/六线时同步标准调弦（显式操作）
    pub fn apply_tab_string_count(&mut self, strings: u8) {
        match strings {
            4 => {
                self.staff_display.enable_four_string_tab();
                if self.string_count() != 4 {
                    self.tuning = Tuning::standard_bass();
                }
            }
            6 => {
                self.staff_display.enable_six_string_tab();
                if self.string_count() != 6 {
                    self.tuning = Tuning::standard_guitar();
                }
            }
            _ => {
                self.staff_display.tab_strings = strings.clamp(4, 6);
                self.staff_display.show_tab = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::MidiChannel;

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
    fn apply_tab_string_count_switches_tuning() {
        let mut track = Track::default();
        track.apply_tab_string_count(4);
        assert!(track.staff_display.show_tab);
        assert_eq!(track.staff_display.tab_strings, 4);
        assert_eq!(track.string_count(), 4);

        track.apply_tab_string_count(6);
        assert_eq!(track.staff_display.tab_strings, 6);
        assert_eq!(track.string_count(), 6);
    }
}
