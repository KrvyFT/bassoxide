//! 谱面编辑命令：光标移动、插删音、改弦、附点与效果开关。

use std::time::Instant;

use bassoxide_core::beat::Beat;
use bassoxide_core::effects::{HammerOnPullOff, NoteEffect, SlideType};
use bassoxide_core::measure::{check_voice_duration, MeasureDurationStatus};
use bassoxide_core::note::{Note, NoteType};
use bassoxide_core::song::Song;
use bassoxide_core::types::{Duration, NoteValue};

use crate::state::{AppState, CursorPosition};

/// 数字品格输入缓冲（支持两位数）
#[derive(Debug, Clone)]
pub struct FretInputBuffer {
    pub digits: String,
    pub last_at: Instant,
}

impl Default for FretInputBuffer {
    fn default() -> Self {
        Self {
            digits: String::new(),
            last_at: Instant::now(),
        }
    }
}

impl FretInputBuffer {
    const TIMEOUT_MS: u128 = 600;

    pub fn push_digit(&mut self, d: char) -> Option<i8> {
        let now = Instant::now();
        if now.duration_since(self.last_at).as_millis() > Self::TIMEOUT_MS {
            self.digits.clear();
        }
        self.last_at = now;
        if !d.is_ascii_digit() {
            return None;
        }
        self.digits.push(d);
        if self.digits.len() >= 2 {
            let fret: i8 = self.digits.parse().unwrap_or(0).min(24);
            self.digits.clear();
            return Some(fret);
        }
        let fret: i8 = self.digits.parse().unwrap_or(0);
        Some(fret)
    }

    pub fn clear(&mut self) {
        self.digits.clear();
    }
}

/// 光标移动方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Left,
    Right,
    UpString,
    DownString,
}

fn clamp_cursor(song: &Song, cursor: &mut CursorPosition) {
    if song.tracks.is_empty() {
        *cursor = CursorPosition::default();
        return;
    }
    cursor.track = cursor.track.min(song.tracks.len() - 1);
    let track = &song.tracks[cursor.track];
    let string_count = track.tuning.string_count().max(1);
    if track.measures.is_empty() {
        cursor.measure = 0;
        cursor.beat = 0;
        cursor.string = 1;
        return;
    }
    cursor.measure = cursor.measure.min(track.measures.len() - 1);
    let beats = track.measures[cursor.measure].primary_voice().beats.len();
    if beats == 0 {
        cursor.beat = 0;
    } else {
        cursor.beat = cursor.beat.min(beats - 1);
    }
    cursor.string = cursor.string.clamp(1, string_count as u8);
}

fn sync_track(state: &mut AppState) {
    if let Some(song) = state.song.as_ref() {
        if !song.tracks.is_empty() {
            state.cursor.track = state.selected_track.min(song.tracks.len() - 1);
            clamp_cursor(song, &mut state.cursor);
        }
    }
}

/// 移动编辑光标（跨小节）
pub fn move_cursor(state: &mut AppState, dir: CursorMove) {
    sync_track(state);
    let Some(song) = state.song.as_ref() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track = &song.tracks[state.cursor.track];
    let string_count = track.tuning.string_count().max(1);

    match dir {
        CursorMove::UpString => {
            if state.cursor.string > 1 {
                state.cursor.string -= 1;
            }
        }
        CursorMove::DownString => {
            if (state.cursor.string as usize) < string_count {
                state.cursor.string += 1;
            }
        }
        CursorMove::Left => {
            if state.cursor.beat > 0 {
                state.cursor.beat -= 1;
            } else if state.cursor.measure > 0 {
                let prev = state.cursor.measure - 1;
                let prev_beats = track.measures[prev].primary_voice().beats.len();
                state.cursor.measure = prev;
                state.cursor.beat = prev_beats.saturating_sub(1);
            }
        }
        CursorMove::Right => {
            let beats = track
                .measures
                .get(state.cursor.measure)
                .map(|m| m.primary_voice().beats.len())
                .unwrap_or(0);
            if beats > 0 && state.cursor.beat + 1 < beats {
                state.cursor.beat += 1;
            } else if state.cursor.measure + 1 < track.measures.len() {
                state.cursor.measure += 1;
                state.cursor.beat = 0;
            }
        }
    }
    if let Some(song) = state.song.as_ref() {
        clamp_cursor(song, &mut state.cursor);
    }
    state.fret_input.clear();
}

