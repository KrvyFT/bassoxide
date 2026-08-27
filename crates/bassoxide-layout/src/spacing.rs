//! 间距常量与计算工具。

use crate::page::PaperSize;

/// 布局参数（可调节的全局排版常量）
#[derive(Debug, Clone)]
pub struct LayoutSettings {
    /// 六线谱弦间距 (px)
    pub tab_string_spacing: f32,
    /// 五线谱线间距 (px)
    pub staff_line_spacing: f32,
    /// 小节最小宽度 (px)
    pub min_measure_width: f32,
    /// 小节内音符最小间距 (px)
    pub min_beat_spacing: f32,
    /// 轨道之间的垂直间距 (px)
    pub track_gap: f32,
    /// System（行）之间的垂直间距 (px)
    pub system_gap: f32,
    /// 左边距 (px)
    pub margin_left: f32,
    /// 上边距 (px)
    pub margin_top: f32,
    /// 可用宽度 (px)
    pub available_width: f32,
    /// 音符数字字体大小 (px)
    pub tab_font_size: f32,
    /// 谱号区域宽度 (px)
    pub clef_width: f32,
    /// 拍号区域宽度 (px)
    pub time_sig_width: f32,
    /// 页面宽度 (px)
    pub page_width: f32,
    /// 页面高度 (px)
    pub page_height: f32,
    /// 页面内边距 (px)
    pub page_margin: f32,
    /// 六线谱下方符杆(节奏)区域高度 (px)
    pub rhythm_height: f32,
    /// 每行小节数：0 = 按页宽自动换行，否则强制每行 N 小节
    pub measures_per_line: u8,
    /// 当前纸张（用于相对 A4 的内容缩放）
    pub paper_size: PaperSize,
    /// 相对 A4 的内容缩放（音符/符杆/最小小节间距）
    pub content_scale: f32,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        let paper = PaperSize::A4;
        let (page_width, page_height) = paper.size_px();
        Self {
            tab_string_spacing: 10.0,
            staff_line_spacing: 10.0,
            min_measure_width: 80.0,
            min_beat_spacing: 22.0,
            track_gap: 50.0,
            system_gap: 80.0,
            margin_left: 40.0,
            margin_top: 60.0,
            available_width: 1200.0,
            tab_font_size: 13.0,
            clef_width: 30.0,
            time_sig_width: 28.0,
            page_width,
            page_height,
            page_margin: 48.0,
            rhythm_height: 26.0,
            measures_per_line: 4,
            paper_size: paper,
            content_scale: 1.0,
        }
    }
}

impl LayoutSettings {
    /// 六线谱弦线区域高度（不含音符内边距）
    pub fn tab_staff_height(&self, string_count: usize) -> f32 {
        self.tab_string_spacing * (string_count.saturating_sub(1)) as f32
    }

    /// 音符在谱表上下各侧预留（保证字号不画出谱表带）
    pub fn note_pad(&self) -> f32 {
        (self.tab_font_size * 0.55).clamp(4.0, 18.0)
    }

    /// 五线谱加线预留
    pub fn ledger_pad(&self) -> f32 {
        (self.staff_line_spacing * 2.2).clamp(8.0, 36.0)
    }

    /// Tab 谱表总绘制高度 = 弦线区 + 上下音符垫
    pub fn tab_band_height(&self, string_count: usize) -> f32 {
        self.tab_staff_height(string_count) + self.note_pad() * 2.0
    }

    /// 五线谱总高度（5 线 = 4 个间距）
    pub fn standard_staff_height(&self) -> f32 {
        self.staff_line_spacing * 4.0
    }

    /// 五线谱绘制带高度（含加线垫）
    pub fn standard_band_height(&self) -> f32 {
        self.standard_staff_height() + self.ledger_pad() * 2.0
    }

    /// 页面可用内容宽度
    pub fn page_content_width(&self) -> f32 {
        (self.page_width - self.page_margin * 2.0).max(50.0)
    }

    /// 页面可用内容高度
    pub fn page_content_height(&self) -> f32 {
        (self.page_height - self.page_margin * 2.0).max(50.0)
    }

    /// 参考节拍间距（用于符杆按小节密度缩放）
    pub fn reference_beat_gap(&self) -> f32 {
        self.min_beat_spacing.max(8.0)
    }
}
