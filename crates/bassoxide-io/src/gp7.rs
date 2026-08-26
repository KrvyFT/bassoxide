//! Guitar Pro 7/8 (.gp) 解析器

use bassoxide_core::song::{Song, SongInfo};
use bassoxide_core::track::Track;
use std::io::Read;
use roxmltree::Document;

use crate::error::{IoError, Result};

/// 解析 GP7/GP8 文件
pub fn parse_gp7(data: &[u8]) -> Result<Song> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid ZIP/GP7 file: {e}")))?;

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

    parse_score_gpif(&score_gpif_content)
}

fn parse_score_gpif(xml: &str) -> Result<Song> {
    let doc = Document::parse(xml)
        .map_err(|e| IoError::ParseError(format!("XML Parse Error: {e}")))?;
    
    let mut song = Song::default();
    
    // GPIF (GP7) 具有更清晰的标签名，如 <Score>, <Title>, <Artist>
    if let Some(score_node) = doc.descendants().find(|n| n.has_tag_name("Score")) {
        song.info.title = score_node.descendants()
            .find(|n| n.has_tag_name("Title"))
            .and_then(|n| n.text())
            .unwrap_or("Unknown Title")
            .to_string();
            
        song.info.artist = score_node.descendants()
            .find(|n| n.has_tag_name("Artist"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();
    }
    
    // 获取轨道列表
    let tracks_node = doc.descendants().find(|n| n.has_tag_name("Tracks"));
    if let Some(tracks) = tracks_node {
        for track_node in tracks.children().filter(|n| n.has_tag_name("Track")) {
            let mut track = Track::default();
            
            track.name = track_node.descendants()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("Track")
                .to_string();
                
            // 添加空的小节占位，与 GPX 相同，完整解析需要复杂的 ID 关联
            track.measures.push(bassoxide_core::measure::Measure::default());
            song.tracks.push(track);
        }
    }
    
    song.master_bars.push(bassoxide_core::measure::MasterBar::default());

    Ok(song)
}
