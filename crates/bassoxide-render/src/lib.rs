//! 乐谱渲染库。
//!
//! 将 `bassoxide-layout` 的排版结果绘制到 `egui::Painter`。

pub mod colors;
pub mod cursor;
pub mod note_render;
pub mod rhythm_render;
pub mod score_painter;
pub mod selection;
pub mod staff_render;
pub mod effect_render;

pub use colors::Theme;
pub use score_painter::{EditCursor, ScorePainter};
