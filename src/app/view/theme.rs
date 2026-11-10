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

/// Corner sizes (D62): every rounded thing takes one of these.
pub(crate) mod radius {
    /// Small parts: key caps, badges, thin bars.
    pub(crate) const SMALL: u8 = 4;
    /// Buttons, rows, fields and their highlights.
    pub(crate) const MEDIUM: u8 = 8;
    /// Cards and panels.
    pub(crate) const LARGE: u8 = 12;
    /// Pills, chips and switches: fully round ends.
    pub(crate) const ROUND: u8 = 255;
}

/// Gaps between things (D62), in steps of 4 points.
pub(crate) mod space {
    pub(crate) const TINY: f32 = 4.0;
    pub(crate) const SMALL: f32 = 8.0;
    pub(crate) const MEDIUM: f32 = 12.0;
    pub(crate) const LARGE: f32 = 16.0;
    pub(crate) const HUGE: f32 = 24.0;
    /// Above the message of an empty list or details pane.
    pub(crate) const EMPTY_PAGE: f32 = 64.0;
}

/// Laid over a filled button: lighter under the pointer, darker while it
/// is pressed.
pub(crate) fn lit(pressed: bool) -> Color32 {
    if pressed {
        Color32::from_black_alpha(26)
    } else {
        Color32::from_white_alpha(28)
    }
}

/// Keys, arrows and dividers drawn on a filled button.
pub(crate) fn on_button(alpha: u8) -> Color32 {
    Color32::from_white_alpha(alpha)
}

/// A key cap's face and edge.
pub(crate) fn key_cap(dark: bool) -> (Color32, Color32) {
    if dark {
        (Color32::from_gray(48), Color32::from_gray(70))
    } else {
        (Color32::WHITE, Color32::from_gray(210))
    }
}

/// The search field's fill.
pub(crate) fn search_fill(dark: bool) -> Color32 {
    if dark {
        Color32::from_gray(22)
    } else {
        Color32::WHITE
    }
}

/// A grid card's shadow, deeper under the pointer.
pub(crate) fn card_shadow(dark: bool, hot: bool) -> Color32 {
    Color32::from_black_alpha(match (dark, hot) {
        (true, _) => 90,
        (false, true) => 34,
        (false, false) => 20,
    })
}

/// The track behind a setting's choices, and the face of the chosen one
/// on it. Both stay where `text_shade` keeps colored text readable.
pub(crate) fn choice_track(dark: bool) -> (Color32, Color32) {
    if dark {
        (Color32::from_gray(40), Color32::from_gray(58))
    } else {
        (Color32::from_gray(236), Color32::WHITE)
    }
}

/// The background of the words a search found: a warm tint, apart from the
/// accent of the selection it can sit on.
pub(crate) fn search_hit(dark: bool) -> Color32 {
    AMBER.gamma_multiply(if dark { 0.40 } else { 0.30 })
}

/// The tint of a selected grid card: lighter than a selected row's, so the
/// card's gray text stays readable on it.
pub(crate) fn selected_card(accent: Color32, dark: bool) -> Color32 {
    accent.gamma_multiply(if dark { 0.16 } else { 0.05 })
}

/// The faint edge around a grid card in the dark theme, where its shadow
/// doesn't show; none in the light theme.
pub(crate) fn card_edge(dark: bool) -> Option<Color32> {
    dark.then(|| Color32::from_gray(70))
}

/// How pictures are tinted: a little dimmer in the dark theme, so a bright
/// picture doesn't glare next to the dark cards.
pub(crate) fn picture_tint(dark: bool) -> Color32 {
    if dark {
        Color32::from_gray(208)
    } else {
        Color32::WHITE
    }
}

/// The strip of buttons at the foot of a card under the pointer.
pub(crate) fn card_footer(dark: bool) -> Color32 {
    if dark {
        Color32::from_gray(44)
    } else {
        Color32::from_gray(238)
    }
}

/// The shadow under the day header that stays at the top.
pub(crate) fn header_shadow() -> Color32 {
    Color32::from_black_alpha(36)
}

