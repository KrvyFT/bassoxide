//! Staff（谱表）布局数据结构。

/// 单个谱表的布局信息
#[derive(Debug, Clone)]
pub struct StaffLayout {
    /// 谱表类型
    pub staff_type: StaffType,
    /// 弦数（仅 Tab 有意义）
    pub string_count: usize,
    /// 谱表顶部 Y 坐标（相对于 System）
    pub y: f32,
    /// 谱表高度
    pub height: f32,
}

/// 谱表类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffType {
    /// 五线谱
    Standard,
    /// 六线谱 (Tab)
    Tablature,
    /// 简谱 (Numbered)
    Numbered,
    /// 斜线记谱
    Slash,
}
