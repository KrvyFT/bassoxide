//! 六线谱专用布局计算。

use crate::spacing::LayoutSettings;

/// 计算弦号在六线谱中的 Y 偏移（相对于谱表顶部）
pub fn string_y_offset(string_number: u8, settings: &LayoutSettings) -> f32 {
    // string 1 = 最高音弦 = 最上面的线
    (string_number.saturating_sub(1)) as f32 * settings.tab_string_spacing
}

/// 计算品格数字显示文本
pub fn fret_display(fret: i8) -> String {
    if fret < 0 {
        "x".to_string()
    } else {
        fret.to_string()
    }
}

/// 六线谱谱号标记 "TAB" 的各字母 Y 位置
pub fn tab_clef_positions(string_count: usize, settings: &LayoutSettings) -> Vec<f32> {
    let total_height = settings.tab_staff_height(string_count);
    // "T", "A", "B" 均匀分布在谱表高度中
    let spacing = total_height / 4.0;
    vec![spacing, spacing * 2.0, spacing * 3.0]
}
