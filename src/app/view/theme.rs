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

/// A card's shadow, deeper under the pointer.
fn card_shadow(dark: bool, hot: bool) -> Color32 {
    Color32::from_black_alpha(match (dark, hot) {
        (true, _) => 90,
        (false, true) => 34,
        (false, false) => 20,
    })
}

/// The surface of a card, in the grid and in Settings: the content's fill,
/// a soft shadow, deeper under the pointer, and the edge of the dark theme
/// (`card_edge`). The edge takes its room in the light theme too, unseen,
/// so what a card holds sits in the same place in both themes.
pub(crate) fn card_frame(v: &egui::Visuals, hot: bool) -> egui::Frame {
    let edge = card_edge(v).unwrap_or(Color32::TRANSPARENT);
    egui::Frame::new()
        .fill(v.extreme_bg_color)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(CornerRadius::same(radius::LARGE))
        .shadow(egui::Shadow {
            offset: [0, 2],
            blur: if hot { 14 } else { 8 },
            spread: 0,
            color: card_shadow(v.dark_mode, hot),
        })
}

/// The faint neutral tint behind a note in a settings card: a little darker
/// than the card in the light theme, a little lighter in the dark one.
pub(crate) fn note_fill(dark: bool) -> Color32 {
    if dark {
        Color32::from_white_alpha(12)
    } else {
        Color32::from_black_alpha(12)
    }
}

/// The faint amber tint behind a warning in a settings card, where its
/// icon takes the amber's text shade.
pub(crate) fn warning_fill(dark: bool) -> Color32 {
    AMBER.gamma_multiply(if dark { 0.14 } else { 0.10 })
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

/// The tint of a chosen chip, such as a filter: stronger in dark, where a
/// faint tint is lost on the dark panel.
pub(crate) fn chosen_pill(color: Color32, dark: bool) -> Color32 {
    color.gamma_multiply(if dark { 0.26 } else { 0.13 })
}

/// The face of a choice's chosen option, tinted with the accent so the
/// chosen one is the colored one: on the light face in the light theme, on
/// the darker track in dark, where the accent's light shade stays readable.
pub(crate) fn chosen_face(accent: Color32, dark: bool) -> Color32 {
    let (track, face) = choice_track(dark);
    if dark {
        track.blend(accent.gamma_multiply(0.22))
    } else {
        face.blend(accent.gamma_multiply(0.12))
    }
}

/// The background of the words a search found: a saturated amber that
/// reads at a glance, apart from the accent of the selection. Opaque, so it
/// looks the same on every tint it sits on, and the strong text color reads
/// on it at 7 to 1 or more: black on a bright amber, white on a deep one.
pub(crate) fn search_hit(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(122, 78, 0)
    } else {
        Color32::from_rgb(255, 200, 70)
    }
}

/// The status bar's fill while recording is paused: the window's gray with
/// a faint amber tint, opaque as a panel's fill must be.
pub(crate) fn paused_bar(panel: Color32, dark: bool) -> Color32 {
    panel.lerp_to_gamma(AMBER, if dark { 0.12 } else { 0.10 })
}

/// The tint of a selected grid card: lighter than a selected row's, so the
/// card's gray text stays readable on it.
pub(crate) fn selected_card(accent: Color32, dark: bool) -> Color32 {
    accent.gamma_multiply(if dark { 0.20 } else { 0.05 })
}

/// The line between a menu's groups: a step stronger than the other
/// lines, so it shows on the menu's face.
pub(crate) fn menu_line(dark: bool) -> Color32 {
    Color32::from_gray(if dark { 76 } else { 204 })
}

/// The gray of a dark card's faint edge before it was made brighter.
const FAINT_CARD_EDGE: u8 = 70;

/// The edge around a card in the dark theme, where its shadow doesn't
/// show: twice as far above the card's own gray as the faint edge it had,
/// so it shows with both Dark panels choices (gray 110 around Lighter's
/// cards, 130 around Darker's). None in the light theme.
fn card_edge(v: &egui::Visuals) -> Option<Color32> {
    v.dark_mode.then(|| {
        let card = i32::from(v.extreme_bg_color.r());
        let edge = card + 2 * (i32::from(FAINT_CARD_EDGE) - card);
        Color32::from_gray(edge.clamp(0, 255) as u8)
    })
}

