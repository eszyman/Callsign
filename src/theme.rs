//! Instrument colors. Amber is the live lamp. Everything else stays quiet.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

pub const BG: Color32 = Color32::from_rgb(0x12, 0x14, 0x10);
pub const NAV: Color32 = Color32::from_rgb(0x18, 0x1b, 0x14);
pub const SURFACE: Color32 = Color32::from_rgb(0x1e, 0x22, 0x19);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x28, 0x2d, 0x22);
pub const LINE: Color32 = Color32::from_rgb(0x3a, 0x40, 0x32);
pub const TEXT: Color32 = Color32::from_rgb(0xf3, 0xef, 0xe2);
pub const MUTED: Color32 = Color32::from_rgb(0xa3, 0x9e, 0x8c);
pub const FAINT: Color32 = Color32::from_rgb(0x6f, 0x6b, 0x5e);
pub const AMBER: Color32 = Color32::from_rgb(0xf0, 0xa2, 0x02);
pub const AMBER_DIM: Color32 = Color32::from_rgb(0x3a, 0x2e, 0x10);
pub const GREEN: Color32 = Color32::from_rgb(0xb7, 0xc4, 0x8a);
pub const RED: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x5f);
pub const BLUE: Color32 = Color32::from_rgb(0x9b, 0xb0, 0xc1);

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_visuals(ctx);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "inter_semibold".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "barlow".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/BarlowCondensed-Medium.ttf"
        ))),
    );
    fonts.font_data.insert(
        "mono".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mono".into());
    fonts
        .families
        .insert(FontFamily::Name("display".into()), vec!["barlow".into()]);
    fonts.families.insert(
        FontFamily::Name("semibold".into()),
        vec!["inter_semibold".into(), "inter".into()],
    );
    ctx.set_fonts(fonts);
}

fn install_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.extreme_bg_color = Color32::from_rgb(0x10, 0x12, 0x0e);
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE;
    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = AMBER;
    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.selection.bg_fill = AMBER_DIM;
    visuals.selection.stroke = Stroke::new(1.0, AMBER);
    let radius = CornerRadius::same(8);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
        widget.bg_stroke = Stroke::new(1.0, LINE);
        widget.fg_stroke = Stroke::new(1.0, TEXT);
    }
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_fill = SURFACE_2;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x32, 0x38, 0x2a);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x3d, 0x44, 0x32);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(15.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(15.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(22.0, FontFamily::Name("semibold".into())),
        );
    });
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(16)
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

pub fn app_icon() -> egui::IconData {
    let size = 64u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            let dx = x as i32 - 32;
            let dy = y as i32 - 32;
            let distance = dx * dx + dy * dy;
            if distance <= 13 * 13 {
                rgba[i] = 0xf0;
                rgba[i + 1] = 0xa2;
                rgba[i + 2] = 0x02;
            } else if distance <= 20 * 20 {
                rgba[i] = 0x8a;
                rgba[i + 1] = 0x64;
                rgba[i + 2] = 0x10;
            } else {
                rgba[i] = 0x1c;
                rgba[i + 1] = 0x1f;
                rgba[i + 2] = 0x18;
            }
            rgba[i + 3] = 255;
        }
    }
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
