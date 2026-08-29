//! Material You 风格色板（种子色 #73a187）与 egui Visuals 应用。

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};
use bassoxide_render::Theme;

/// Material You 种子色
pub const SEED: Color32 = Color32::from_rgb(0x73, 0xA1, 0x87);

/// 纸张纯白（谱面与主内容区背景）
pub const PAPER_WHITE: Color32 = Color32::WHITE;

/// 由种子色派生的 MD3 风格色板
#[derive(Debug, Clone, Copy)]
pub struct MaterialPalette {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub secondary: Color32,
    pub on_secondary: Color32,
    pub secondary_container: Color32,
    pub surface: Color32,
    pub surface_container: Color32,
    pub surface_container_high: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub error: Color32,
    pub on_error: Color32,
    pub is_light: bool,
}

impl MaterialPalette {
    pub fn light() -> Self {
        Self {
            primary: Color32::from_rgb(0x3D, 0x6B, 0x52),
            on_primary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            primary_container: Color32::from_rgb(0xBF, 0xE9, 0xCF),
            on_primary_container: Color32::from_rgb(0x1A, 0x37, 0x28),
            secondary: Color32::from_rgb(0x4F, 0x63, 0x57),
            on_secondary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            secondary_container: Color32::from_rgb(0xD1, 0xE8, 0xD9),
            // Material You surface（浅绿灰），衬托纯白纸张
            surface: Color32::from_rgb(0xF0, 0xF5, 0xF1),
            surface_container: Color32::from_rgb(0xE4, 0xED, 0xE6),
            surface_container_high: Color32::from_rgb(0xD8, 0xE5, 0xDB),
            on_surface: Color32::from_rgb(0x1A, 0x1C, 0x1A),
            on_surface_variant: Color32::from_rgb(0x40, 0x4A, 0x43),
            outline: Color32::from_rgb(0x70, 0x7A, 0x72),
            outline_variant: Color32::from_rgb(0xC0, 0xC9, 0xC1),
            error: Color32::from_rgb(0xBA, 0x1A, 0x1A),
            on_error: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            is_light: true,
        }
    }

    pub fn dark() -> Self {
        Self {
            primary: Color32::from_rgb(0xA3, 0xCD, 0xB4),
            on_primary: Color32::from_rgb(0x0F, 0x38, 0x25),
            primary_container: Color32::from_rgb(0x25, 0x52, 0x3B),
            on_primary_container: Color32::from_rgb(0xBF, 0xE9, 0xCF),
            secondary: Color32::from_rgb(0xB5, 0xCC, 0xBD),
            on_secondary: Color32::from_rgb(0x20, 0x37, 0x2C),
            secondary_container: Color32::from_rgb(0x36, 0x4B, 0x42),
            surface: Color32::from_rgb(0x12, 0x15, 0x13),
            surface_container: Color32::from_rgb(0x1E, 0x22, 0x1F),
            surface_container_high: Color32::from_rgb(0x28, 0x2D, 0x2A),
            on_surface: Color32::from_rgb(0xE1, 0xE3, 0xDF),
            on_surface_variant: Color32::from_rgb(0xC0, 0xC9, 0xC1),
            outline: Color32::from_rgb(0x8A, 0x93, 0x8B),
            outline_variant: Color32::from_rgb(0x40, 0x4A, 0x43),
            error: Color32::from_rgb(0xFF, 0xB4, 0xAB),
            on_error: Color32::from_rgb(0x69, 0x00, 0x05),
            is_light: false,
        }
    }

    pub fn for_mode(is_light: bool) -> Self {
        if is_light {
            Self::light()
        } else {
            Self::dark()
        }
    }

