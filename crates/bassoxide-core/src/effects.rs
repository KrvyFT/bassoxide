//! 演奏技巧效果定义。
//!
//! 每个枚举变体携带该效果的完整参数数据，
//! 例如 `Bend` 包含弯音曲线的控制点序列。

use serde::{Deserialize, Serialize};

// ── 推弦 (Bend) ──

/// 推弦类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BendType {
    Bend,
    BendRelease,
    BendReleaseBend,
    Prebend,
    PrebendRelease,
    Dip,
}

/// 推弦曲线上的一个控制点
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BendPoint {
    /// 位置 (0–12)，0 = 音符起始, 12 = 音符结束
    pub position: u8,
    /// 偏移量，单位为 25 cents (1 = 25cents, 4 = 半音, 8 = 全音)
    pub value: i8,
    /// 是否有颤音
    pub vibrato: bool,
}

/// 推弦效果数据
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendEffect {
    pub bend_type: BendType,
    pub points: Vec<BendPoint>,
}

// ── 滑音 ──

/// 滑音类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideType {
    /// 向上滑入（从低品位到目标）
    IntoFromBelow,
    /// 向下滑入（从高品位到目标）
    IntoFromAbove,
    /// 向下滑出
    OutDownwards,
    /// 向上滑出
    OutUpwards,
    /// 连奏滑音 (legato slide)
    ShiftSlide,
    /// 换把滑音
    LegatoSlide,
}

// ── 泛音 ──

/// 泛音类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmonicType {
    /// 自然泛音
    Natural,
    /// 人工泛音
    Artificial,
    /// 拍弦泛音 (tap harmonic)
    Tap,
    /// 点弦泛音 (pinch harmonic)
    Pinch,
    /// 半泛音
    Semi,
}

/// 泛音效果数据
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HarmonicEffect {
    pub harmonic_type: HarmonicType,
    /// 人工/Tap 泛音的品格偏移
    pub fret_offset: Option<u8>,
}

// ── 颤音 / 揉弦 ──

/// 颤音速度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VibratoSpeed {
    Slow,
    #[default]
    Medium,
    Fast,
}

/// 颤音类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VibratoType {
    /// 手指揉弦 (左手)
    Finger,
    /// 摇把颤音 (whammy bar)
    WhamBar,
}

// ── 颤音/震音 (Trill / Tremolo) ──

/// 颤音 (快速交替两个音)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrillEffect {
    /// 交替到的品格
    pub fret: u8,
    /// 颤音速度 (对应的音符时值)
    pub duration: TrillSpeed,
}

/// 颤音速度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrillSpeed {
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
}

/// 震音拨弦 (Tremolo Picking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TremoloPickingSpeed {
    Eighth,
    Sixteenth,
    ThirtySecond,
}

// ── 击勾弦 ──

/// 击弦/勾弦标记 (Hammer-On / Pull-Off)
/// 通常标记在音符上，表示与下一个音符的连接方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HammerOnPullOff {
    HammerOn,
    PullOff,
}

// ── 装饰音 ──

/// 装饰音
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraceNote {
    pub fret: u8,
    pub velocity: u8,
    pub duration: GraceNoteDuration,
    pub is_on_beat: bool,
    /// 是否为死音
    pub is_dead: bool,
    pub transition: GraceNoteTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraceNoteDuration {
    Sixteenth,
    TwentyFourth,
    ThirtySecond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraceNoteTransition {
    None,
    Slide,
    Bend,
    HammerOn,
}

// ── 聚合：音符级效果 ──

/// 单个音符上可附加的演奏效果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoteEffect {
    Bend(BendEffect),
    Slide(Vec<SlideType>),
    Harmonic(HarmonicEffect),
    Vibrato(VibratoType, VibratoSpeed),
    HammerOnPullOff(HammerOnPullOff),
    /// 延音 (Let Ring)
    LetRing,
    /// 闷音 (Palm Mute)
    PalmMute,
    /// 左手闷音
    LeftHandMute,
    /// 鬼音 (Ghost Note)
    GhostNote,
    /// 重音 (Accent)
    Accent,
    /// 强重音 (Heavy Accent / Marcato)
    HeavyAccent,
    /// 断奏 (Staccato)
    Staccato,
    /// 连奏 (Legato)
    Legato,
    /// 点弦 (Tapping)
    Tapping,
    /// 拍弦 (Slap)
    Slap,
    /// 勾弦弹奏 (Pop)
    Pop,
    /// 颤音
    Trill(TrillEffect),
    /// 震音拨弦
    TremoloPicking(TremoloPickingSpeed),
    /// 装饰音
    GraceNote(GraceNote),
    /// 指法标记 (左手)
    LeftFingering(Fingering),
    /// 指法标记 (右手)
    RightFingering(Fingering),
}

/// 手指编号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fingering {
    Thumb,
    Index,
    Middle,
    Ring,
    Pinky,
}

// ── 聚合：Beat 级效果 ──

/// 拍级效果（影响整拍的所有音符）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BeatEffect {
    /// 扫弦方向
    Stroke(StrokeDirection, StrokeSpeed),
    /// 摇把效果 (Whammy Bar)
    WhammyBar(BendEffect),
    /// 拍击 (Slap/Pop 组合)
    SlapPop,
    /// 渐入 (Fade In)
    FadeIn,
    /// 连音标记 (Tie) — 与前一拍连接
    Tie,
    /// 琶音（分解和弦方向）
    Arpeggio(ArpeggioDirection),
}

/// 扫弦方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeDirection {
    Down,
    Up,
}

/// 扫弦速度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeSpeed {
    /// 极快 (几乎同时)
    Fastest,
    Fast,
    Medium,
    Slow,
}

/// 琶音方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpeggioDirection {
    Up,
    Down,
}
