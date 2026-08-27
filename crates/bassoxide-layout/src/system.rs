//! System（系统行）布局：一行中多个轨道的垂直堆叠。

use crate::staff::StaffLayout;

/// 一个 System 代表乐谱中的一行，包含多个小节横向排列，
/// 多个轨道的谱表纵向堆叠。
#[derive(Debug, Clone)]
pub struct SystemLayout {
    /// 起始小节索引
    pub start_measure: usize,
    /// 结束小节索引（不含）
    pub end_measure: usize,
    /// System 顶部 Y 坐标（绝对坐标）
    pub y: f32,
    /// System 总高度
    pub height: f32,
    /// 谱表/前导区起始 X（绝对坐标，含页面左边距）
    pub content_left: f32,
    /// 谱线绘制宽度（从 content_left 起）
    pub content_width: f32,
    /// 所属页码
    pub page_index: usize,
    /// 各轨道的谱表布局
    pub staves: Vec<StaffLayout>,
    /// 各小节在此 System 中的 X 坐标和宽度（绝对坐标）
    pub measure_positions: Vec<MeasurePosition>,
}

/// 小节在 System 中的水平位置
#[derive(Debug, Clone)]
pub struct MeasurePosition {
    pub measure_index: usize,
    pub x: f32,
    pub width: f32,
}
