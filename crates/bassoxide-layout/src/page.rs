//! 页面布局与纸张规格。

use serde::{Deserialize, Serialize};

/// 单个页面在画布上的矩形区域（绝对坐标）。
#[derive(Debug, Clone)]
pub struct PageLayout {
    /// 页码 (0-based)
    pub index: usize,
    /// 左上角 X
    pub x: f32,
    /// 左上角 Y
    pub y: f32,
    /// 页面宽度
    pub width: f32,
    /// 页面高度
    pub height: f32,
}

/// 常用纸张规格（纵向，约 96dpi 像素）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PaperSize {
    /// 148×210 mm
    A5,
    #[default]
    /// 210×297 mm
    A4,
    /// 297×420 mm
    A3,
    /// 8.5×11 in
    Letter,
    /// 8.5×14 in
    Legal,
}

impl PaperSize {
    pub const ALL: [PaperSize; 5] = [
        PaperSize::A5,
        PaperSize::A4,
        PaperSize::A3,
        PaperSize::Letter,
        PaperSize::Legal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A5 => "A5",
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
            PaperSize::Letter => "Letter",
            PaperSize::Legal => "Legal",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PaperSize::A5 => "148×210 mm",
            PaperSize::A4 => "210×297 mm",
            PaperSize::A3 => "297×420 mm",
            PaperSize::Letter => "8.5×11 in",
            PaperSize::Legal => "8.5×14 in",
        }
    }

    /// 纸张像素尺寸（96dpi）
    pub fn size_px(self) -> (f32, f32) {
        match self {
            PaperSize::A5 => (559.0, 794.0),
            PaperSize::A4 => (794.0, 1123.0),
            PaperSize::A3 => (1123.0, 1587.0),
            PaperSize::Letter => (816.0, 1056.0),
            PaperSize::Legal => (816.0, 1344.0),
        }
    }

    /// 相对 A4 宽度的内容缩放系数（驱动音符/符杆/小节间距）
    pub fn content_scale(self) -> f32 {
        let (w, _) = self.size_px();
        let (a4_w, _) = PaperSize::A4.size_px();
        (w / a4_w).clamp(0.55, 1.6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_reference_scale() {
        assert!((PaperSize::A4.content_scale() - 1.0).abs() < f32::EPSILON);
        assert!(PaperSize::A5.content_scale() < 1.0);
        assert!(PaperSize::A3.content_scale() > 1.0);
    }
}
