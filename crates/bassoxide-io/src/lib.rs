//! 文件 I/O 库：支持 Guitar Pro 文件格式读写。
//!
//! 目前实现：GP5 (.gp5) 读取
//! 计划支持：GP3, GP4, GPX(GP6), GP7, MIDI, MusicXML

pub mod binary;
pub mod error;
pub mod gp5;
pub mod gpx;
pub mod gp7;
pub mod midi;

use bassoxide_core::song::Song;

use crate::error::{IoError, Result};

/// 自动检测文件格式并解析
pub fn load_file(data: &[u8]) -> Result<Song> {
    if data.len() < 4 {
        return Err(IoError::UnsupportedFormat("文件过小".to_string()));
    }

    let mut song = if &data[0..4] == b"MThd" {
        midi::parse_midi(data)
    } else if &data[0..4] == b"PK\x03\x04" {
        gp7::parse_gp7(data).or_else(|_| gpx::parse_gpx(data))
    } else if let Some(version_str) = detect_version(data) {
        if version_str.contains("v5.") {
            gp5::parse_gp5(data)
        } else {
            Err(IoError::UnsupportedFormat(version_str))
        }
    } else {
        Err(IoError::UnsupportedFormat("无法识别的文件格式".to_string()))
    }?;

    song.apply_file_instruments();
    Ok(song)
}

/// 从文件扩展名加载
pub fn load_from_path(path: &std::path::Path) -> Result<Song> {
    let data = std::fs::read(path)?;
    load_file(&data)
}

/// 检测文件版本字符串
fn detect_version(data: &[u8]) -> Option<String> {
    if data.len() < 31 {
        return None;
    }
    // GP 文件以 byte-size-string 开头：第一个字节是长度
    let len = data[0] as usize;
    if len > 30 || len + 1 > data.len() {
        return None;
    }
    let version = String::from_utf8_lossy(&data[1..1 + len]).to_string();
    if version.starts_with("FICHIER GUITAR PRO") {
        Some(version)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_version_empty() {
        assert!(detect_version(&[]).is_none());
    }

    #[test]
    fn test_detect_version_invalid() {
        assert!(detect_version(&[5, b'H', b'E', b'L', b'L', b'O']).is_none());
    }
}