/// The shade over the window while files are dragged over it.
pub(crate) fn drop_shade() -> Color32 {
    Color32::from_black_alpha(160)
}

/// A message's button: near black in the light theme, near white in the
/// dark one, as Sonner's. The fill, then the text.
pub(crate) fn message_button(dark: bool) -> (Color32, Color32) {
    if dark {
        (Color32::from_gray(235), Color32::from_gray(20))
    } else {
        (Color32::from_gray(28), Color32::WHITE)
    }
}

/// A thin progress line's track and fill: the accent on a faint tint of
/// it. A stronger tint looks like a full line in the dark theme.
pub(crate) fn progress_colors(dark: bool) -> (Color32, Color32) {
    let a = accent();
    (a.gamma_multiply(if dark { 0.15 } else { 0.18 }), a)
}

/// The soft line between groups of a menu. egui's own separator uses the
/// outline of things you can't click, which this theme leaves out.
pub(crate) fn line_color(dark_mode: bool) -> Color32 {
    Color32::from_gray(if dark_mode { 60 } else { 222 })
}

/// `color` moved toward black on a light `background`, or toward white on a
/// dark one, just enough for text to read on it: 4.5 to 1, the WCAG AA
/// minimum for normal text.
pub(crate) fn readable(color: Color32, background: Color32) -> Color32 {
    // Above about 0.18, black text has more contrast than white.
    let toward = if luminance(background) > 0.18 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    (0..=50)
        .map(|step| color.lerp_to_gamma(toward, step as f32 / 50.0))
        .find(|c| contrast(*c, background) >= 4.5)
        .unwrap_or(toward)
}

/// `color` for text: darker in the light theme, lighter in the dark one.
/// Coloured text also sits on tinted chips, rows and buttons; these grays are
/// as hard to read on as the hardest of them (the tests check each place).
/// Fills, outlines and icons on their own keep the plain colour.
pub(crate) fn text_shade(color: Color32, dark_mode: bool) -> Color32 {
    readable(color, Color32::from_gray(if dark_mode { 62 } else { 220 }))
}

/// `color` as the fill behind white text, darkened just enough to read.
pub(crate) fn button_fill(color: Color32) -> Color32 {
    readable(color, Color32::WHITE)
}

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

pub(crate) fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Text => Color32::from_rgb(90, 120, 230),
        Kind::Link => Color32::from_rgb(20, 150, 170),
        Kind::Image => Color32::from_rgb(220, 110, 50),
        Kind::Files => Color32::from_rgb(140, 100, 210),
    }
}

/// The tag colours; `tag_color` picks one for each tag.
const TAG_COLORS: [Color32; 8] = [
    Color32::from_rgb(88, 101, 242),
    Color32::from_rgb(14, 140, 214),
    Color32::from_rgb(16, 150, 110),
    Color32::from_rgb(200, 120, 10),
    Color32::from_rgb(210, 60, 120),
    Color32::from_rgb(130, 90, 210),
    Color32::from_rgb(20, 150, 170),
    Color32::from_rgb(200, 80, 50),
];

/// A stable colour per tag, so "#work" looks the same everywhere.
pub(crate) fn tag_color(tag: &str) -> Color32 {
    let h = crate::store::fnv1a(tag.to_lowercase().as_bytes());
    TAG_COLORS[(h % TAG_COLORS.len() as u64) as usize]
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

/// The five text sizes: small text such as times and counts; controls such as
/// buttons and chips; body text in rows and settings; large text in the
/// search field and empty lists; and page titles.
pub(crate) mod size {
    pub(crate) const SMALL: f32 = 12.0;
    pub(crate) const CONTROL: f32 = 13.0;
    pub(crate) const BODY: f32 = 14.0;
    pub(crate) const LARGE: f32 = 16.0;
    pub(crate) const TITLE: f32 = 20.0;
}

/// The widest a line of long text gets, in multiples of its text size:
/// about 70 characters.
pub(crate) const READING_WIDTH: f32 = 35.0;

/// The font family of titles: Noto Sans and Noto Sans Arabic in SemiBold.
const SEMIBOLD: &str = "semibold";

/// The SemiBold font in `size`, for titles.
pub(crate) fn semibold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name(SEMIBOLD.into()))
}

