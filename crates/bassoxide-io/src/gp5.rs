//! GP5 (.gp5) 文件格式解析器。
//!
//! GP5 是 Guitar Pro 5.x 使用的二进制格式，社区逆向工程文档最为完善。
//! 本解析器参考 PyGuitarPro 和 TuxGuitar 的实现。

use bassoxide_core::beat::{Beat, Voice};
use bassoxide_core::chord::ChordDiagram;
use bassoxide_core::effects::*;
use bassoxide_core::lyrics::{Lyrics, LyricsLine};
use bassoxide_core::measure::{MasterBar, Measure, Marker, MAX_VOICES};
use bassoxide_core::midi::MidiChannel;
use bassoxide_core::note::{Note, NoteType};
use bassoxide_core::song::{Song, SongInfo};
use bassoxide_core::track::{GuitarString, Track, Tuning};
use bassoxide_core::types::*;
use tracing::debug;

use crate::binary::GpReader;
use crate::error::{IoError, Result};

/// 支持的 GP5 版本前缀
const GP5_VERSIONS: &[&str] = &[
    "FICHIER GUITAR PRO v5.00",
    "FICHIER GUITAR PRO v5.10",
];

/// 从字节数据解析 GP5 文件
pub fn parse_gp5(data: &[u8]) -> Result<Song> {
    let mut r = GpReader::new(data);
    let mut song = Song::default();

    // 1. 版本头
    let version = read_version(&mut r)?;
    debug!("GP5 version: {version}");
    song.version = version;

    // 2. 歌曲信息
    song.info = read_song_info(&mut r)?;

    // 3. 歌词
    song.lyrics = read_lyrics(&mut r)?;

    // 4. RSE master effect (GP5 特有，跳过)
    skip_rse_master_effect(&mut r)?;

    // 5. 页面设置 (跳过，使用默认值)
    skip_page_setup(&mut r)?;

    // 6. 速度
    song.tempo = read_tempo(&mut r)?;

    // 7. 调号 (全局初始)
    let _initial_key = r.read_i32()?; // key
    let _octave = r.read_u8()?;

    // 8. MIDI 通道
    song.midi_channels = read_midi_channels(&mut r)?;

    // 9. 排练方向 (GP5.10)
    skip_directions(&mut r)?;

    // 10. 小节数与轨道数
    let measure_count = r.read_i32()? as usize;
    let track_count = r.read_i32()? as usize;
    debug!("Measures: {measure_count}, Tracks: {track_count}");

    // 11. MasterBars (全局小节信息)
    song.master_bars = read_master_bars(&mut r, measure_count)?;

    // 12. Tracks
    song.tracks = read_tracks(&mut r, track_count)?;

    // 13. 小节音符数据 (measure × track 矩阵)
    read_measure_data(&mut r, &mut song, measure_count, track_count)?;

    Ok(song)
}

// ── 版本 ──

fn read_version(r: &mut GpReader) -> Result<String> {
    let version = r.read_byte_string_fixed(30)?;
    if !GP5_VERSIONS.iter().any(|v| version.starts_with(v)) {
        return Err(IoError::IncompatibleVersion(version));
    }
    Ok(version)
}

// ── 歌曲信息 ──

fn read_song_info(r: &mut GpReader) -> Result<SongInfo> {
    let title = r.read_int_byte_string()?;
    let subtitle = r.read_int_byte_string()?;
    let artist = r.read_int_byte_string()?;
    let album = r.read_int_byte_string()?;
    let words = r.read_int_byte_string()?;
    let music = r.read_int_byte_string()?;
    let copyright = r.read_int_byte_string()?;
    let tab_author = r.read_int_byte_string()?;
    let instructions = r.read_int_byte_string()?;

    // 注释行数
    let comment_count = r.read_i32()? as usize;
    let mut comments = Vec::with_capacity(comment_count);
    for _ in 0..comment_count {
        comments.push(r.read_int_byte_string()?);
    }

    Ok(SongInfo {
        title,
        subtitle,
        artist,
        album,
        words,
        music,
        copyright,
        tab_author,
        instructions,
        comments,
    })
}