fn ensure_beat_exists(state: &mut AppState) {
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let track = &mut song.tracks[track_idx];
    if track.measures.is_empty() {
        return;
    }
    state.cursor.measure = state.cursor.measure.min(track.measures.len() - 1);
    let voice = track.measures[state.cursor.measure].primary_voice_mut();
    if voice.beats.is_empty() {
        voice.beats.push(Beat {
            duration: state.edit_tool.slot_duration(),
            is_rest: true,
            ..Beat::default()
        });
        state.cursor.beat = 0;
    } else {
        state.cursor.beat = state.cursor.beat.min(voice.beats.len() - 1);
    }
}

/// Ctrl+↑/↓：将当前弦音符移到邻弦（按音高重映射品格）；无音符则只移光标
pub fn change_note_string(state: &mut AppState, delta: i8) {
    if delta == 0 {
        return;
    }
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let string_count = song.tracks[track_idx].tuning.string_count().max(1) as u8;
    let cur = state.cursor.string;
    let target = (cur as i16 + delta as i16).clamp(1, string_count as i16) as u8;
    if target == cur {
        return;
    }

    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    let open = song.tracks[track_idx]
        .tuning
        .strings
        .iter()
        .find(|s| s.number == target)
        .map(|s| s.tuning)
        .unwrap_or(40);
    let track = &mut song.tracks[track_idx];
    let mut moved = false;
    if let Some(measure) = track.measures.get_mut(measure_idx) {
        if let Some(beat) = measure.primary_voice_mut().beats.get_mut(beat_idx) {
            if let Some(note) = beat.note_on_string(cur).cloned() {
                if beat.note_on_string(target).is_none() {
                    let pitch = note.midi_note;
                    let fret = (pitch as i16 - open as i16).clamp(-24, 24) as i8;
                    if let Some(n) = beat.note_on_string_mut(cur) {
                        n.string = target;
                        n.fret = fret;
                        n.midi_note = pitch;
                        moved = true;
                    }
                }
            }
        }
    }

    state.cursor.string = target;
    if moved {
        state.needs_relayout = true;
        state.status_message = format!("音符移至第 {} 弦", target);
        refresh_duration_status(state);
    } else {
        state.status_message = format!("光标第 {} 弦", target);
    }
    state.fret_input.clear();
}