/// How bright a picture in a card or a row is at rest in the dark theme:
/// about 80%, so a bright screenshot doesn't glare among the dark cards.
const RESTING_PICTURE: u8 = 204;

/// How long a picture in a card or a row takes to light up or dim again,
/// in seconds.
pub(crate) const PICTURE_FADE: f32 = 0.12;

/// How a picture is tinted. In the dark theme a picture in a card or a row
/// rests at about 80% of its brightness and shows in full while `lit` is
/// 1: while its card or row is under the pointer or selected, and always
/// where it is shown large, as in the details. In between while it fades.
/// The light theme shows every picture in full.
pub(crate) fn picture_tint(dark: bool, lit: f32) -> Color32 {
    if dark {
        Color32::from_gray(RESTING_PICTURE).lerp_to_gamma(Color32::WHITE, lit)
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

/// A menu's shadow: soft and all around, a little lower than the menu,
/// `alpha` dark at its core.
fn menu_shadow(alpha: u8) -> egui::Shadow {
    egui::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
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

/// A tag's text, in its color: the text shade, a step lighter in the dark
/// theme.
pub(crate) fn tag_shade(color: Color32, dark_mode: bool) -> Color32 {
    if dark_mode {
        readable(color, Color32::from_gray(80))
    } else {
        text_shade(color, false)
    }
}

/// A kind's symbol on a popup's row: the text shade of its `color`, 40%
/// darker in the light theme (red, green and blue each times 0.6), so the
/// thin symbols read at the popup's small size. The dark theme keeps the
/// text shade.
pub(crate) fn popup_symbol(color: Color32, dark_mode: bool) -> Color32 {
    let shade = text_shade(color, dark_mode);
    if dark_mode {
        return shade;
    }
    let darker = |value: u8| (f32::from(value) * 0.6).round() as u8;
    Color32::from_rgb(darker(shade.r()), darker(shade.g()), darker(shade.b()))
}

/// The pin of a pinned item on rows and cards: the `accent`, in its text
/// shade in the dark theme, where the plain accent is dim.
pub(crate) fn pin_color(accent: Color32, dark_mode: bool) -> Color32 {
    if dark_mode {
        text_shade(accent, true)
    } else {
        accent
    }
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

/// How strong a scroll bar's handle is while the pointer is elsewhere:
/// faint, a little more than half as strong as with the pointer over the
/// list.
const RESTING_SCROLL_HANDLE: f32 = 0.35;

/// The font family of titles: Noto Sans and Noto Sans Arabic in SemiBold.
const SEMIBOLD: &str = "semibold";

/// The SemiBold font in `size`, for titles.
pub(crate) fn semibold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name(SEMIBOLD.into()))
}

/// How much larger Arabic is drawn than Latin text of the same size: at one
/// size, Noto Sans Arabic looks smaller than the Noto Sans beside it.
/// egui takes a line's height from its first font, Noto Sans, so lines keep
/// their height, and it centres the taller Arabic face in them: scaled, the
/// Arabic stays centred and its baseline moves a little toward the Latin
/// one, so no vertical offset is needed.
const ARABIC_SCALE: f32 = 1.1;

/// The bundled fonts: Noto Sans (Latin, Greek and Cyrillic) and Noto Sans
/// Arabic, each in Regular and SemiBold, so both scripts look alike in one
/// weight whatever fonts the PC has. egui's own fonts stay after them for
/// symbols and emoji, and code keeps its monospace font.
pub(crate) fn add_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

fn font_definitions() -> egui::FontDefinitions {
    use eframe::egui::{FontData, FontDefinitions, FontFamily, FontTweak};
    let mut fonts = FontDefinitions::default();
    let bundled: [(&str, &'static [u8], f32); 4] = [
        (
            "noto-sans",
            include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf"),
            1.0,
        ),
        (
            "noto-sans-semibold",
            include_bytes!("../../../assets/fonts/NotoSans-SemiBold.ttf"),
            1.0,
        ),
        (
            "noto-sans-arabic",
            include_bytes!("../../../assets/fonts/NotoSansArabic-Regular.ttf"),
            ARABIC_SCALE,
        ),
        (
            "noto-sans-arabic-semibold",
            include_bytes!("../../../assets/fonts/NotoSansArabic-SemiBold.ttf"),
            ARABIC_SCALE,
        ),
    ];
    for (name, bytes, scale) in bundled {
        let tweak = FontTweak {
            scale,
            ..FontTweak::default()
        };
        fonts.font_data.insert(
            name.into(),
            std::sync::Arc::new(FontData::from_static(bytes).tweak(tweak)),
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
    fonts
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
        // A list or a page shows that it goes on: the scroll bar's handle
        // stays faintly in sight while the content is longer than its
        // area, and grows stronger under the pointer. The bar still floats,
        // taking no room from the content.
        s.spacing.scroll.dormant_handle_opacity = RESTING_SCROLL_HANDLE;
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
        // Menus float clearly above the white cards under them.
        s.visuals.popup_shadow = menu_shadow(46);
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
        s.visuals.popup_shadow = menu_shadow(120);
        s.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_gray(200);
        // Gray text reads at 5.5 to 1 or more on the dark surfaces, a step
        // above the least; lighter cards and menus need it lighter.
        let weak = if panels == crate::config::DarkPanels::Lighter {
            176
        } else {
            165
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

    /// `lay_out` in a frame of a window with the fonts `fonts`, at `scale`
    /// pixels per point.
    fn with_fonts<T>(
        fonts: egui::FontDefinitions,
        scale: f32,
        lay_out: impl Fn(&mut egui::Ui) -> T,
    ) -> T {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(scale);
        ctx.set_fonts(fonts);
        // The fonts apply from the next frame; the font atlas uploads no
        // renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut laid = None;
        ctx.run_ui(egui::RawInput::default(), |ui| laid = Some(lay_out(ui)))
            .textures_delta
            .clear();
        laid.expect("one frame")
    }

    /// Arabic is drawn 10% larger. Latin text is laid out exactly as
    /// without that, lines keep their height, and Arabic beside Latin keeps
    /// its baseline or comes nearer to the Latin one.
    #[test]
    fn arabic_is_larger_and_latin_text_does_not_change() {
        // The bundled fonts as they were: none of them tweaked.
        let mut unscaled = font_definitions();
        let bundled = [
            "noto-sans",
            "noto-sans-semibold",
            "noto-sans-arabic",
            "noto-sans-arabic-semibold",
        ];
        for name in bundled {
            let mut data = unscaled.font_data[name].as_ref().clone();
            data.tweak = egui::FontTweak::default();
            unscaled
                .font_data
                .insert(name.into(), std::sync::Arc::new(data));
        }
        let fonts = [
            FontId::proportional(size::SMALL),
            FontId::proportional(size::CONTROL),
            FontId::proportional(size::BODY),
            FontId::proportional(size::LARGE),
            FontId::proportional(size::TITLE),
            semibold(size::TITLE),
            FontId::monospace(size::CONTROL),
        ];
        // For each font: the Latin text's size and where each letter goes,
        // the Arabic text's size, and in a line of both, where the Latin
        // and the Arabic letters stand.
        type Laid = (Vec2, Vec<Pos2>, Vec2, (f32, f32));
        let lay_out = |ui: &mut egui::Ui| -> Vec<Laid> {
            let galley = |text: &str, font: &FontId| {
                ui.fonts_mut(|f| f.layout_no_wrap(text.into(), font.clone(), Color32::WHITE))
            };
            fonts
                .iter()
                .map(|font| {
                    let latin = galley("The quick brown fox jumps over 12 lazy dogs!", font);
                    let arabic = galley("موعد الاجتماع يوم الخميس", font);
                    let both = galley("Hx مرحبًا", font);
                    let glyphs = &both.rows[0].glyphs;
                    let baselines = (glyphs[0].pos.y, glyphs[glyphs.len() - 1].pos.y);
                    let letters = latin.rows[0].glyphs.iter().map(|g| g.pos).collect();
                    (latin.size(), letters, arabic.size(), baselines)
                })
                .collect()
        };
        for scale in [1.0, 1.25, 1.5] {
            let before = with_fonts(unscaled.clone(), scale, lay_out);
            let after = with_fonts(font_definitions(), scale, lay_out);
            for (font, (before, after)) in fonts.iter().zip(before.iter().zip(&after)) {
                let case = format!("{font:?} at {scale}");
                assert_eq!(before.0, after.0, "{case}: the Latin text's size");
                assert_eq!(before.1, after.1, "{case}: the Latin letters");
                // The Arabic words grow by a tenth; its spaces are Latin.
                let wider = after.2.x / before.2.x;
                assert!((1.07..=1.1).contains(&wider), "{case}: {wider}");
                assert_eq!(before.2.y, after.2.y, "{case}: the line's height");
                let (latin, arabic) = (before.3 .0, before.3 .1);
                assert_eq!(after.3 .0, latin, "{case}: the Latin baseline");
                let apart = |at: f32| (at - latin).abs();
                assert!(
                    apart(after.3 .1) <= apart(arabic),
                    "{case}: {before:?} {after:?}"
                );
            }
        }
    }

    /// In the dark theme a picture rests at about 80% of its brightness and
    /// shows in full when lit; the light theme shows it in full.
    #[test]
    fn a_picture_rests_dimmer_only_in_the_dark_theme() {
        assert_eq!(picture_tint(true, 0.0), Color32::from_gray(204));
        assert_eq!(picture_tint(true, 1.0), Color32::WHITE);
        let fading = picture_tint(true, 0.5);
        assert!(fading.r() > 204 && fading.r() < 255, "{fading:?}");
        for lit in [0.0, 0.5, 1.0] {
            assert_eq!(picture_tint(false, lit), Color32::WHITE);
        }
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
    /// tints of a selected row (`list`, where the popup's numbers are gray
    /// too), a chosen search mode (`mode_option`) and a changed setting
    /// (`card_row`), a gray chip, a note in a settings card, and search
    /// hits. A row under the pointer keeps 4.5 to 1.
    #[test]
    fn gray_text_is_readable_on_every_background() {
        for (ctx, theme) in styled() {
            let v = ctx.style_of(theme).visuals.clone();
            let dark = v.dark_mode;
            let (panel, content, weak) = (v.panel_fill, v.extreme_bg_color, v.weak_text_color());
            let hovered_row = over(v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6), panel);
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
                    // Also search by meaning's state while it has no color.
                    ("gray chip", over(weak.gamma_multiply(0.14), content)),
                    // "+N tags" on a list row at rest; on a tinted row it
                    // is on the panel's color.
                    ("more tags on a row", over(weak.gamma_multiply(0.12), panel)),
                    ("settings note", over(note_fill(dark), content)),
                    ("settings warning", over(warning_fill(dark), content)),
                    ("status bar while paused", paused_bar(panel, dark)),
                ];
                // Gray text a step above the least in the dark theme.
                let weak_least = if dark { 5.5 } else { 4.5 };
                for (text, color, least) in
                    [("normal", v.text_color(), 4.5), ("weak", weak, weak_least)]
                {
                    for (place, background) in backgrounds {
                        let ratio = contrast(color, background);
                        assert!(
                            ratio >= least,
                            "{theme:?}, {a:?}: {text} text on {place} is {ratio:.2} to 1"
                        );
                    }
                }
                // A search hit: the strong text colour on its amber, which
                // covers what is under it.
                assert_eq!(search_hit(dark).a(), 255, "{theme:?}: an opaque hit");
                for (place, under) in [
                    ("list", panel),
                    ("card", content),
                    ("selected row", selected_row),
                    ("selected card", over(selected_card(a, dark), content)),
                ] {
                    let ratio = contrast(v.strong_text_color(), over(search_hit(dark), under));
                    assert!(
                        ratio >= 7.0,
                        "{theme:?}, {a:?}: a search hit in a {place} is {ratio:.2} to 1"
                    );
                }
            }
            assert!(
                contrast(weak, panel) < contrast(v.text_color(), panel),
                "{theme:?}: weak text stays weaker than normal text"
            );
            // A row under the pointer, where the popup's numbers and a list
            // row's second line stay in sight: the least for normal text.
            for (text, color) in [("normal", v.text_color()), ("weak", weak)] {
                let ratio = contrast(color, hovered_row);
                assert!(
                    ratio >= 4.5,
                    "{theme:?}: {text} text on a row under the pointer is {ratio:.2} to 1"
                );
            }
            // Pause recording in the status bar under the pointer.
            let ratio = contrast(v.text_color(), v.widgets.hovered.weak_bg_fill);
            assert!(
                ratio >= 4.5,
                "{theme:?}: a quiet button under the pointer is {ratio:.2} to 1"
            );
            // The status bar's key hints, which stay while recording is
            // paused: each key's name on its cap, and the words on the
            // bar's amber tint.
            let (cap, _) = key_cap(dark);
            assert_eq!(cap.a(), 255, "{theme:?}: an opaque key cap");
            for (place, background) in [
                ("a key's name on its cap", cap),
                (
                    "a key hint's words on the paused status bar",
                    paused_bar(panel, dark),
                ),
            ] {
                let ratio = contrast(v.text_color(), background);
                assert!(ratio >= 4.5, "{theme:?}: {place} is {ratio:.2} to 1");
            }
        }
    }

    /// A list row's tag chips, and its "+N tags", stay pills on every row
    /// state, for every accent: at rest a tint over the panel, and under
    /// the pointer or selected the panel's own color. Each chip's fill and
    /// the row's differ by 1.1 to 1 or more, and the chip's text reads on
    /// it at 4.5 to 1 or more.
    #[test]
    fn a_rows_tag_chips_stand_out_on_every_row_state() {
        for (ctx, theme) in styled() {
            let v = ctx.style_of(theme).visuals.clone();
            let (dark, panel, weak) = (v.dark_mode, v.panel_fill, v.weak_text_color());
            let tint = if dark { 0.3 } else { 0.13 };
            let hovered = over(v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6), panel);
            // The chips on a row, each as its text's color and its fill.
            let at_rest: Vec<(Color32, Color32)> = TAG_COLORS
                .iter()
                .map(|c| (tag_shade(*c, dark), over(c.gamma_multiply(tint), panel)))
                .chain([(weak, over(weak.gamma_multiply(0.12), panel))])
                .collect();
            let tinted: Vec<(Color32, Color32)> = TAG_COLORS
                .iter()
                .map(|c| (tag_shade(*c, dark), panel))
                .chain([(weak, panel)])
                .collect();
            for a in ACCENTS {
                let selected = over(a.gamma_multiply(if dark { 0.16 } else { 0.12 }), panel);
                for (state, row, chips) in [
                    ("at rest", panel, &at_rest),
                    ("under the pointer", hovered, &tinted),
                    ("selected", selected, &tinted),
                ] {
                    for (text, fill) in chips {
                        let case = format!("{theme:?}, {a:?}, {state}, text {text:?}");
                        let apart = contrast(*fill, row);
                        assert!(apart >= 1.1, "{case}: chip and row {apart:.3} to 1");
                        let ratio = contrast(*text, *fill);
                        assert!(ratio >= 4.5, "{case}: text {ratio:.2} to 1");
                    }
                }
            }
        }
    }

    /// A selected grid card's tag chips, and its "+N tags", are the card's
    /// own color at rest, for every accent: opaque, apart from the selected
    /// card's tint, and each chip's text reads on it at 4.5 to 1 or more.
    /// In the dark theme chip and card differ by 1.1 to 1 or more. In the
    /// light theme the card's tint is so faint (5%) that its white, the
    /// lightest a chip can be, differs from it by about 1.06 to 1.
    #[test]
    fn a_selected_cards_tag_chips_stand_out() {
        for (ctx, theme) in styled() {
            let v = ctx.style_of(theme).visuals.clone();
            let (dark, content, weak) = (v.dark_mode, v.extreme_bg_color, v.weak_text_color());
            let chip = content;
            assert_eq!(chip.a(), 255, "{theme:?}: an opaque chip");
            let texts = TAG_COLORS.iter().map(|c| tag_shade(*c, dark)).chain([weak]);
            for text in texts {
                let ratio = contrast(text, chip);
                assert!(ratio >= 4.5, "{theme:?}: text {text:?} {ratio:.2} to 1");
            }
            let least = if dark { 1.1 } else { 1.05 };
            for a in ACCENTS {
                let card = over(selected_card(a, dark), content);
                let apart = contrast(chip, card);
                assert!(
                    apart >= least,
                    "{theme:?}, {a:?}: chip and card {apart:.3} to 1"
                );
            }
        }
    }

    /// A scroll bar's handle at rest stays in sight on every surface it
    /// floats over, fainter than under the pointer, and the bar takes no
    /// room from the content.
    #[test]
    fn a_resting_scroll_handle_stays_in_sight() {
        for (ctx, theme) in styled() {
            let style = ctx.style_of(theme);
            let scroll = style.spacing.scroll;
            assert!(scroll.floating, "{theme:?}");
            assert_eq!(scroll.floating_allocated_width, 0.0, "{theme:?}");
            assert!(scroll.dormant_handle_opacity < scroll.active_handle_opacity);
            // egui draws a floating bar's handle in the color of a
            // button's text.
            let v = &style.visuals;
            let handle = v
                .widgets
                .inactive
                .fg_stroke
                .color
                .gamma_multiply(scroll.dormant_handle_opacity);
            for (place, under) in [("panel", v.panel_fill), ("content", v.extreme_bg_color)] {
                let ratio = contrast(over(handle, under), under);
                assert!(ratio >= 1.8, "{theme:?}: on {place}, {ratio:.2} to 1");
            }
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
            let paused = paused_bar(panel, dark);
            // (what and where, text colour, background)
            let mut cases: Vec<(String, Color32, Color32)> = Vec::new();
            cases.push((
                "a warning's icon on its tint".to_string(),
                text_shade(AMBER, dark),
                over(warning_fill(dark), content),
            ));
            let kinds = [Kind::Text, Kind::Link, Kind::Image, Kind::Files].map(kind_color);
            let tint = if dark { 0.3 } else { 0.13 };
            for c in kinds {
                let text = text_shade(c, dark);
                for (place, background) in [
                    ("chip", over(c.gamma_multiply(0.14), content)),
                    ("card chip", over(c.gamma_multiply(tint), content)),
                ] {
                    cases.push((format!("{c:?} on {place}"), text, background));
                }
            }
            for c in TAG_COLORS {
                let text = tag_shade(c, dark);
                let mut grounds = vec![
                    ("tag".to_string(), over(c.gamma_multiply(tint), content)),
                    (
                        "chosen tag filter".to_string(),
                        over(chosen_pill(c, dark), panel),
                    ),
                    // Also a tag's chip on a selected card, which is the
                    // card's own color.
                    ("tag suggestion".to_string(), content),
                ];
                // A list row's tag at rest, and on a row under the pointer
                // or selected, where its chip is the panel's color.
                grounds.push(("tag on a row".into(), over(c.gamma_multiply(tint), panel)));
                grounds.push(("tag on a tinted row".into(), panel));
                for (place, background) in grounds {
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
                    ("chosen part of a choice", chosen_face(a, dark)),
                    ("chosen filter", over(chosen_pill(a, dark), panel)),
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
                for (place, background) in [
                    ("card", content),
                    ("update banner", banner),
                    ("status bar while paused", paused),
                ] {
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
                        // The state of search by meaning, in its card.
                        (
                            "chip in a settings card",
                            over(s.gamma_multiply(0.14), content),
                        ),
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
                        ("status bar while paused", paused),
                        (
                            "chip on the status bar while paused",
                            over(s.gamma_multiply(0.14), paused),
                        ),
                    ] {
                        cases.push((format!("{s:?} on {place}, accent {a:?}"), text, background));
                    }
                }
                for (place, fill) in [
                    ("main button", button_fill(a)),
                    ("delete button", button_fill(RED)),
                    ("resume recording button", button_fill(AMBER)),
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
                // The pin of a pinned row or card, in dark.
                if dark {
                    let pin = pin_color(a, dark);
                    let hovered = v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6);
                    for (place, background) in [
                        ("row", panel),
                        ("card", content),
                        ("selected row", over(a.gamma_multiply(0.16), panel)),
                        ("selected card", over(selected_card(a, dark), content)),
                        ("row under the pointer", over(hovered, panel)),
                    ] {
                        cases.push((format!("pin {a:?} on {place}"), pin, background));
                    }
                }
            }
            for (case, text, background) in cases {
                let ratio = contrast(text, background);
                assert!(ratio >= 4.5, "{theme:?}: {case} is {ratio:.2} to 1");
            }
        }
    }
}