// ── 歌词 ──

fn read_lyrics(r: &mut GpReader) -> Result<Lyrics> {
    let track_number = r.read_i32()? as u8;
    let mut lines = Vec::new();
    for _ in 0..5 {
        let start_measure = r.read_i32()? as u32;
        let text = r.read_int_string()?;
        if !text.is_empty() {
            lines.push(LyricsLine {
                start_measure,
                text,
            });
        }
    }
    Ok(Lyrics {
        track_number,
        lines,
    })
}

// ── RSE 跳过 ──

fn skip_rse_master_effect(r: &mut GpReader) -> Result<()> {
    // GP5 RSE master effect: 固定大小块
    // volume, eq band 有固定结构，需要跳过
    // master volume
    r.skip(4)?;
    // 10-band EQ: 每个 band 有 1 字节
    r.skip(10)?;
    // equalizer knobs 的值
    r.skip(1)?;
    Ok(())
}

fn skip_page_setup(r: &mut GpReader) -> Result<()> {
    // 页面设置由多个 int-byte-string 模板字符串组成
    // GP5 有 11 个模板字符串
    // page size
    r.skip(4)?; // width
    r.skip(4)?; // height
    // margins
    r.skip(4 * 4)?; // left, right, top, bottom
    // score size
    r.skip(4)?;
    // header/footer flags
    r.skip(2)?;

    // 11 个 title 模板字符串
    for _ in 0..11 {
        r.read_int_byte_string()?;
    }
    Ok(())
}

// ── 速度 ──

fn read_tempo(r: &mut GpReader) -> Result<u16> {
    // tempo 文本标记
    r.read_int_byte_string()?;
    let bpm = r.read_i32()? as u16;

    // GP5.10 有 tempo 隐藏标志
    if r.remaining() > 0 {
        let _hide = r.read_bool().ok();
    }

    Ok(bpm)
}

// ── MIDI 通道 ──

fn read_midi_channels(r: &mut GpReader) -> Result<Vec<MidiChannel>> {
    let mut channels = Vec::with_capacity(64);
    for i in 0..64 {
        let instrument = r.read_i32()? as u8;
        let volume = r.read_u8()?;
        let balance = r.read_u8()?;
        let chorus = r.read_u8()?;
        let reverb = r.read_u8()?;
        let phaser = r.read_u8()?;
        let tremolo = r.read_u8()?;
        // 2 bytes padding
        r.skip(2)?;

        channels.push(MidiChannel {
            channel: (i % 16) as u8,
            effect_channel: 0,
            instrument,
            volume,
            balance,
            chorus,
            reverb,
            phaser,
            tremolo,
        });
    }
    Ok(channels)
}

fn skip_directions(r: &mut GpReader) -> Result<()> {
    // GP5.10 排练方向（可变数量），这里采用保守策略
    // 实际上这些数据可能不存在于所有 GP5 文件中
    // 跳过的安全方式：读取到已知的下一段
    // 简化处理：跳过 musical directions
    // 19 个 short 值
    for _ in 0..19 {
        r.read_i16()?;
    }
    Ok(())
}

// ── MasterBars ──

