//! 乐谱排版引擎。
//!
//! 将 `Song` 数据模型转换为可渲染的布局结果 `LayoutResult`。
//! 计算每个元素在屏幕上的精确坐标。

pub mod engine;
pub mod measure_layout;
pub mod page;
pub mod spacing;
pub mod staff;
pub mod system;
pub mod tablature;

pub use engine::{LayoutEngine, LayoutResult};
