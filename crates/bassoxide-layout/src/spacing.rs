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
        }
    }
}

impl LayoutSettings {
    /// 六线谱总高度 (6弦)
    pub fn tab_staff_height(&self, string_count: usize) -> f32 {
        self.tab_string_spacing * (string_count.saturating_sub(1)) as f32
    }
}