fn read_master_bars(r: &mut GpReader, count: usize) -> Result<Vec<MasterBar>> {
    let mut bars = Vec::with_capacity(count);
    let mut prev_time = TimeSignature::default();
    let mut prev_key = KeySignature::default();

    for i in 0..count {
        let flags = r.read_u8()?;

        let mut bar = MasterBar::default();

        // bit 0: 拍号分子变化
        if flags & 0x01 != 0 {
            prev_time.numerator = r.read_u8()?;
        }
        // bit 1: 拍号分母变化
        if flags & 0x02 != 0 {
            let denom = r.read_u8()?;
            prev_time.denominator = match denom {
                1 => NoteValue::Whole,
                2 => NoteValue::Half,
                4 => NoteValue::Quarter,
                8 => NoteValue::Eighth,
                16 => NoteValue::Sixteenth,
                32 => NoteValue::ThirtySecond,
                _ => NoteValue::Quarter,
            };
        }
        bar.time_signature = prev_time;

        // bit 2: 反复开始
        if flags & 0x04 != 0 {
            bar.bar_line_start = BarLineType::RepeatOpen;
        }
        // bit 3: 反复结束
        if flags & 0x08 != 0 {
            let close_count = r.read_u8()?;
            bar.bar_line_end = BarLineType::RepeatClose;
            bar.repeat = Some(RepeatType::Close(close_count));
        }
        // bit 4: 替代结尾
        if flags & 0x10 != 0 {
            bar.alternate_endings = r.read_u8()?;
        }
        // bit 5: 排练标记
        if flags & 0x20 != 0 {
            let name = r.read_int_byte_string()?;
            let cr = r.read_u8()?;
            let cg = r.read_u8()?;
            let cb = r.read_u8()?;
            r.skip(1)?; // padding
            bar.marker = Some(Marker {
                name,
                color: Color::rgb(cr, cg, cb),
            });
        }
        // bit 6: 调号变化
        if flags & 0x40 != 0 {
            prev_key.key = r.read_i8()?;
            prev_key.is_minor = r.read_bool()?;
        }
        bar.key_signature = prev_key;

        // bit 7: 双小节线
        if flags & 0x80 != 0 {
            bar.bar_line_end = BarLineType::Double;
        }

        // GP5 额外数据：beam groups (4 bytes)
        if i == 0 || (flags & 0x03) != 0 {
            // 当拍号变化时，重新读取 beam groups
        }
        // beam eight notes grouping
        r.skip(4)?;

        // GP5: triplet feel
        if flags & 0x10 == 0 {
            // 没有替代结尾时才读取这个字节
            // 实际上 GP5 对这个字段的处理比较复杂
        }
        r.read_u8()?; // triplet feel

        bars.push(bar);
    }

    Ok(bars)
}

// ── Tracks ──

fn read_tracks(r: &mut GpReader, count: usize) -> Result<Vec<Track>> {
    let mut tracks = Vec::with_capacity(count);

    for i in 0..count {
        let flags = r.read_u8()?;
        let is_percussion = flags & 0x01 != 0;
        let _is_12_string = flags & 0x02 != 0;
        let _is_banjo = flags & 0x04 != 0;

        let name = r.read_byte_string_fixed(40)?;

        let string_count = r.read_i32()? as usize;
        let mut strings = Vec::with_capacity(string_count);
        for s in 0..7 {
            let tuning = r.read_i32()? as u8;
            if s < string_count {
                strings.push(GuitarString {
                    number: (s + 1) as u8,
                    tuning,
                });
            }
        }

        let midi_port = r.read_i32()? as u8;
        let midi_channel_index = r.read_i32()? as u8;
        let _effect_channel = r.read_i32()?;
        let fret_count = r.read_i32()? as u8;
        let capo = r.read_i32()? as u8;

        let cr = r.read_u8()?;
        let cg = r.read_u8()?;
        let cb = r.read_u8()?;
        r.skip(1)?; // padding

        // GP5 扩展数据
        // RSE settings (较多字段，需要跳过)
        skip_track_rse(r)?;

        let midi_channel = midi_channel_index.saturating_sub(1);

        tracks.push(Track {
            number: (i + 1) as u8,
            name,
            instrument_type: if is_percussion {
                InstrumentType::Drums
            } else {
                InstrumentType::ElectricGuitar
            },
            tuning: Tuning {
                name: String::new(),
                strings,
            },
            midi_channel,
            midi_port,
            midi_program: 25,
            midi_bank: 0,
            capo,
            fret_count,
            clef: if is_percussion { Clef::Tab } else { Clef::Treble },
            color: Color::rgb(cr, cg, cb),
            volume: 100,
            pan: 64,
            is_muted: false,
            is_solo: false,
            is_percussion,
            measures: Vec::new(),
        });
    }

    Ok(tracks)
}