    /// 应用到 egui Context
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.visuals = self.to_visuals();
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.menu_margin = Margin::same(6);
        apply_corner_radius(&mut style, 10);
        ctx.set_style(style);
    }

    fn to_visuals(&self) -> Visuals {
        let mut v = if self.is_light {
            Visuals::light()
        } else {
            Visuals::dark()
        };

        v.window_fill = self.surface_container;
        v.panel_fill = self.surface;
        v.extreme_bg_color = self.surface_container_high;
        v.faint_bg_color = self.surface_container;
        v.code_bg_color = self.surface_container_high;
        v.override_text_color = Some(self.on_surface);
        v.hyperlink_color = self.primary;
        v.warn_fg_color = Color32::from_rgb(0xC4, 0x7A, 0x20);
        v.error_fg_color = self.error;
        v.window_stroke = Stroke::new(1.0_f32, self.outline_variant);
        v.window_shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(if self.is_light { 40 } else { 120 }),
        };
        v.popup_shadow = egui::epaint::Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(if self.is_light { 30 } else { 100 }),
        };

        let cr = CornerRadius::same(8);
        v.widgets.noninteractive.bg_fill = self.surface_container;
        v.widgets.noninteractive.weak_bg_fill = self.surface;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, self.outline_variant);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, self.on_surface_variant);
        v.widgets.noninteractive.corner_radius = cr;

        v.widgets.inactive.bg_fill = self.secondary_container;
        v.widgets.inactive.weak_bg_fill = self.surface_container;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, self.outline_variant);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, self.on_surface);
        v.widgets.inactive.corner_radius = cr;

        v.widgets.hovered.bg_fill = self.primary_container;
        v.widgets.hovered.weak_bg_fill = self.primary_container;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, self.primary);
        v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, self.on_primary_container);
        v.widgets.hovered.corner_radius = cr;

        v.widgets.active.bg_fill = self.primary;
        v.widgets.active.weak_bg_fill = self.primary;
        v.widgets.active.bg_stroke = Stroke::new(1.0_f32, self.primary);
        v.widgets.active.fg_stroke = Stroke::new(1.5_f32, self.on_primary);
        v.widgets.active.corner_radius = cr;

        v.widgets.open.bg_fill = self.primary_container;
        v.widgets.open.weak_bg_fill = self.primary_container;
        v.widgets.open.bg_stroke = Stroke::new(1.0_f32, self.primary);
        v.widgets.open.fg_stroke = Stroke::new(1.0_f32, self.on_primary_container);
        v.widgets.open.corner_radius = cr;

        v.selection.bg_fill = self.primary;
        v.selection.stroke = Stroke::new(1.0_f32, self.on_primary);

        v
    }

    /// 生成乐谱渲染主题
    pub fn to_score_theme(&self) -> Theme {
        if self.is_light {
            Theme {
                staff_line: Color32::from_rgb(0xC8, 0xCE, 0xCA),
                bar_line: Color32::from_rgb(0x6A, 0x72, 0x6C),
                note_text: Color32::from_rgb(0x1A, 0x1C, 0x1A),
                rest_color: Color32::from_rgb(0x2A, 0x2E, 0x2C),
                selected_note: self.primary,
                cursor_color: Color32::from_rgba_unmultiplied(
                    self.primary.r(),
                    self.primary.g(),
                    self.primary.b(),
                    90,
                ),
                marker_color: Color32::from_rgb(0xC4, 0x7A, 0x20),
                // 谱面元素擦除底与纸张一致：纯白
                background: PAPER_WHITE,
                clef_color: self.on_surface_variant,
                time_sig_color: self.on_surface,
            }
        } else {
            Theme {
                staff_line: Color32::from_rgb(0x55, 0x60, 0x58),
                bar_line: Color32::from_rgb(0xA0, 0xAA, 0xA2),
                note_text: self.on_surface,
                rest_color: self.on_surface_variant,
                selected_note: self.primary,
                cursor_color: Color32::from_rgba_unmultiplied(
                    self.primary.r(),
                    self.primary.g(),
                    self.primary.b(),
                    110,
                ),
                marker_color: Color32::from_rgb(0xFF, 0xB7, 0x4D),
                background: self.surface,
                clef_color: self.on_surface_variant,
                time_sig_color: self.on_surface,
            }
        }
    }
}

fn apply_corner_radius(style: &mut Style, r: u8) {
    style.visuals.window_corner_radius = CornerRadius::same(r);
    style.visuals.menu_corner_radius = CornerRadius::same(r.saturating_sub(2));
}
