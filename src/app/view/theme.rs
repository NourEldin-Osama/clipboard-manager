//! Colours, fonts and the egui style.

use super::*;

/// The accent colours; `accent_name` names them in the same order.
pub(crate) const ACCENTS: [Color32; 5] = [
    Color32::from_rgb(88, 101, 242),
    Color32::from_rgb(14, 140, 214),
    Color32::from_rgb(16, 160, 110),
    Color32::from_rgb(214, 128, 10),
    Color32::from_rgb(220, 60, 120),
];

pub(crate) fn accent_name(index: usize) -> String {
    match index {
        0 => t!("accent-indigo"),
        1 => t!("accent-sky"),
        2 => t!("accent-emerald"),
        3 => t!("accent-amber"),
        _ => t!("accent-rose"),
    }
}

pub(super) static ACCENT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn set_accent(i: usize) {
    ACCENT.store(i.min(ACCENTS.len() - 1), Ordering::Relaxed);
}

pub(crate) fn accent() -> Color32 {
    ACCENTS[ACCENT.load(Ordering::Relaxed)]
}

pub(crate) const GREEN: Color32 = Color32::from_rgb(34, 160, 90);

pub(crate) const AMBER: Color32 = Color32::from_rgb(214, 140, 20);

pub(crate) const RED: Color32 = Color32::from_rgb(214, 60, 60);

pub(crate) fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Text => Color32::from_rgb(90, 120, 230),
        Kind::Link => Color32::from_rgb(20, 150, 170),
        Kind::Image => Color32::from_rgb(220, 110, 50),
        Kind::Files => Color32::from_rgb(140, 100, 210),
    }
}

/// A stable colour per tag, so "#work" looks the same everywhere.
pub(crate) fn tag_color(tag: &str) -> Color32 {
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(88, 101, 242),
        Color32::from_rgb(14, 140, 214),
        Color32::from_rgb(16, 150, 110),
        Color32::from_rgb(200, 120, 10),
        Color32::from_rgb(210, 60, 120),
        Color32::from_rgb(130, 90, 210),
        Color32::from_rgb(20, 150, 170),
        Color32::from_rgb(200, 80, 50),
    ];
    let h = crate::store::fnv1a(tag.to_lowercase().as_bytes());
    PALETTE[(h % PALETTE.len() as u64) as usize]
}

pub(super) fn filter_icon(f: Filter) -> Icon {
    match f {
        Filter::All => Icon::Clipboard,
        Filter::Text => Icon::Text,
        Filter::Links => Icon::Link,
        Filter::Images => Icon::Image,
        Filter::Files => Icon::Folder,
        Filter::Pinned => Icon::PinFilled,
    }
}

pub(crate) fn mode_label(m: SearchMode) -> String {
    match m {
        SearchMode::Hybrid => t!("search-mode-smart"),
        SearchMode::Semantic => t!("search-mode-meaning"),
        SearchMode::Keyword => t!("search-mode-words"),
    }
}

pub(crate) fn mode_help(m: SearchMode) -> String {
    match m {
        SearchMode::Hybrid => t!("search-mode-smart-help"),
        SearchMode::Semantic => t!("search-mode-meaning-help"),
        SearchMode::Keyword => t!("search-mode-words-help"),
    }
}

/// Adds the bundled Arabic font after egui's own, so Latin text looks the same
/// and Arabic always has glyphs, whatever fonts the PC has.
pub(crate) fn add_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};
    let families = [FontFamily::Proportional, FontFamily::Monospace]
        .map(|family| InsertFontFamily {
            family,
            priority: FontPriority::Lowest,
        })
        .to_vec();
    ctx.add_font(FontInsert::new(
        "noto-sans-arabic",
        FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansArabic-Regular.ttf"
        )),
        families,
    ));
}