fn skip_track_rse(r: &mut GpReader) -> Result<()> {
    // GP5 track RSE 数据块大小不固定
    // 安全跳过策略：读取已知字段
    // auto accentuation, midi bank, human playing, auto-let-ring
    r.skip(4 + 4 + 4 + 4)?;
    // RSE instrument/sound bank
    r.skip(4 + 4 + 4)?;
    // effect number
    r.skip(4)?;
    // eq
    r.skip(9)?;
    // instrument effect 1 label
    r.read_int_byte_string()?;
    // instrument effect 2 label
    r.read_int_byte_string()?;
    Ok(())
}

// ── 小节音符数据 ──

fn read_measure_data(
    r: &mut GpReader,
    song: &mut Song,
    measure_count: usize,
    track_count: usize,
) -> Result<()> {
    // 先为每个 track 初始化空 measures
    for track in song.tracks.iter_mut() {
        track.measures = vec![Measure::default(); measure_count];
    }

    for m in 0..measure_count {
        for t in 0..track_count {
            let measure = read_measure(r)?;
            if t < song.tracks.len() {
                song.tracks[t].measures[m] = measure;
            }
            // 每个小节之间有 1 字节分隔符（GP5.10）
            if r.remaining() > 0 {
                r.read_u8().ok();
            }
        }
    }

    Ok(())
}

fn read_measure(r: &mut GpReader) -> Result<Measure> {
    let mut measure = Measure::default();

    for v in 0..MAX_VOICES {
        let beat_count = r.read_i32()? as usize;
        let mut beats = Vec::with_capacity(beat_count);
        let mut tick = 0u32;

        for _ in 0..beat_count {
            let mut beat = read_beat(r)?;
            beat.start_tick = tick;
            tick += beat.ticks();
            beats.push(beat);
        }

        measure.voices[v] = Voice { beats };
    }

    // GP5: line break flag
    let line_break = r.read_u8()?;
    measure.line_break = line_break > 0;

    Ok(measure)
}

fn read_beat(r: &mut GpReader) -> Result<Beat> {
    let flags = r.read_u8()?;
    let mut beat = Beat::default();

    // bit 6: 休止符状态
    if flags & 0x40 != 0 {
        let beat_type = r.read_u8()?;
        beat.is_rest = beat_type == 0x02; // 0x02 = rest, 0x00 = empty
    }

    // 时值
    let duration_value = r.read_i8()?;
    beat.duration.value = NoteValue::from_gp_value(duration_value)
        .unwrap_or(NoteValue::Quarter);

    // bit 0: 附点
    if flags & 0x01 != 0 {
        beat.duration.dotted = true;
    }

    // bit 5: 连音符
    if flags & 0x20 != 0 {
        let tuplet_n = r.read_i32()? as u8;
        beat.duration.tuplet_numerator = tuplet_n;
        beat.duration.tuplet_denominator = match tuplet_n {
            3 => 2,
            5 | 6 => 4,
            7 => 4,
            9 | 10 => 8,
            11 | 12 => 8,
            13 => 8,
            _ => tuplet_n,
        };
    }

    // bit 1: 和弦图
    if flags & 0x02 != 0 {
        beat.chord = Some(read_chord(r)?);
    }

    // bit 2: 文本
    if flags & 0x04 != 0 {
        beat.text = Some(r.read_int_byte_string()?);
    }

    // bit 3: 拍级效果
    if flags & 0x08 != 0 {
        read_beat_effects(r, &mut beat)?;
    }

    // bit 4: 混音变化
    if flags & 0x10 != 0 {
        skip_mix_change(r)?;
    }

    // 音符掩码：哪些弦有音符
    let note_mask = r.read_u8()?;
    for string_num in (1..=7).rev() {
        let bit = 1 << (string_num - 1);
        if note_mask & bit != 0 {
            let note = read_note(r, string_num)?;
            beat.notes.push(note);
        }
    }

    // GP5: 额外 beat 属性
    // read flag for transpose
    r.skip(2)?;

    Ok(beat)
}

