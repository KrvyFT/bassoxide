//! Bassoxide 核心数据模型。
//!
//! 本 crate 定义了乐谱文件的完整层次结构：
//! `Song` → `Track` → `Measure` → `Voice` → `Beat` → `Note`
//!
//! 这是一个纯数据层，不依赖任何 GUI 或 I/O 库。

pub mod automation;
pub mod beat;
pub mod chord;
pub mod effects;
pub mod lyrics;
pub mod measure;
pub mod midi;
pub mod note;
pub mod song;
pub mod track;
pub mod types;

// 重新导出常用类型
pub use song::Song;
pub use track::{midi_note_name, GuitarString, StaffDisplay, Track, Tuning};
pub use measure::{
    check_voice_duration, Direction, Marker, MasterBar, Measure, MeasureDurationStatus,
};
pub use beat::{Beat, Voice};
pub use note::Note;
