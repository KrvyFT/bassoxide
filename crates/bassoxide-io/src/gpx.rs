//! Guitar Pro 6 (.gpx) 解析器

use bassoxide_core::song::Song;
use std::io::Read;

use crate::error::{IoError, Result};

/// 解析 GP6 文件
pub fn parse_gpx(data: &[u8]) -> Result<Song> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid ZIP/GPX file: {e}")))?;

    // GPX 的主要数据在 score.xml 中
    let mut score_xml_content = String::new();
    let mut found = false;
    
    for i in 0..archive.len() {
        if let Ok(mut file) = archive.by_index(i) {
            if file.name() == "score.xml" {
                file.read_to_string(&mut score_xml_content)?;
                found = true;
                break;
            }
        }
    }

    if !found {
        return Err(IoError::UnsupportedFormat("Missing score.xml in GPX archive".to_string()));
    }

    // 解析 score.xml
    parse_score_xml(&score_xml_content)
}

fn parse_score_xml(_xml: &str) -> Result<Song> {
    // TODO: 实现 XML 解析并映射到 Song 模型
    // 为保持本骨架能够编译和运行，返回一个空的 Song，或者报错说暂未实现
    Ok(Song::default())
}