pub(crate) fn style(ctx: &egui::Context) {
    let a = accent();
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
        for w in [
            &mut s.visuals.widgets.noninteractive,
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.open,
        ] {
            w.corner_radius = CornerRadius::same(8);
        }
        // Flat controls: no outline until hovered.
        s.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        s.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
        s.visuals.window_corner_radius = CornerRadius::same(12);
        s.visuals.menu_corner_radius = CornerRadius::same(10);
        s.visuals.selection.bg_fill = a.gamma_multiply(0.35);
        s.visuals.selection.stroke = Stroke::new(1.0, a);
        s.visuals.hyperlink_color = a;
    });
    // Input fields need a visible fill: the content panes are already the
    // lightest (or darkest) tone, so a default field would vanish into them.
    // Gray text is dark (or light) enough to read on every background it
    // sits on: egui's own weak text was about 3 to 1, and readable text
    // needs 4.5 to 1 (WCAG AA). In the dark theme the normal text gets
    // brighter too, to stay apart from the weak text.
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.visuals.text_edit_bg_color = Some(Color32::from_gray(240));
        s.visuals.weak_text_color = Some(Color32::from_gray(100));
    });
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.visuals.text_edit_bg_color = Some(Color32::from_gray(38));
        s.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_gray(180);
        s.visuals.weak_text_color = Some(Color32::from_gray(150));
    });
}

pub(crate) fn apply_theme(ctx: &egui::Context, t: Theme) {
    ctx.set_theme(match t {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance of an opaque colour.
    fn luminance(c: Color32) -> f32 {
        let linear = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
    }

    /// WCAG contrast ratio: 4.5 or more is readable for normal text.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// A see-through (premultiplied) colour painted over an opaque one.
    fn over(top: Color32, under: Color32) -> Color32 {
        let rest = 1.0 - f32::from(top.a()) / 255.0;
        let mix = |t: u8, u: u8| (f32::from(t) + f32::from(u) * rest).round() as u8;
        Color32::from_rgb(
            mix(top.r(), under.r()),
            mix(top.g(), under.g()),
            mix(top.b(), under.b()),
        )
    }

    /// Every place gray text sits on, per accent: the plain backgrounds, the
    /// tints of a selected row (`list`), a chosen search mode (`mode_option`)
    /// and a changed setting (`setting_row`), a gray chip, and search hits.
    #[test]
    fn gray_text_is_readable_on_every_background() {
        let ctx = egui::Context::default();
        style(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let v = ctx.style_of(theme).visuals.clone();
            let dark = v.dark_mode;
            let (panel, content, weak) = (v.panel_fill, v.extreme_bg_color, v.weak_text_color());
            for a in ACCENTS {
                let selected_row = over(a.gamma_multiply(if dark { 0.16 } else { 0.12 }), panel);
                let backgrounds = [
                    ("panel", panel),
                    ("content", content),
                    ("menu", v.window_fill),
                    ("field", v.text_edit_bg_color()),
                    ("selected row", selected_row),
                    (
                        "selected settings page",
                        over(a.gamma_multiply(0.13), panel),
                    ),
                    (
                        "changed setting",
                        over(a.gamma_multiply(if dark { 0.14 } else { 0.07 }), content),
                    ),
                    (
                        "chosen search mode",
                        over(
                            a.gamma_multiply(if dark { 0.16 } else { 0.10 }),
                            v.window_fill,
                        ),
                    ),
                    ("gray chip", over(weak.gamma_multiply(0.14), content)),
                ];
                for (text, color) in [("normal", v.text_color()), ("weak", weak)] {
                    for (place, background) in backgrounds {
                        let ratio = contrast(color, background);
                        assert!(
                            ratio >= 4.5,
                            "{theme:?}, {a:?}: {text} text on {place} is {ratio:.2} to 1"
                        );
                    }
                }
                // A search hit: the strong text colour on the accent tint.
                for (place, under) in [
                    ("list", panel),
                    ("card", content),
                    ("selected row", selected_row),
                ] {
                    let ratio =
                        contrast(v.strong_text_color(), over(a.gamma_multiply(0.28), under));
                    assert!(
                        ratio >= 4.5,
                        "{theme:?}, {a:?}: a search hit in a {place} is {ratio:.2} to 1"
                    );
                }
            }
            assert!(
                contrast(weak, panel) < contrast(v.text_color(), panel),
                "{theme:?}: weak text stays weaker than normal text"
            );
        }
    }
}
