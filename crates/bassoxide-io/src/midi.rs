//! 标准 MIDI (.mid) 解析器

use bassoxide_core::song::Song;
use midly::Smf;

use crate::error::{IoError, Result};

/// 解析 MIDI 文件
pub fn parse_midi(data: &[u8]) -> Result<Song> {
    let smf = Smf::parse(data)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid MIDI file: {e}")))?;

    // TODO: 实现 MIDI 轨道到 Song 的映射、量化和指法推导
    // 目前返回一个空 Song 骨架
    
    let mut song = Song::default();
    song.info.title = "Imported MIDI".to_string();
    
    // 粗略示例：将所有 MIDI track 读出
    for (i, track) in smf.tracks.iter().enumerate() {
        tracing::debug!("Parsing MIDI track {}", i);
        for _event in track {
            // tracing::debug!("MIDI Event: {:?}", event);
        }
    }

    Ok(song)
}
