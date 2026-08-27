//! 布局硬约束：音符 ∈ 谱表 > 谱表 ∈ 纸张。
//!
//! 用户调节设置时，若不满足上述优先级，自动放宽次要参数。

use crate::spacing::LayoutSettings;

/// 当前显示轨道的谱表形态（用于估算行高）
#[derive(Debug, Clone, Copy)]
pub struct StaffFitContext {
    pub show_standard: bool,
    pub show_tab: bool,
    pub tab_strings: u8,
}

impl Default for StaffFitContext {
    fn default() -> Self {
        Self {
            show_standard: false,
            show_tab: true,
            tab_strings: 6,
        }
    }
}

/// 约束求解结果
#[derive(Debug, Clone, Default)]
pub struct FitResult {
    /// 是否改动了 settings
    pub adjusted: bool,
    /// 人类可读的调节说明
    pub messages: Vec<String>,
}

impl FitResult {
    pub fn summary(&self) -> Option<String> {
        if self.messages.is_empty() {
            None
        } else {
            Some(self.messages.join("；"))
        }
    }
}

/// 估算单行 System 占用高度（含符杆与音符内边距）
pub fn estimate_system_height(settings: &LayoutSettings, ctx: StaffFitContext) -> f32 {
    let note_pad = settings.note_pad();
    let ledger_pad = settings.ledger_pad();
    let mut h = 0.0;
    let mut any = false;

    if ctx.show_standard {
        h += settings.standard_staff_height() + ledger_pad * 2.0;
        any = true;
    }
    if ctx.show_tab {
        if any {
            h += settings.track_gap;
        }
        let strings = ctx.tab_strings.max(1) as usize;
        h += settings.tab_staff_height(strings) + note_pad * 2.0 + settings.rhythm_height + 8.0;
        any = true;
    }
    if !any {
        h += settings.standard_staff_height() + ledger_pad * 2.0;
    }
    h.max(24.0)
}

/// 纸张内容区可容纳的最大单行高度（至少保证 1 行）
pub fn max_system_height_on_page(settings: &LayoutSettings) -> f32 {
    let top_pad = settings.margin_top.min(settings.page_margin);
    (settings.page_content_height() - top_pad).max(40.0)
}

