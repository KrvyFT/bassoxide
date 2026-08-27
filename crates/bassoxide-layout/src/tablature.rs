//! 六线谱专用布局计算。

use crate::spacing::LayoutSettings;

/// 计算弦号在六线谱中的 Y 偏移（相对于谱表带顶部）。
///
/// 弦线位于 `note_pad` 之下，保证音符字号落在谱表带内。
/// `string_count` 用于夹紧非法弦号，避免画出谱外。
pub fn string_y_offset(
    string_number: u8,
    string_count: usize,
    settings: &LayoutSettings,
) -> f32 {
    let pad = settings.note_pad();
    let max_idx = string_count.saturating_sub(1) as u8;
    let idx = string_number.saturating_sub(1).min(max_idx);
    pad + f32::from(idx) * settings.tab_string_spacing
}

/// 计算品格数字显示文本
pub fn fret_display(fret: i8) -> String {
    if fret < 0 {
        "x".to_string()
    } else {
        fret.to_string()
    }
}

/// 六线谱谱号标记 "TAB" 的各字母 Y 位置（落在弦线区内）
pub fn tab_clef_positions(string_count: usize, settings: &LayoutSettings) -> Vec<f32> {
    let pad = settings.note_pad();
    let total_height = settings.tab_staff_height(string_count);
    let spacing = total_height / 4.0;
    vec![pad + spacing, pad + spacing * 2.0, pad + spacing * 3.0]
}