/// 在光标处插入音符（该弦 fret=0）；已有则只提示
pub fn insert_note(state: &mut AppState) {
    ensure_beat_exists(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    let string = state.cursor.string;
    let open_midi = song.tracks[track_idx]
        .tuning
        .midi_note(string, 0)
        .unwrap_or(40);
    let track = &mut song.tracks[track_idx];
    let Some(beat) = track
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
    else {
        return;
    };
    if beat.note_on_string(string).is_some() {
        state.status_message = "已有该弦音符".into();
        return;
    }
    beat.is_rest = false;
    beat.notes.push(Note {
        string,
        fret: 0,
        midi_note: open_midi,
        ..Note::default()
    });
    state.needs_relayout = true;
    state.status_message = "已插入音符".into();
    refresh_duration_status(state);
    state.fret_input.clear();
}

/// 删除光标弦音符；拍空则标休止
pub fn delete_note(state: &mut AppState) {
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    let string = state.cursor.string;
    let track = &mut song.tracks[track_idx];
    let Some(beat) = track
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
    else {
        state.status_message = "该弦无音符".into();
        return;
    };
    let before = beat.notes.len();
    beat.notes.retain(|n| n.string != string);
    if beat.notes.is_empty() {
        beat.is_rest = true;
    }
    if beat.notes.len() < before {
        state.needs_relayout = true;
        state.status_message = "已删除音符".into();
        refresh_duration_status(state);
    } else {
        state.status_message = "该弦无音符".into();
    }
    state.fret_input.clear();
}

/// 设置光标弦品格；无音符则创建。支持负品格（显示为负数）。
pub fn set_fret(state: &mut AppState, fret: i8) {
    ensure_beat_exists(state);
    let fret = fret.clamp(-24, 24);
    let targets = edit_targets(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let mut count = 0usize;
    for target in &targets {
        let midi = song.tracks[track_idx]
            .tuning
            .midi_note(target.string, fret)
            .unwrap_or(40);
        let track = &mut song.tracks[track_idx];
        let Some(beat) = track
            .measures
            .get_mut(target.measure)
            .and_then(|m| m.primary_voice_mut().beats.get_mut(target.beat))
        else {
            continue;
        };
        beat.is_rest = false;
        if let Some(note) = beat.note_on_string_mut(target.string) {
            note.fret = fret;
            note.midi_note = midi;
            if note.note_type == NoteType::Dead {
                note.note_type = NoteType::Normal;
            }
        } else {
            beat.notes.push(Note {
                string: target.string,
                fret,
                midi_note: midi,
                ..Note::default()
            });
        }
        count += 1;
    }
    state.needs_relayout = true;
    state.status_message = if count > 1 {
        format!("品格 {}（{} 个音符）", fret, count)
    } else {
        format!("品格 {}", fret)
    };
    refresh_duration_status(state);
    state.fret_input.clear();
}

/// 当前编辑目标：多选音符，否则仅光标格
fn edit_targets(state: &AppState) -> Vec<crate::state::NoteRef> {
    if !state.selection.notes.is_empty() {
        let mut v: Vec<_> = state.selection.notes.iter().copied().collect();
        v.sort_by_key(|n| (n.measure, n.beat, n.string));
        return v;
    }
    if let Some(m) = state.selection.measure {
        let mut v = Vec::new();
        if let Some(song) = state.song.as_ref() {
            if let Some(track) = song.tracks.get(state.cursor.track) {
                if let Some(measure) = track.measures.get(m) {
                    for (bi, beat) in measure.primary_voice().beats.iter().enumerate() {
                        for note in &beat.notes {
                            v.push(crate::state::NoteRef {
                                measure: m,
                                beat: bi,
                                string: note.string,
                            });
                        }
                    }
                }
            }
        }
        if !v.is_empty() {
            return v;
        }
    }
    vec![crate::state::NoteRef::from(state.cursor)]
}

/// 品格 ±1（可越过 0 到负数）
pub fn nudge_fret(state: &mut AppState, delta: i8) {
    if delta == 0 {
        return;
    }
    sync_track(state);
    let targets = edit_targets(state);
    let Some(song) = state.song.as_ref() else {
        return;
    };
    let track = match song.tracks.get(state.cursor.track) {
        Some(t) => t,
        None => return,
    };
    // 以光标格（或多选首个）为基准读取当前品格
    let anchor = targets.first().copied().unwrap_or_else(|| state.cursor.into());
    let current = track
        .measures
        .get(anchor.measure)
        .and_then(|m| m.primary_voice().beats.get(anchor.beat))
        .and_then(|b| b.note_on_string(anchor.string))
        .map(|n| n.fret)
        .unwrap_or(0);
    set_fret(state, current.saturating_add(delta));
}

/// 切换附点
pub fn toggle_dotted(state: &mut AppState) {
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    if let Some(beat) = song.tracks[track_idx]
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
    {
        if beat.duration.dotted {
            beat.duration.dotted = false;
        } else {
            beat.duration.dotted = true;
            beat.duration.double_dotted = false;
        }
        state.needs_relayout = true;
        state.status_message = "切换附点".into();
        refresh_duration_status(state);
    }
}

/// 设置拍时值
pub fn set_duration(state: &mut AppState, value: NoteValue) {
    sync_track(state);
    // 同步左侧工具时值，并按此时值重划分当前轨道小节空格
    state.edit_tool.duration = value;
    apply_duration_grid(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    if let Some(beat) = song.tracks[track_idx]
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
    {
        beat.duration.value = value;
        beat.duration.dotted = state.edit_tool.dotted;
        state.needs_relayout = true;
        state.status_message = format!("时值 {:?}", value);
        refresh_duration_status(state);
    }
}

/// 选用左侧工具：音符 / 休止符（带时值）会重划分小节空格；标记打开编辑器
pub fn select_edit_tool(
    state: &mut AppState,
    kind: crate::state::EditToolKind,
    duration: Option<NoteValue>,
) {
    use crate::state::EditToolKind;
    if let Some(d) = duration {
        state.edit_tool.duration = d;
    }
    state.edit_tool.kind = kind;
    match kind {
        EditToolKind::Note | EditToolKind::Rest => {
            apply_duration_grid(state);
            let slots = slots_per_measure(state).unwrap_or(0);
            let label = match kind {
                EditToolKind::Note => "音符",
                EditToolKind::Rest => "休止符",
                EditToolKind::Marker => "标记",
            };
            state.status_message = format!(
                "工具: {} {:?} · 每小节 {} 格",
                label, state.edit_tool.duration, slots
            );
        }
        EditToolKind::Marker => {
            state.status_message = "工具: 小节标记".into();
            state.marker_editor_open = true;
            if let Some(song) = state.song.as_ref() {
                if let Some(mb) = song.master_bar(state.cursor.measure) {
                    state.marker_edit_name = mb
                        .marker
                        .as_ref()
                        .map(|m| m.name.clone())
                        .unwrap_or_default();
                }
            }
        }
    }
    state.fret_input.clear();
}

/// 当前拍号下，所选时值能整除时的每小节空格数
pub fn slots_per_measure(state: &AppState) -> Option<usize> {
    let song = state.song.as_ref()?;
    let measure = state.cursor.measure;
    let master = song.master_bar(measure)?;
    let measure_ticks = master.time_signature.measure_ticks();
    let slot = state.edit_tool.slot_duration().ticks();
    if slot == 0 || measure_ticks % slot != 0 {
        return None;
    }
    Some((measure_ticks / slot) as usize)
}

/// 按当前工具时值，将当前轨道每个小节重分为等长空格（尽量按 tick 保留原音符）
pub fn apply_duration_grid(state: &mut AppState) {
    sync_track(state);
    let slot_dur = state.edit_tool.slot_duration();
    let slot_ticks = slot_dur.ticks();
    if slot_ticks == 0 {
        return;
    }
    let track_idx = state.cursor.track;
    let cursor_measure = state.cursor.measure;
    let cursor_beat = state.cursor.beat;
    let cursor_tick_hint = {
        // 尽量保持光标所在 tick，划分后落回对应空格
        let mut tick = 0u32;
        if let Some(song) = state.song.as_ref() {
            if let Some(track) = song.tracks.get(track_idx) {
                if let Some(m) = track.measures.get(cursor_measure) {
                    for (i, b) in m.primary_voice().beats.iter().enumerate() {
                        if i >= cursor_beat {
                            break;
                        }
                        tick = tick.saturating_add(b.ticks());
                    }
                }
            }
        }
        tick
    };

    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = track_idx.min(song.tracks.len() - 1);
    let n_measures = song.tracks[track_idx].measures.len();
    let mut changed = false;

    for mi in 0..n_measures {
        let measure_ticks = song
            .master_bar(mi)
            .map(|m| m.time_signature.measure_ticks())
            .unwrap_or(3840);
        if measure_ticks % slot_ticks != 0 {
            continue;
        }
        let n_slots = (measure_ticks / slot_ticks) as usize;
        if n_slots == 0 {
            continue;
        }

        let old_beats = song.tracks[track_idx].measures[mi]
            .primary_voice()
            .beats
            .clone();
        // 已是目标网格则跳过（避免无谓重排）
        if old_beats.len() == n_slots
            && old_beats
                .iter()
                .all(|b| b.duration.ticks() == slot_ticks)
        {
            continue;
        }

        let mut by_tick: Vec<(u32, Vec<Note>)> = Vec::new();
        let mut t = 0u32;
        for b in &old_beats {
            if !b.notes.is_empty() {
                by_tick.push((t, b.notes.clone()));
            }
            t = t.saturating_add(b.ticks());
        }

        let mut new_beats = Vec::with_capacity(n_slots);
        for si in 0..n_slots {
            let start = si as u32 * slot_ticks;
            let end = start + slot_ticks;
            let mut notes = Vec::new();
            for (tick, ns) in &by_tick {
                if *tick >= start && *tick < end {
                    for n in ns {
                        if notes.iter().all(|x: &Note| x.string != n.string) {
                            notes.push(n.clone());
                        }
                    }
                }
            }
            let is_rest = notes.is_empty();
            new_beats.push(Beat {
                duration: slot_dur,
                notes,
                is_rest,
                start_tick: start,
                ..Beat::default()
            });
        }
        song.tracks[track_idx].measures[mi]
            .primary_voice_mut()
            .beats = new_beats;
        changed = true;
    }

    if changed {
        state.needs_relayout = true;
        // 光标落到原 tick 对应空格
        let new_beat = (cursor_tick_hint / slot_ticks) as usize;
        if let Some(song) = state.song.as_ref() {
            if let Some(m) = song.tracks.get(track_idx).and_then(|t| t.measures.get(cursor_measure))
            {
                let len = m.primary_voice().beats.len();
                state.cursor.beat = new_beat.min(len.saturating_sub(1));
            }
        }
        clamp_cursor_from_state(state);
    }
}

fn clamp_cursor_from_state(state: &mut AppState) {
    if let Some(song) = state.song.as_ref() {
        let mut c = state.cursor;
        clamp_cursor(song, &mut c);
        state.cursor = c;
    }
}

/// 在光标格写入休止符（清空音符）
pub fn insert_rest_at_cursor(state: &mut AppState) {
    ensure_beat_exists(state);
    let slot_dur = state.edit_tool.slot_duration();
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let Some(beat) = song.tracks[track_idx]
        .measures
        .get_mut(state.cursor.measure)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(state.cursor.beat))
    else {
        return;
    };
    beat.notes.clear();
    beat.is_rest = true;
    beat.duration = slot_dur;
    state.needs_relayout = true;
    state.status_message = "已写入休止符".into();
    refresh_duration_status(state);
    state.fret_input.clear();
}

/// 按工具写入：休止工具 → 休止；音符工具 → 插入空弦音
pub fn apply_tool_at_cursor(state: &mut AppState) {
    use crate::state::EditToolKind;
    match state.edit_tool.kind {
        EditToolKind::Rest => insert_rest_at_cursor(state),
        EditToolKind::Note => {
            // 保证时值与工具一致后再插音
            ensure_beat_exists(state);
            let slot_dur = state.edit_tool.slot_duration();
            if let Some(song) = state.song.as_mut() {
                if let Some(beat) = song.tracks.get_mut(state.cursor.track)
                    .and_then(|t| t.measures.get_mut(state.cursor.measure))
                    .and_then(|m| m.primary_voice_mut().beats.get_mut(state.cursor.beat))
                {
                    beat.duration = slot_dur;
                }
            }
            insert_note(state);
        }
        EditToolKind::Marker => {
            state.marker_editor_open = true;
        }
    }
}

fn toggle_note_effect(state: &mut AppState, effect: NoteEffect, label: &str) {
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    let string = state.cursor.string;
    let Some(note) = song.tracks[track_idx]
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
        .and_then(|b| b.note_on_string_mut(string))
    else {
        state.status_message = "光标处无音符".into();
        return;
    };
    let same = |e: &NoteEffect| match (&effect, e) {
        (NoteEffect::HammerOnPullOff(a), NoteEffect::HammerOnPullOff(b)) => a == b,
        (NoteEffect::Slide(a), NoteEffect::Slide(b)) => a == b,
        _ => false,
    };
    if note.effects.iter().any(same) {
        note.effects.retain(|e| !same(e));
    } else {
        match &effect {
            NoteEffect::HammerOnPullOff(_) => {
                note.effects
                    .retain(|e| !matches!(e, NoteEffect::HammerOnPullOff(_)));
            }
            NoteEffect::Slide(_) => {
                note.effects.retain(|e| !matches!(e, NoteEffect::Slide(_)));
            }
            _ => {}
        }
        note.effects.push(effect);
    }
    state.needs_relayout = true;
    state.status_message = format!("切换{}", label);
}

pub fn toggle_hammer_on(state: &mut AppState) {
    toggle_note_effect(
        state,
        NoteEffect::HammerOnPullOff(HammerOnPullOff::HammerOn),
        "击弦 H",
    );
}

pub fn toggle_pull_off(state: &mut AppState) {
    toggle_note_effect(
        state,
        NoteEffect::HammerOnPullOff(HammerOnPullOff::PullOff),
        "勾弦 P",
    );
}

pub fn toggle_slide_up(state: &mut AppState) {
    toggle_note_effect(
        state,
        NoteEffect::Slide(vec![SlideType::OutUpwards]),
        "上滑音",
    );
}

pub fn toggle_slide_down(state: &mut AppState) {
    toggle_note_effect(
        state,
        NoteEffect::Slide(vec![SlideType::OutDownwards]),
        "下滑音",
    );
}

/// 切换延音 Tie
pub fn toggle_tie(state: &mut AppState) {
    sync_track(state);
    let Some(song) = state.song.as_mut() else {
        return;
    };
    if song.tracks.is_empty() {
        return;
    }
    let track_idx = state.cursor.track;
    let measure_idx = state.cursor.measure;
    let beat_idx = state.cursor.beat;
    let string = state.cursor.string;
    let Some(note) = song.tracks[track_idx]
        .measures
        .get_mut(measure_idx)
        .and_then(|m| m.primary_voice_mut().beats.get_mut(beat_idx))
        .and_then(|b| b.note_on_string_mut(string))
    else {
        state.status_message = "光标处无音符".into();
        return;
    };
    if note.note_type == NoteType::Tie {
        note.note_type = NoteType::Normal;
    } else {
        note.note_type = NoteType::Tie;
    }
    state.needs_relayout = true;
    state.status_message = "切换延音".into();
}

/// 刷新当前光标小节的时值状态到 status（越界时提示）
pub fn refresh_duration_status(state: &mut AppState) {
    let Some(status) = current_measure_duration_status(state) else {
        return;
    };
    match status {
        MeasureDurationStatus::Ok => {}
        MeasureDurationStatus::Under { expected, actual } => {
            state.status_message = format!(
                "第 {} 小节时值不足（{} / {} ticks）",
                state.cursor.measure + 1,
                actual,
                expected
            );
        }
        MeasureDurationStatus::Over { expected, actual } => {
            state.status_message = format!(
                "第 {} 小节时值超出（{} / {} ticks）",
                state.cursor.measure + 1,
                actual,
                expected
            );
        }
    }
}

pub fn current_measure_duration_status(state: &AppState) -> Option<MeasureDurationStatus> {
    let song = state.song.as_ref()?;
    let track = song.tracks.get(state.cursor.track)?;
    let measure = track.measures.get(state.cursor.measure)?;
    let master = song.master_bar(state.cursor.measure)?;
    Some(check_voice_duration(
        measure.primary_voice(),
        master.time_signature.measure_ticks(),
    ))
}

/// 查询指定轨道小节是否时值越界
pub fn measure_has_duration_error(song: &Song, track_idx: usize, measure_idx: usize) -> bool {
    let Some(track) = song.tracks.get(track_idx) else {
        return false;
    };
    let Some(measure) = track.measures.get(measure_idx) else {
        return false;
    };
    let Some(master) = song.master_bar(measure_idx) else {
        return false;
    };
    !check_voice_duration(
        measure.primary_voice(),
        master.time_signature.measure_ticks(),
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bassoxide_core::measure::{MasterBar, Measure};
    use bassoxide_core::song::Song;
    use bassoxide_core::track::{Track, Tuning};

    fn song_one_measure() -> Song {
        let mut song = Song::default();
        song.master_bars.push(MasterBar::default());
        let mut track = Track::default();
        track.tuning = Tuning::standard_guitar();
        let mut measure = Measure::default();
        measure.primary_voice_mut().beats.push(Beat {
            duration: Duration {
                value: NoteValue::Quarter,
                ..Duration::default()
            },
            is_rest: true,
            ..Beat::default()
        });
        track.measures.push(measure);
        song.tracks.push(track);
        song
    }

    #[test]
    fn move_cursor_across_beats() {
        let mut state = AppState::default();
        let mut song = song_one_measure();
        song.tracks[0]
            .measures[0]
            .primary_voice_mut()
            .beats
            .push(Beat {
                duration: Duration::default(),
                is_rest: true,
                ..Beat::default()
            });
        state.load_song(song, None);
        assert_eq!(state.cursor.beat, 0);
        move_cursor(&mut state, CursorMove::Right);
        assert_eq!(state.cursor.beat, 1);
        move_cursor(&mut state, CursorMove::Left);
        assert_eq!(state.cursor.beat, 0);
    }

    #[test]
    fn insert_and_duration_over() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        state.cursor.string = 1;
        insert_note(&mut state);
        let beat = &state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0];
        assert!(!beat.is_empty());
        assert_eq!(beat.notes[0].string, 1);

        for _ in 0..4 {
            let voice = state.song.as_mut().unwrap().tracks[0].measures[0].primary_voice_mut();
            voice.beats.push(Beat {
                duration: Duration::default(),
                notes: vec![Note {
                    string: 1,
                    fret: 0,
                    ..Note::default()
                }],
                ..Beat::default()
            });
        }
        assert!(measure_has_duration_error(
            state.song.as_ref().unwrap(),
            0,
            0
        ));
    }

    #[test]
    fn change_string_preserves_pitch() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        state.cursor.string = 1;
        set_fret(&mut state, 5);
        let midi_before = state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0]
            .notes[0]
            .midi_note;
        change_note_string(&mut state, 1);
        assert_eq!(state.cursor.string, 2);
        let note = &state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0]
            .notes[0];
        assert_eq!(note.string, 2);
        assert_eq!(note.midi_note, midi_before);
    }

    #[test]
    fn change_string_can_produce_negative_fret() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        // 弦 6 空弦（低 E）移到弦 1：品格应为负数并显示
        state.cursor.string = 6;
        set_fret(&mut state, 0);
        change_note_string(&mut state, -5); // 向高音弦移动
        assert_eq!(state.cursor.string, 1);
        let note = &state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0]
            .notes[0];
        assert!(note.fret < 0, "fret={}", note.fret);
        assert_eq!(
            bassoxide_layout::tablature::fret_display(note.fret),
            note.fret.to_string()
        );
    }

    #[test]
    fn nudge_fret_below_zero() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        state.cursor.string = 1;
        set_fret(&mut state, 0);
        nudge_fret(&mut state, -1);
        let fret = state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0]
            .notes[0]
            .fret;
        assert_eq!(fret, -1);
        assert_eq!(bassoxide_layout::tablature::fret_display(fret), "-1");
    }

    #[test]
    fn eighth_tool_grids_measure_into_eight_slots() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        // 先放一个四分音符在 beat0
        state.cursor.string = 6;
        set_fret(&mut state, 3);
        select_edit_tool(&mut state, crate::state::EditToolKind::Note, Some(NoteValue::Eighth));
        let beats = &state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats;
        assert_eq!(beats.len(), 8);
        assert!(beats.iter().all(|b| b.duration.ticks() == 480));
        // 原音符应落在第 0 格
        assert!(!beats[0].is_empty());
        assert_eq!(beats[0].notes[0].fret, 3);
        assert!(beats[1].is_rest || beats[1].notes.is_empty());
        assert_eq!(slots_per_measure(&state), Some(8));

        move_cursor(&mut state, CursorMove::Right);
        assert_eq!(state.cursor.beat, 1);
        move_cursor(&mut state, CursorMove::Right);
        assert_eq!(state.cursor.beat, 2);
    }

    #[test]
    fn rest_tool_clears_cursor_slot() {
        let mut state = AppState::default();
        state.load_song(song_one_measure(), None);
        state.cursor.string = 1;
        set_fret(&mut state, 5);
        select_edit_tool(&mut state, crate::state::EditToolKind::Rest, Some(NoteValue::Quarter));
        insert_rest_at_cursor(&mut state);
        let beat = &state.song.as_ref().unwrap().tracks[0].measures[0]
            .primary_voice()
            .beats[0];
        assert!(beat.is_rest);
        assert!(beat.notes.is_empty());
    }
}