/// The bundled fonts: Noto Sans (Latin, Greek and Cyrillic) and Noto Sans
/// Arabic, each in Regular and SemiBold, so both scripts look alike in one
/// weight whatever fonts the PC has. egui's own fonts stay after them for
/// symbols and emoji, and code keeps its monospace font.
pub(crate) fn add_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    let mut fonts = FontDefinitions::default();
    let bundled: [(&str, &'static [u8]); 4] = [
        (
            "noto-sans",
            include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf"),
        ),
        (
            "noto-sans-semibold",
            include_bytes!("../../../assets/fonts/NotoSans-SemiBold.ttf"),
        ),
        (
            "noto-sans-arabic",
            include_bytes!("../../../assets/fonts/NotoSansArabic-Regular.ttf"),
        ),
        (
            "noto-sans-arabic-semibold",
            include_bytes!("../../../assets/fonts/NotoSansArabic-SemiBold.ttf"),
        ),
    ];
    for (name, bytes) in bundled {
        fonts.font_data.insert(
            name.into(),
            std::sync::Arc::new(FontData::from_static(bytes)),
        );
    }
    let fallbacks: Vec<String> = fonts.families[&FontFamily::Proportional]
        .iter()
        .filter(|name| *name != "Ubuntu-Light")
        .cloned()
        .collect();
    let family = |first: [&str; 2]| -> Vec<String> {
        first
            .iter()
            .map(|name| name.to_string())
            .chain(fallbacks.iter().cloned())
            .collect()
    };
    fonts.families.insert(
        FontFamily::Proportional,
        family(["noto-sans", "noto-sans-arabic"]),
    );
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD.into()),
        family(["noto-sans-semibold", "noto-sans-arabic-semibold"]),
    );
    if let Some(code) = fonts.families.get_mut(&FontFamily::Monospace) {
        let after_text_fonts = code
            .iter()
            .position(|name| name == "Ubuntu-Light")
            .map_or(code.len(), |at| at + 1);
        code.insert(after_text_fonts, "noto-sans-arabic".into());
    }
    ctx.set_fonts(fonts);
}

pub(crate) fn style(ctx: &egui::Context, panels: crate::config::DarkPanels) {
    let a = accent();
    ctx.all_styles_mut(|s| {
        use egui::TextStyle;
        // egui's own widgets (text fields, menus, tooltips) in the same sizes.
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(size::SMALL)),
            (TextStyle::Button, FontId::proportional(size::CONTROL)),
            (TextStyle::Body, FontId::proportional(size::BODY)),
            (TextStyle::Monospace, FontId::monospace(size::CONTROL)),
            (TextStyle::Heading, semibold(size::TITLE)),
        ]
        .into();
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        // Nothing moves when Windows' "Animation effects" is off.
        if !crate::screen::animations_on() {
            s.animation_time = 0.0;
        }
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
        // Raised surfaces lighter (D58): the window darkest, panes and cards
        // a step up, menus and tooltips another, each step about twice the
        // light theme's.
        if panels == crate::config::DarkPanels::Lighter {
            s.visuals.panel_fill = Color32::from_gray(18);
            s.visuals.extreme_bg_color = Color32::from_gray(30);
            s.visuals.window_fill = Color32::from_gray(38);
            s.visuals.window_stroke.color = Color32::from_gray(58);
            s.visuals.widgets.noninteractive.bg_fill = Color32::from_gray(18);
            s.visuals.widgets.noninteractive.weak_bg_fill = Color32::from_gray(18);
        }
        s.visuals.text_edit_bg_color = Some(Color32::from_gray(38));
        s.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_gray(180);
        // Lighter menus need lighter gray text to stay readable on them.
        let weak = if panels == crate::config::DarkPanels::Lighter {
            160
        } else {
            150
        };
        s.visuals.weak_text_color = Some(Color32::from_gray(weak));
    });
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |s| accent_visuals(&mut s.visuals, a));
    }
}