fn read_chord(r: &mut GpReader) -> Result<ChordDiagram> {
    let mut chord = ChordDiagram::default();

    let format = r.read_u8()?;

    if format == 0 {
        // 简单格式
        chord.name = r.read_int_byte_string()?;
        let diagram_start = r.read_i32()? as u8;
        chord.first_fret = diagram_start;
        if diagram_start > 0 {
            for i in 0..6 {
                chord.frets[i] = r.read_i32()? as i8;
            }
        }
    } else {
        // 完整格式 (GP5 新增的扩展和弦格式)
        r.skip(1)?; // sharp/flat
        r.skip(3)?; // blank
        r.read_u8()?; // root
        r.skip(1)?; // major/minor type
        r.read_u8()?; // chord type extension
        r.skip(4)?; // bass note
        r.skip(4)?; // add
        chord.name = r.read_byte_string_fixed(20)?;
        r.skip(2)?; // blank
        r.read_u8()?; // fifth
        r.read_u8()?; // ninth
        chord.first_fret = r.read_i32()? as u8;
        for i in 0..7 {
            let fret = r.read_i32()? as i8;
            if i < 6 {
                chord.frets[i] = fret;
            }
        }
        // barre count
        let barre_count = r.read_u8()?;
        // barre frets (5 bytes)
        let barre_frets: Vec<u8> = (0..5).map(|_| r.read_u8().unwrap_or(0)).collect();
        // barre start strings (5 bytes)
        let barre_starts: Vec<u8> = (0..5).map(|_| r.read_u8().unwrap_or(0)).collect();
        // barre end strings (5 bytes)
        let barre_ends: Vec<u8> = (0..5).map(|_| r.read_u8().unwrap_or(0)).collect();
        // omissions (7 bytes), fingering (2*7 bytes)
        r.skip(7 + 14)?;
        // show fingering
        r.skip(1)?;

        for i in 0..barre_count as usize {
            if i < 5 {
                chord.barre.push(bassoxide_core::chord::Barre {
                    fret: barre_frets[i],
                    start_string: barre_starts[i],
                    end_string: barre_ends[i],
                });
            }
        }
    }

    Ok(chord)
}

fn read_beat_effects(r: &mut GpReader, beat: &mut Beat) -> Result<()> {
    let flags1 = r.read_u8()?;
    let flags2 = r.read_u8()?;

    // flag1 bit 5: tapping/slapping/popping
    if flags1 & 0x20 != 0 {
        let effect_type = r.read_u8()?;
        match effect_type {
            1 => beat.effects.push(BeatEffect::SlapPop), // tapping
            2 => beat.effects.push(BeatEffect::SlapPop), // slapping
            3 => beat.effects.push(BeatEffect::SlapPop), // popping
            _ => {}
        }
    }

    // flag2 bit 2: tremolo bar
    if flags2 & 0x04 != 0 {
        let bend = read_bend_effect(r)?;
        beat.effects.push(BeatEffect::WhammyBar(bend));
    }

    // flag1 bit 6: stroke direction
    if flags1 & 0x40 != 0 {
        let up_speed = r.read_u8()?;
        let down_speed = r.read_u8()?;
        let (dir, speed_val) = if down_speed > 0 {
            (StrokeDirection::Down, down_speed)
        } else {
            (StrokeDirection::Up, up_speed)
        };
        let speed = match speed_val {
            1 => StrokeSpeed::Fastest,
            2 => StrokeSpeed::Fast,
            3 => StrokeSpeed::Medium,
            _ => StrokeSpeed::Slow,
        };
        beat.effects.push(BeatEffect::Stroke(dir, speed));
    }

    // flag2 bit 1: pick stroke
    if flags2 & 0x02 != 0 {
        let _pick = r.read_u8()?;
    }

    // flag1 bit 2: fade in
    if flags1 & 0x04 != 0 {
        beat.effects.push(BeatEffect::FadeIn);
    }

    Ok(())
}

