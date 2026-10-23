//! Colours, fonts and the egui style.

use super::*;

pub(crate) const ACCENTS: [(&str, Color32); 5] = [
    ("Indigo", Color32::from_rgb(88, 101, 242)),
    ("Sky", Color32::from_rgb(14, 140, 214)),
    ("Emerald", Color32::from_rgb(16, 160, 110)),
    ("Amber", Color32::from_rgb(214, 128, 10)),
    ("Rose", Color32::from_rgb(220, 60, 120)),
];

pub(super) static ACCENT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn set_accent(i: usize) {
    ACCENT.store(i.min(ACCENTS.len() - 1), Ordering::Relaxed);
}

pub(crate) fn accent() -> Color32 {
    ACCENTS[ACCENT.load(Ordering::Relaxed)].1
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
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.visuals.text_edit_bg_color = Some(Color32::from_gray(240))
    });
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.visuals.text_edit_bg_color = Some(Color32::from_gray(38))
    });
}

pub(crate) fn apply_theme(ctx: &egui::Context, t: Theme) {
    ctx.set_theme(match t {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
}