/// The accent's parts of egui's own style. Links take the accent's text
/// shade. egui draws selected text, a selected menu entry and a progress
/// bar's label in the selection stroke colour on the selection tint, so that
/// colour is the accent shade that reads on the tint.
fn accent_visuals(v: &mut egui::Visuals, a: Color32) {
    v.hyperlink_color = text_shade(a, v.dark_mode);
    v.selection.bg_fill = a.gamma_multiply(0.35);
    // The tint is hardest to read on over a text field, the background
    // nearest to middle gray in both themes.
    let tint = over(v.selection.bg_fill, v.text_edit_bg_color());
    v.selection.stroke = Stroke::new(1.0, readable(a, tint));
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

    /// Screen code takes its corner sizes, gaps and colors from this file
    /// (D62): a raw one anywhere else in the window's code fails here.
    #[test]
    fn screen_code_uses_named_sizes_and_colors() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app");
        let mut found = Vec::new();
        let mut paths = vec![root];
        while let Some(path) = paths.pop() {
            if path.is_dir() {
                for entry in std::fs::read_dir(&path).unwrap() {
                    paths.push(entry.unwrap().path());
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") || path.ends_with("view/theme.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            // Tests may use any value.
            let code = text.split("#[cfg(test)]").next().unwrap_or("");
            for (n, line) in code.lines().enumerate() {
                let number_in = |call: &str| {
                    line.split(call)
                        .skip(1)
                        .any(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
                };
                let raw = number_in("CornerRadius::same(")
                    || number_in("add_space(")
                    || (line.contains("Color32::from_") && !line.contains("// copied color"));
                if raw {
                    found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
        assert!(
            found.is_empty(),
            "raw sizes or colors:\n{}",
            found.join("\n")
        );
    }

    /// A context styled with each dark-panels choice, in each theme.
    fn styled() -> Vec<(egui::Context, egui::Theme)> {
        use crate::config::DarkPanels;
        [DarkPanels::Darker, DarkPanels::Lighter]
            .into_iter()
            .flat_map(|panels| {
                let ctx = egui::Context::default();
                style(&ctx, panels);
                [egui::Theme::Light, egui::Theme::Dark].map(|theme| (ctx.clone(), theme))
            })
            .collect()
    }

    /// Every place gray text sits on, per accent: the plain backgrounds, the
    /// tints of a selected row (`list`), a chosen search mode (`mode_option`)
    /// and a changed setting (`setting_row`), a gray chip, and search hits.
    #[test]
    fn gray_text_is_readable_on_every_background() {
        for (ctx, theme) in styled() {
            let v = ctx.style_of(theme).visuals.clone();
            let dark = v.dark_mode;
            let (panel, content, weak) = (v.panel_fill, v.extreme_bg_color, v.weak_text_color());
            for a in ACCENTS {
                let selected_row = over(a.gamma_multiply(if dark { 0.16 } else { 0.12 }), panel);
                let backgrounds = [
                    ("panel", panel),
                    ("content", content),
                    ("selected card", over(selected_card(a, dark), content)),
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
                // A search hit: the strong text colour on its warm tint.
                for (place, under) in [
                    ("list", panel),
                    ("card", content),
                    ("selected row", selected_row),
                    ("selected card", over(selected_card(a, dark), content)),
                ] {
                    let ratio = contrast(v.strong_text_color(), over(search_hit(dark), under));
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

    /// Coloured text in its text shade on every background it sits on, as the
    /// chips, pills, rows, menus and setting rows draw them; white text on the
    /// main and delete buttons; egui's selected text. For every accent.
    #[test]
    fn colored_text_is_readable_on_every_background() {
        for (ctx, theme) in styled() {
            let base = ctx.style_of(theme).visuals.clone();
            let dark = base.dark_mode;
            let (panel, content) = (base.panel_fill, base.extreme_bg_color);
            // (what and where, text colour, background)
            let mut cases: Vec<(String, Color32, Color32)> = Vec::new();
            let kinds = [Kind::Text, Kind::Link, Kind::Image, Kind::Files].map(kind_color);
            for c in kinds.into_iter().chain(TAG_COLORS) {
                let text = text_shade(c, dark);
                let tint = if dark { 0.3 } else { 0.13 };
                for (place, background) in [
                    ("chip", over(c.gamma_multiply(0.14), content)),
                    ("card chip or tag", over(c.gamma_multiply(tint), content)),
                    ("chosen tag filter", over(c.gamma_multiply(0.13), panel)),
                    ("tag suggestion", content),
                ] {
                    cases.push((format!("{c:?} on {place}"), text, background));
                }
            }
            for a in ACCENTS {
                let mut v = base.clone();
                accent_visuals(&mut v, a);
                let changed = over(a.gamma_multiply(if dark { 0.14 } else { 0.07 }), content);
                let banner = panel.lerp_to_gamma(a, if dark { 0.18 } else { 0.10 });
                let text = text_shade(a, dark);
                for (place, background) in [
                    ("panel", panel),
                    ("content", content),
                    ("menu", v.window_fill),
                    (
                        "selected settings page",
                        over(a.gamma_multiply(0.13), panel),
                    ),
                    ("chosen preset", over(a.gamma_multiply(0.13), content)),
                    ("pinned chip", over(a.gamma_multiply(0.14), content)),
                    ("changed setting", changed),
                    ("chosen option", over(a.gamma_multiply(0.14), changed)),
                    ("search mode button", over(a.gamma_multiply(0.12), panel)),
                    (
                        "chosen search mode",
                        over(
                            a.gamma_multiply(if dark { 0.16 } else { 0.10 }),
                            v.window_fill,
                        ),
                    ),
                    (
                        "shortcut being recorded",
                        over(a.gamma_multiply(0.10), changed),
                    ),
                    ("update banner", banner),
                ] {
                    cases.push((format!("{a:?} on {place}"), text, background));
                }
                for (place, background) in [("card", content), ("update banner", banner)] {
                    cases.push((
                        format!("{a:?} link on {place}"),
                        v.hyperlink_color,
                        background,
                    ));
                }
                for s in [RED, AMBER, GREEN] {
                    let text = text_shade(s, dark);
                    for (place, background) in [
                        ("status bar", panel),
                        ("settings", content),
                        ("chip", over(s.gamma_multiply(0.14), panel)),
                        (
                            "chip in a changed setting",
                            over(s.gamma_multiply(0.14), changed),
                        ),
                        (
                            "shortcut being recorded",
                            over(a.gamma_multiply(0.10), changed),
                        ),
                        ("update banner", banner),
                        ("button", v.widgets.inactive.weak_bg_fill),
                    ] {
                        cases.push((format!("{s:?} on {place}, accent {a:?}"), text, background));
                    }
                }
                for (place, fill) in [
                    ("main button", button_fill(a)),
                    ("delete button", button_fill(RED)),
                ] {
                    cases.push((
                        format!("white on {place}, accent {a:?}"),
                        Color32::WHITE,
                        fill,
                    ));
                }
                // egui's selected text, selected menu entry and progress label.
                let selected = v.selection.stroke.color;
                for (place, under) in [
                    ("field", v.text_edit_bg_color()),
                    ("menu", v.window_fill),
                    ("content", content),
                ] {
                    let background = over(v.selection.bg_fill, under);
                    cases.push((format!("{a:?} selected in {place}"), selected, background));
                }
                cases.push((format!("{a:?} progress label"), selected, content));
            }
            for (case, text, background) in cases {
                let ratio = contrast(text, background);
                assert!(ratio >= 4.5, "{theme:?}: {case} is {ratio:.2} to 1");
            }
        }
    }
}