fn skip_mix_change(r: &mut GpReader) -> Result<()> {
    let _instrument = r.read_i8()?;
    // GP5 RSE instrument
    r.skip(16)?;
    let volume = r.read_i8()?;
    let pan = r.read_i8()?;
    let chorus = r.read_i8()?;
    let reverb = r.read_i8()?;
    let phaser = r.read_i8()?;
    let tremolo = r.read_i8()?;

    // tempo text
    r.read_int_byte_string()?;
    let tempo = r.read_i32()?;

    // transition durations
    if volume >= 0 { r.skip(1)?; }
    if pan >= 0 { r.skip(1)?; }
    if chorus >= 0 { r.skip(1)?; }
    if reverb >= 0 { r.skip(1)?; }
    if phaser >= 0 { r.skip(1)?; }
    if tremolo >= 0 { r.skip(1)?; }
    if tempo >= 0 {
        r.skip(1)?;
        // hidden tempo
        r.skip(1)?;
    }

    // GP5: 应用到所有轨道的标志
    r.skip(1)?;

    Ok(())
}

fn read_note(r: &mut GpReader, string_num: u8) -> Result<Note> {
    let flags = r.read_u8()?;
    let mut note = Note {
        string: string_num,
        ..Default::default()
    };

    // bit 5: 音符类型
    if flags & 0x20 != 0 {
        let note_type = r.read_u8()?;
        note.note_type = match note_type {
            1 => NoteType::Normal,
            2 => NoteType::Tie,
            3 => NoteType::Dead,
            _ => NoteType::Normal,
        };
    }

    // bit 4: 力度
    if flags & 0x10 != 0 {
        // GP5: 力度用 i8 编码
        let dynamic = r.read_i8()?;
        note.velocity = dynamic.max(0) as u8;
    }

    // bit 0: 品格
    if flags & 0x20 != 0 {
        let fret = r.read_u8()?;
        note.fret = fret as i8;
    }

    // bit 7: 右手指法
    if flags & 0x80 != 0 {
        let _left = r.read_i8()?;
        let _right = r.read_i8()?;
    }

    // bit 1: 额外属性
    if flags & 0x02 != 0 {
        // GP5 "time-independent duration" 等
        r.skip(1)?;
        r.skip(1)?;
    }

    // bit 3: 音符效果
    if flags & 0x08 != 0 {
        read_note_effects(r, &mut note)?;
    }

    Ok(note)
}

