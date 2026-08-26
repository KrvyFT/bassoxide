//! 和弦图数据模型。

use serde::{Deserialize, Serialize};

/// 和弦图 — 用于在六线谱上方显示和弦指法
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChordDiagram {
    /// 和弦名称 (如 "Am7", "Cmaj9")
    pub name: String,
    /// 起始品格 (0 = 从 nut 开始)
    pub first_fret: u8,
    /// 每根弦上的品格值 (-1 = 不按/不弹, 0 = 空弦, N = 品格)
    pub frets: Vec<i8>,
    /// 每根弦上的手指 (0 = 无, 1-4 = 食中无小)
    pub fingers: Vec<i8>,
    /// 是否显示横按 (barré)
    pub barre: Vec<Barre>,
}

/// 横按信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Barre {
    /// 品格
    pub fret: u8,
    /// 起始弦 (1-based)
    pub start_string: u8,
    /// 结束弦 (1-based)
    pub end_string: u8,
}

impl Default for ChordDiagram {
    fn default() -> Self {
        Self {
            name: String::new(),
            first_fret: 0,
            frets: vec![-1; 6],
            fingers: vec![0; 6],
            barre: Vec::new(),
        }
    }
}