/// 按优先级求解：
/// 1. 音符必须落在谱表带内（字号 ≤ 弦距，并预留 note_pad）
/// 2. 谱表行必须落在纸张内容区内（必要时压缩行距/符杆/字号/弦距）
pub fn resolve_fit(settings: &mut LayoutSettings, ctx: StaffFitContext) -> FitResult {
    let mut result = FitResult::default();

    // —— 优先级 1：音符 ∈ 谱表 ——
    // 弦距必须能容纳字号，否则抬高弦距（或略降字号）
    let min_spacing_for_font = (settings.tab_font_size * 0.92).max(5.0);
    if settings.tab_string_spacing < min_spacing_for_font {
        let old = settings.tab_string_spacing;
        settings.tab_string_spacing = min_spacing_for_font;
        settings.staff_line_spacing = settings.staff_line_spacing.max(min_spacing_for_font * 0.85);
        result.adjusted = true;
        result.messages.push(format!(
            "线间距 {old:.0}→{:.0} 以保证音符在谱表内",
            settings.tab_string_spacing
        ));
    }

    // 五线距同样不低于字号相关下限
    let min_staff = (settings.tab_font_size * 0.7).max(5.0);
    if settings.staff_line_spacing < min_staff {
        settings.staff_line_spacing = min_staff;
        result.adjusted = true;
        result.messages.push("五线距已提高以容纳符头".into());
    }

    // 符杆区至少能画最短符干
    let min_rhythm = (settings.tab_font_size * 0.9).max(8.0);
    if settings.rhythm_height < min_rhythm {
        settings.rhythm_height = min_rhythm;
        result.adjusted = true;
        result.messages.push("符杆区已扩大以容纳符干".into());
    }

    // —— 优先级 2：谱表 ∈ 纸张 ——
    let mut guard = 0;
    while estimate_system_height(settings, ctx) > max_system_height_on_page(settings) && guard < 64 {
        guard += 1;
        let need = estimate_system_height(settings, ctx);
        let limit = max_system_height_on_page(settings);
        let overflow = need - limit;
        result.adjusted = true;

        if settings.system_gap > 0.0 {
            let cut = (overflow * 0.55).min(settings.system_gap).max(1.0);
            settings.system_gap = (settings.system_gap - cut).max(0.0);
            result.messages.push(format!("行间距降至 {:.0} 以适应纸张", settings.system_gap));
            continue;
        }
        if settings.rhythm_height > min_rhythm + 1.0 {
            let cut = (overflow * 0.4)
                .min(settings.rhythm_height - min_rhythm)
                .max(1.0);
            settings.rhythm_height = (settings.rhythm_height - cut).max(min_rhythm);
            result.messages.push(format!("符杆区降至 {:.0} 以适应纸张", settings.rhythm_height));
            continue;
        }
        if settings.track_gap > 12.0 {
            settings.track_gap = (settings.track_gap - 6.0).max(12.0);
            result.messages.push(format!("谱表间距降至 {:.0}", settings.track_gap));
            continue;
        }
        if settings.tab_font_size > 7.0 {
            // 先降字号，才能继续降弦距（优先级 1 下限）
            let next = (settings.tab_font_size - 1.0).max(7.0);
            settings.tab_font_size = next;
            let ms = (next * 0.92).max(5.0);
            if settings.tab_string_spacing > ms {
                settings.tab_string_spacing = ms;
                settings.staff_line_spacing = settings.staff_line_spacing.min(ms).max(ms * 0.85);
            }
            settings.rhythm_height = settings
                .rhythm_height
                .min((next * 1.85).max(min_rhythm))
                .max((next * 0.9).max(8.0));
            result.messages.push(format!("字体降至 {:.0} 以适应纸张", next));
            continue;
        }
        if settings.tab_string_spacing > min_spacing_for_font.min(settings.tab_font_size * 0.92) {
            let floor = (settings.tab_font_size * 0.92).max(5.0);
            let next = (settings.tab_string_spacing - 1.5).max(floor);
            settings.tab_string_spacing = next;
            settings.staff_line_spacing = settings.staff_line_spacing.min(next).max(floor * 0.85);
            result.messages.push(format!("线间距降至 {:.0} 以适应纸张", next));
            continue;
        }
        // 仍放不下：减小页边距（最后手段）
        if settings.page_margin > 16.0 {
            settings.page_margin = (settings.page_margin - 6.0).max(16.0);
            result.messages.push(format!("页边距降至 {:.0}", settings.page_margin));
            continue;
        }
        break;
    }

    // 水平：最小节拍间距不能大到无法塞进每行小节
    if settings.measures_per_line > 0 {
        let preamble = settings.clef_width + settings.time_sig_width;
        let avail = (settings.page_content_width() - preamble).max(40.0);
        let per = avail / f32::from(settings.measures_per_line);
        // 假设每小节最多约 8 个最短音
        let max_gap = ((per - 16.0) / 8.0).max(6.0);
        if settings.min_beat_spacing > max_gap {
            settings.min_beat_spacing = max_gap;
            result.adjusted = true;
            result.messages.push(format!(
                "节拍间距降至 {:.0} 以保证音符在小节内",
                max_gap
            ));
        }
        if settings.min_measure_width > per {
            settings.min_measure_width = per.max(32.0);
            result.adjusted = true;
        }
    }

    // 去重消息（循环可能重复同类）
    result.messages.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_font_raises_spacing_to_keep_notes_in_staff() {
        let mut s = LayoutSettings::default();
        s.tab_font_size = 28.0;
        s.tab_string_spacing = 8.0;
        let r = resolve_fit(&mut s, StaffFitContext::default());
        assert!(r.adjusted);
        assert!(s.tab_string_spacing >= s.tab_font_size * 0.9);
    }

    #[test]
    fn oversized_system_shrinks_to_fit_paper() {
        let mut s = LayoutSettings::default();
        // 极小纸张 + 巨大行距/字号
        s.page_width = 400.0;
        s.page_height = 280.0;
        s.page_margin = 40.0;
        s.tab_font_size = 22.0;
        s.tab_string_spacing = 20.0;
        s.rhythm_height = 60.0;
        s.system_gap = 120.0;
        let ctx = StaffFitContext {
            show_standard: true,
            show_tab: true,
            tab_strings: 6,
        };
        let before = estimate_system_height(&s, ctx);
        let r = resolve_fit(&mut s, ctx);
        let after = estimate_system_height(&s, ctx);
        assert!(r.adjusted);
        assert!(after <= max_system_height_on_page(&s) + 1.0, "before={before} after={after}");
        assert!(after < before);
    }
}
