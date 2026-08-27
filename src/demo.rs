//! 内置演示乐谱（用于无外部 GP 文件时的 UI/播放验证）。

use bassoxide_core::beat::{Beat, Voice};
use bassoxide_core::measure::{MasterBar, Measure};
use bassoxide_core::note::Note;
use bassoxide_core::song::{Song, SongInfo};
use bassoxide_core::track::{StaffDisplay, Track, Tuning};
use bassoxide_core::types::{Duration, InstrumentType};

fn note(string: u8, fret: i8, midi: u8) -> Note {
    Note {
        string,
        fret,
        midi_note: midi,
        velocity: 100,
        ..Note::default()
    }
}

fn beat_quarter(notes: Vec<Note>) -> Beat {
    Beat {
        duration: Duration::default(),
        notes,
        is_rest: false,
        ..Beat::default()
    }
}

fn measure_with_beats(beats: Vec<Beat>) -> Measure {
    let mut voices = [
        Voice::default(),
        Voice::default(),
        Voice::default(),
        Voice::default(),
    ];
    voices[0].beats = beats;
    Measure {
        voices,
        ..Measure::default()
    }
}

/// 构建含吉他 / 贝斯 / 键盘 / 鼓 四轨的演示曲
pub fn build_demo_song() -> Song {
    let mut song = Song {
        version: "DEMO".into(),
        info: SongInfo {
            title: "Material You Demo".into(),
            artist: "Bassoxide".into(),
            ..SongInfo::default()
        },
        tempo: 110,
        ..Song::default()
    };

    for _ in 0..4 {
        song.master_bars.push(MasterBar {
            tempo: Some(110),
            ..MasterBar::default()
        });
    }

    // 电吉他 — 六线谱
    let mut guitar = Track {
        number: 1,
        name: "Electric Guitar".into(),
        instrument_type: InstrumentType::ElectricGuitar,
        tuning: Tuning::standard_guitar(),
        midi_program: 27,
        midi_channel: 0,
        staff_display: StaffDisplay {
            show_standard: false,
            show_tab: true,
            tab_strings: 6,
        },
        ..Track::default()
    };
    guitar.measures = (0..4)
        .map(|i| {
            let frets = [0i8, 2, 3, 2];
            let beats = frets
                .iter()
                .map(|f| {
                    let midi = (40i16 + i16::from(*f) + i as i16).clamp(0, 127) as u8;
                    beat_quarter(vec![note(6, *f, midi)])
                })
                .collect();
            measure_with_beats(beats)
        })
        .collect();

    // 电贝斯 — 四线谱
    let mut bass = Track {
        number: 2,
        name: "Electric Bass".into(),
        instrument_type: InstrumentType::Bass,
        tuning: Tuning::standard_bass(),
        midi_program: 33,
        midi_channel: 1,
        staff_display: StaffDisplay {
            show_standard: false,
            show_tab: true,
            tab_strings: 4,
        },
        ..Track::default()
    };
    bass.measures = (0..4)
        .map(|_| {
            measure_with_beats(vec![
                beat_quarter(vec![note(4, 0, 28)]),
                beat_quarter(vec![note(4, 3, 31)]),
                beat_quarter(vec![note(3, 0, 33)]),
                beat_quarter(vec![note(3, 2, 35)]),
            ])
        })
        .collect();

    // 键盘 — 五线谱
    let mut keys = Track {
        number: 3,
        name: "Electric Piano".into(),
        instrument_type: InstrumentType::Piano,
        tuning: Tuning {
            name: String::new(),
            strings: vec![],
        },
        midi_program: 4,
        midi_channel: 2,
        staff_display: StaffDisplay {
            show_standard: true,
            show_tab: false,
            tab_strings: 6,
        },
        ..Track::default()
    };
    keys.measures = (0..4)
        .map(|_| {
            measure_with_beats(vec![
                beat_quarter(vec![Note {
                    midi_note: 60,
                    velocity: 90,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 64,
                    velocity: 90,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 67,
                    velocity: 90,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 72,
                    velocity: 90,
                    ..Note::default()
                }]),
            ])
        })
        .collect();

    // 鼓
    let mut drums = Track {
        number: 4,
        name: "Drums".into(),
        instrument_type: InstrumentType::Drums,
        is_percussion: true,
        midi_bank: 128,
        midi_program: 0,
        midi_channel: 9,
        staff_display: StaffDisplay {
            show_standard: true,
            show_tab: false,
            tab_strings: 6,
        },
        tuning: Tuning {
            name: String::new(),
            strings: vec![],
        },
        ..Track::default()
    };
    drums.measures = (0..4)
        .map(|_| {
            measure_with_beats(vec![
                beat_quarter(vec![Note {
                    midi_note: 36,
                    velocity: 110,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 42,
                    velocity: 80,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 38,
                    velocity: 100,
                    ..Note::default()
                }]),
                beat_quarter(vec![Note {
                    midi_note: 42,
                    velocity: 80,
                    ..Note::default()
                }]),
            ])
        })
        .collect();

    song.tracks = vec![guitar, bass, keys, drums];
    song
}
