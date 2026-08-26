//! 歌词数据模型。

use serde::{Deserialize, Serialize};

/// 歌词信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Lyrics {
    /// 歌词关联的轨道编号 (1-based)
    pub track_number: u8,
    /// 每行歌词及其起始小节号
    pub lines: Vec<LyricsLine>,
}

/// 单行歌词
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricsLine {
    /// 起始小节号 (1-based)
    pub start_measure: u32,
    /// 歌词文本（空格分隔对应各拍）
    pub text: String,
}
