//! Guitar Pro 7/8 (.gp) 解析器

use bassoxide_core::song::Song;
use std::io::Read;

use crate::error::{IoError, Result};

/// 解析 GP7/GP8 文件
pub fn parse_gp7(data: &[u8]) -> Result<Song> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid ZIP/GP7 file: {e}")))?;

    // GP7+ 的主要数据在 Content/score.gpif 中
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
        return Err(IoError::UnsupportedFormat("Missing score.gpif in GP archive".to_string()));
    }

    // 解析 score.gpif
    parse_score_gpif(&score_gpif_content)
}

fn parse_score_gpif(_xml: &str) -> Result<Song> {
    // TODO: 实现 GPIF 解析并映射到 Song 模型
    Ok(Song::default())
}