fn read_note_effects(r: &mut GpReader, note: &mut Note) -> Result<()> {
    let flags1 = r.read_u8()?;
    let flags2 = r.read_u8()?;

    // flag1 bit 0: bend
    if flags1 & 0x01 != 0 {
        let bend = read_bend_effect(r)?;
        note.effects.push(NoteEffect::Bend(bend));
    }

    // flag1 bit 1: grace note
    if flags1 & 0x02 != 0 {
        let fret = r.read_u8()?;
        let velocity = r.read_u8()?;
        let transition = r.read_u8()?;
        let dur = r.read_u8()?;

        // GP5: 额外标志
        let gp5_flags = r.read_u8()?;

        note.effects.push(NoteEffect::GraceNote(GraceNote {
            fret,
            velocity,
            duration: match dur {
                1 => GraceNoteDuration::Sixteenth,
                2 => GraceNoteDuration::TwentyFourth,
                3 => GraceNoteDuration::ThirtySecond,
                _ => GraceNoteDuration::Sixteenth,
            },
            is_on_beat: gp5_flags & 0x01 != 0,
            is_dead: gp5_flags & 0x02 != 0,
            transition: match transition {
                0 => GraceNoteTransition::None,
                1 => GraceNoteTransition::Slide,
                2 => GraceNoteTransition::Bend,
                3 => GraceNoteTransition::HammerOn,
                _ => GraceNoteTransition::None,
            },
        }));
    }

    // flag2 bit 0: tremolo picking
    if flags2 & 0x01 != 0 {
        let speed = r.read_u8()?;
        note.effects.push(NoteEffect::TremoloPicking(match speed {
            1 => TremoloPickingSpeed::Eighth,
            2 => TremoloPickingSpeed::Sixteenth,
            3 => TremoloPickingSpeed::ThirtySecond,
            _ => TremoloPickingSpeed::Sixteenth,
        }));
    }

    // flag2 bit 1: slide
    if flags2 & 0x02 != 0 {
        let slide_flags = r.read_u8()?;
        let mut slides = Vec::new();
        if slide_flags & 0x01 != 0 { slides.push(SlideType::ShiftSlide); }
        if slide_flags & 0x02 != 0 { slides.push(SlideType::LegatoSlide); }
        if slide_flags & 0x04 != 0 { slides.push(SlideType::OutDownwards); }
        if slide_flags & 0x08 != 0 { slides.push(SlideType::OutUpwards); }
        if slide_flags & 0x10 != 0 { slides.push(SlideType::IntoFromBelow); }
        if slide_flags & 0x20 != 0 { slides.push(SlideType::IntoFromAbove); }
        if !slides.is_empty() {
            note.effects.push(NoteEffect::Slide(slides));
        }
    }

    // flag2 bit 2: harmonic
    if flags2 & 0x04 != 0 {
        let harmonic_type = r.read_u8()?;
        note.effects.push(NoteEffect::Harmonic(HarmonicEffect {
            harmonic_type: match harmonic_type {
                1 => HarmonicType::Natural,
                2 => HarmonicType::Artificial,
                3 => HarmonicType::Tap,
                4 => HarmonicType::Pinch,
                5 => HarmonicType::Semi,
                _ => HarmonicType::Natural,
            },
            fret_offset: if harmonic_type >= 2 {
                // AH/TH 有额外数据
                Some(r.read_u8().unwrap_or(0))
            } else {
                None
            },
        }));
        // AH 有更多数据
        if harmonic_type == 2 {
            r.skip(2).ok(); // AH key/octave
        } else if harmonic_type == 3 {
            // tap harmonic fret
            // already read above
        }
    }

    // flag2 bit 3: trill
    if flags2 & 0x08 != 0 {
        let fret = r.read_u8()?;
        let speed = r.read_u8()?;
        note.effects.push(NoteEffect::Trill(TrillEffect {
            fret,
            duration: match speed {
                1 => TrillSpeed::Sixteenth,
                2 => TrillSpeed::ThirtySecond,
                3 => TrillSpeed::SixtyFourth,
                _ => TrillSpeed::Sixteenth,
            },
        }));
    }

    // flag1 bit 2: hammer-on / pull-off
    if flags1 & 0x04 != 0 {
        note.effects.push(NoteEffect::HammerOnPullOff(HammerOnPullOff::HammerOn));
    }

    // flag1 bit 3: let ring
    if flags1 & 0x08 != 0 {
        note.effects.push(NoteEffect::LetRing);
    }

    // flag1 bit 4: left hand fingering
    // flag1 bit 5: staccato / palm mute
    if flags1 & 0x10 != 0 {
        note.effects.push(NoteEffect::Staccato);
    }
    if flags1 & 0x20 != 0 {
        note.effects.push(NoteEffect::PalmMute);
    }

    // flag2 bit 4: vibrato
    if flags2 & 0x10 != 0 {
        note.effects.push(NoteEffect::Vibrato(VibratoType::Finger, VibratoSpeed::Medium));
    }

    Ok(())
}

fn read_bend_effect(r: &mut GpReader) -> Result<BendEffect> {
    let bend_type_val = r.read_u8()?;
    let _value = r.read_i32()?;
    let point_count = r.read_i32()? as usize;

    let mut points = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        let position = r.read_i32()?;
        let value = r.read_i32()?;
        let vibrato = r.read_u8()? != 0;
        points.push(BendPoint {
            position: (position as f64 * 12.0 / 60.0) as u8,
            value: (value as f64 / 25.0) as i8,
            vibrato,
        });
    }

    let bend_type = match bend_type_val {
        1 => BendType::Bend,
        2 => BendType::BendRelease,
        3 => BendType::BendReleaseBend,
        4 => BendType::Prebend,
        5 => BendType::PrebendRelease,
        6 => BendType::Dip,
        _ => BendType::Bend,
    };

    Ok(BendEffect { bend_type, points })
}
