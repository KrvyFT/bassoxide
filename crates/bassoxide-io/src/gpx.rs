//! Guitar Pro 6 (.gpx) 解析器

use bassoxide_core::song::Song;
use bassoxide_core::track::Track;
use std::io::Read;
use roxmltree::Document;

use crate::error::{IoError, Result};

/// 解析 GP6 文件
pub fn parse_gpx(data: &[u8]) -> Result<Song> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| IoError::UnsupportedFormat(format!("Not a valid ZIP/GPX file: {e}")))?;

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

    parse_score_xml(&score_xml_content)
}

fn parse_score_xml(xml: &str) -> Result<Song> {
    let doc = Document::parse(xml)
        .map_err(|e| IoError::ParseError(format!("XML Parse Error: {e}")))?;
    
    let mut song = Song::default();
    
    // 解析乐谱信息
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
    
    // 解析轨道基础信息
    let tracks_node = doc.descendants().find(|n| n.has_tag_name("Tracks"));
    if let Some(tracks) = tracks_node {
        for track_node in tracks.children().filter(|n| n.has_tag_name("Track")) {
            let mut track = Track::default();
            
            track.name = track_node.descendants()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("Track")
                .to_string();
                
            // 注意：完整的 GPX 解析需要基于 ID 建立哈希表，
            // 将 MasterBars, Bars, Voices, Beats, Notes 连接起来。
            // 这是一个庞大的映射工程，这里我们仅提取元信息和生成一个空小节以保证渲染不崩溃。
            track.measures.push(bassoxide_core::measure::Measure::default());
            song.tracks.push(track);
        }
    }
    
    // 给全局小节塞一个默认值
    song.master_bars.push(bassoxide_core::measure::MasterBar::default());

    Ok(song)
}
