//! 颜色主题定义。

use egui::Color32;

/// 乐谱渲染颜色主题
pub struct Theme {
    /// 谱表线颜色
    pub staff_line: Color32,
    /// 小节线颜色
    pub bar_line: Color32,
    /// 音符数字颜色
    pub note_text: Color32,
    /// 休止符颜色
    pub rest_color: Color32,
    /// 选中音符颜色
    pub selected_note: Color32,
    /// 播放光标颜色
    pub cursor_color: Color32,
    /// 排练标记颜色
    pub marker_color: Color32,
    /// 背景色
    pub background: Color32,
    /// 谱号文字颜色
    pub clef_color: Color32,
    /// 拍号文字颜色
    pub time_sig_color: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// 深色主题
    pub fn dark() -> Self {
        Self {
            staff_line: Color32::from_gray(90),
            bar_line: Color32::from_gray(120),
            note_text: Color32::from_rgb(230, 230, 230),
            rest_color: Color32::from_gray(100),
            selected_note: Color32::from_rgb(100, 180, 255),
            cursor_color: Color32::from_rgba_premultiplied(80, 160, 255, 120),
            marker_color: Color32::from_rgb(255, 200, 60),
            background: Color32::from_rgb(30, 30, 35),
            clef_color: Color32::from_gray(160),
            time_sig_color: Color32::from_gray(200),
        }
    }

    /// 浅色主题
    pub fn light() -> Self {
        Self {
            staff_line: Color32::from_gray(180),
            bar_line: Color32::from_gray(80),
            note_text: Color32::from_gray(20),
            rest_color: Color32::from_gray(120),
            selected_note: Color32::from_rgb(30, 100, 200),
            cursor_color: Color32::from_rgba_premultiplied(30, 100, 200, 80),
            marker_color: Color32::from_rgb(200, 120, 0),
            background: Color32::from_rgb(250, 248, 245),
            clef_color: Color32::from_gray(80),
            time_sig_color: Color32::from_gray(40),
        }
    }
}
