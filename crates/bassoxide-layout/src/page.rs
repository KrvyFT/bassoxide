//! A4 页面布局。

/// 单个 A4 页面在画布上的矩形区域（绝对坐标）。
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
