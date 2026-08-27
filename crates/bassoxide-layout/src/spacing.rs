//! 间距常量与计算工具。

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
    /// A4 页面宽度 (px)
    pub page_width: f32,
    /// A4 页面高度 (px)
    pub page_height: f32,
    /// 页面内边距 (px)
    pub page_margin: f32,
    /// 六线谱下方符杆(节奏)区域高度 (px)
    pub rhythm_height: f32,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            tab_string_spacing: 14.0,
            staff_line_spacing: 10.0,
            min_measure_width: 120.0,
            min_beat_spacing: 25.0,
            track_gap: 50.0,
            system_gap: 80.0,
            margin_left: 40.0,
            margin_top: 60.0,
            available_width: 1200.0,
            tab_font_size: 12.0,
            clef_width: 30.0,
            time_sig_width: 25.0,
            // A4 纵向 210×297mm，约 96dpi 下 794×1123 px
            page_width: 794.0,
            page_height: 1123.0,
            page_margin: 48.0,
            rhythm_height: 26.0,
        }
    }
}

impl LayoutSettings {
    /// 六线谱总高度 (6弦)
    pub fn tab_staff_height(&self, string_count: usize) -> f32 {
        self.tab_string_spacing * (string_count.saturating_sub(1)) as f32
    }

    /// A4 页面可用内容宽度
    pub fn page_content_width(&self) -> f32 {
        (self.page_width - self.page_margin * 2.0).max(50.0)
    }

    /// A4 页面可用内容高度
    pub fn page_content_height(&self) -> f32 {
        (self.page_height - self.page_margin * 2.0).max(50.0)
    }
}
