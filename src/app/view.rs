//! Main window: search bar, filter chips, grouped list, details pane, status bar.
//!
//! Structure comes from alignment, background tone and type weight rather than
//! borders: the list and details panes differ by fill, rows by tint, groups by
//! small headers on the same left edge as the content.

use std::sync::atomic::AtomicUsize;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Align, Align2, Frame, Layout, Margin, Pos2, Rect, Ui};

use super::icons::{self, icon_button_at, icon_button_ex, view_icon, Icon};
use super::bidi;
use super::shortcut;
use super::*;
use crate::config::Density;

// ----- palette & style ---------------------------------------------------------

pub(super) const ACCENTS: [(&str, Color32); 5] = [
    ("Indigo", Color32::from_rgb(88, 101, 242)),
    ("Sky", Color32::from_rgb(14, 140, 214)),
    ("Emerald", Color32::from_rgb(16, 160, 110)),
    ("Amber", Color32::from_rgb(214, 128, 10)),
    ("Rose", Color32::from_rgb(220, 60, 120)),
];

static ACCENT: AtomicUsize = AtomicUsize::new(0);

pub(super) fn set_accent(i: usize) {
    ACCENT.store(i.min(ACCENTS.len() - 1), Ordering::Relaxed);
}

pub(super) fn accent() -> Color32 {
    ACCENTS[ACCENT.load(Ordering::Relaxed)].1
}

pub(super) const GREEN: Color32 = Color32::from_rgb(34, 160, 90);
pub(super) const AMBER: Color32 = Color32::from_rgb(214, 140, 20);
pub(super) const RED: Color32 = Color32::from_rgb(214, 60, 60);

pub(super) fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Text => Color32::from_rgb(90, 120, 230),
        Kind::Link => Color32::from_rgb(20, 150, 170),
        Kind::Image => Color32::from_rgb(220, 110, 50),
        Kind::Files => Color32::from_rgb(140, 100, 210),
    }
}

/// A stable colour per tag, so "#work" looks the same everywhere.
pub(super) fn tag_color(tag: &str) -> Color32 {
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

fn filter_icon(f: Filter) -> Icon {
    match f {
        Filter::All => Icon::Clipboard,
        Filter::Text => Icon::Text,
        Filter::Links => Icon::Link,
        Filter::Images => Icon::Image,
        Filter::Files => Icon::Folder,
        Filter::Pinned => Icon::PinFilled,
    }
}

pub(super) fn mode_label(m: SearchMode) -> &'static str {
    match m {
        SearchMode::Hybrid => "Smart",
        SearchMode::Semantic => "Meaning",
        SearchMode::Keyword => "Exact words",
    }
}

pub(super) fn mode_help(m: SearchMode) -> &'static str {
    match m {
        SearchMode::Hybrid => "Exact words and meaning, merged (recommended)",
        SearchMode::Semantic => "Only by meaning: \"trip\" finds \"flight to Jeddah\"",
        SearchMode::Keyword => "Only items containing the words you type",
    }
}

/// Adds the bundled Arabic font after egui's own, so Latin text looks the same
/// and Arabic always has glyphs, whatever fonts the PC has.
pub(super) fn add_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};
    let families = [FontFamily::Proportional, FontFamily::Monospace].map(|family| InsertFontFamily { family, priority: FontPriority::Lowest }).to_vec();
    ctx.add_font(FontInsert::new("noto-sans-arabic", FontData::from_static(include_bytes!("../../assets/fonts/NotoSansArabic-Regular.ttf")), families));
}

pub(super) fn style(ctx: &egui::Context) {
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
    ctx.style_mut_of(egui::Theme::Light, |s| s.visuals.text_edit_bg_color = Some(Color32::from_gray(240)));
    ctx.style_mut_of(egui::Theme::Dark, |s| s.visuals.text_edit_bg_color = Some(Color32::from_gray(38)));
}

pub(super) fn apply_theme(ctx: &egui::Context, t: Theme) {
    ctx.set_theme(match t {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
}

pub(super) fn looks_like_code(s: &str) -> bool {
    let sample: String = s.chars().take(2000).collect();
    let symbols = sample.chars().filter(|c| "{}();=<>[]_/\\".contains(*c)).count();
    sample.lines().count() > 1 && symbols * 20 > sample.len().max(1)
}

// ----- time --------------------------------------------------------------------

// Stored times are UTC seconds; `i18n::local` shows each in the user's zone.

/// Whole days between the item's local date and today (0 = today).
fn days_ago(ts: i64) -> i64 {
    (i18n::local(now()).date_naive() - i18n::local(ts).date_naive()).num_days()
}

fn day_month(dt: &chrono::DateTime<chrono::Local>, year: bool) -> String {
    use chrono::Datelike;
    let mut s = format!("{} {}", dt.day(), i18n::month(dt));
    if year {
        s.push_str(&format!(" {}", dt.year()));
    }
    s
}

/// Short time for the edge of a row; the group header already gives the day.
fn row_time(ts: i64) -> String {
    use chrono::Datelike;
    let dt = i18n::local(ts);
    match days_ago(ts) {
        0 | 1 => i18n::clock(&dt),
        2..=6 => i18n::weekday(&dt).to_string(),
        _ => day_month(&dt, dt.year() != i18n::local(now()).year()),
    }
}

/// "just now", "5 minutes ago", "3 hours ago", then the short date: card headers
/// have no day groups to lean on.
fn ago(ts: i64) -> String {
    let secs = (now() - ts).max(0);
    match secs {
        0..=44 => tr("just now").into(),
        45..=119 => tr("1 minute ago").into(),
        120..=3_599 => trf("{n} minutes ago", &[("n", &(secs / 60))]),
        3_600..=7_199 => tr("1 hour ago").into(),
        7_200..=86_399 => trf("{n} hours ago", &[("n", &(secs / 3_600))]),
        _ => match days_ago(ts) {
            1 => tr("yesterday").into(),
            _ => row_time(ts),
        },
    }
}

/// "Today at 09:59", "Yesterday at 6:20 PM", "Mon 14 Oct 2026 at 10:00".
pub(super) fn when(ts: i64) -> String {
    let dt = i18n::local(ts);
    let time = i18n::clock(&dt);
    match days_ago(ts) {
        0 => trf("Today at {time}", &[("time", &time)]),
        1 => trf("Yesterday at {time}", &[("time", &time)]),
        _ => trf("{date} at {time}", &[("date", &format!("{} {}", i18n::weekday(&dt), day_month(&dt, true))), ("time", &time)]),
    }
}

fn group_label(it: &Item) -> &'static str {
    if it.pinned {
        return "Pinned";
    }
    match days_ago(it.created) {
        0 => "Today",
        1 => "Yesterday",
        2..=6 => "This week",
        7..=30 => "This month",
        _ => "Older",
    }
}

// ----- small widgets -------------------------------------------------------------

fn search_id() -> egui::Id {
    egui::Id::unique("search")
}

// ----- language-aware text and layout ---------------------------------------------

/// Text for a label or button in display order. `None` keeps the widget's own colour.
pub(super) fn txt(text: &str, size: f32, color: Option<Color32>) -> egui::WidgetText {
    if bidi::has_rtl(text) {
        bidi::job(text, FontId::proportional(size), color.unwrap_or(Color32::PLACEHOLDER)).into()
    } else {
        let r = RichText::new(text).size(size);
        match color {
            Some(c) => r.color(c).into(),
            None => r.into(),
        }
    }
}

/// `en`, translated, as a label.
pub(super) fn lbl(ui: &mut Ui, en: &str, size: f32, color: Color32) -> egui::Response {
    ui.label(txt(tr(en), size, Some(color)))
}

/// A hover tooltip that wraps Arabic in reading order.
pub(super) fn tip(resp: egui::Response, text: &str) -> egui::Response {
    if bidi::has_rtl(text) {
        let text = text.to_string();
        resp.on_hover_ui(move |ui| {
            ui.set_max_width(340.0);
            let c = ui.visuals().text_color();
            bidi::paragraph(ui, &text, FontId::proportional(13.0), c);
        })
    } else {
        resp.on_hover_text(text)
    }
}

/// Main-axis direction for a row: right to left in Arabic.
pub(super) fn row_layout() -> Layout {
    if i18n::rtl() {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    }
}

/// The opposite direction, for pushing controls to the far end of a row.
pub(super) fn far_layout() -> Layout {
    if i18n::rtl() {
        Layout::left_to_right(Align::Center)
    } else {
        Layout::right_to_left(Align::Center)
    }
}

/// Like `ui.horizontal_wrapped`, in reading direction.
pub(super) fn row_wrapped<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = Vec2::new(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, row_layout().with_main_wrap(true), add).inner
}

/// Like `ui.horizontal`, in reading direction.
pub(super) fn row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = Vec2::new(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, row_layout(), add).inner
}

/// The rest of the row, filled from its far end.
pub(super) fn far<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(far_layout(), add).inner
}

/// A label plus an optional smaller count ("Text  5") for pills; the count
/// trails the label in reading order.
pub(super) fn pill_job(text: &str, size: f32, color: Color32, count: Option<(String, Color32)>) -> LayoutJob {
    let mut job = if bidi::has_rtl(text) {
        bidi::job(text, FontId::proportional(size), color)
    } else {
        let mut j = LayoutJob::default();
        j.append(text, 0.0, TextFormat { font_id: FontId::proportional(size), color, ..Default::default() });
        j
    };
    job.halign = egui::Align::LEFT;
    if let Some((n, c)) = count {
        let fmt = TextFormat { font_id: FontId::proportional(size - 1.5), color: c, ..Default::default() };
        if i18n::rtl() {
            let mut j = LayoutJob::default();
            j.append(&format!("{n}  "), 0.0, fmt);
            for s in job.sections.iter() {
                j.append(&job.text[s.byte_range.start.0..s.byte_range.end.0], 0.0, s.format.clone());
            }
            job = j;
        } else {
            job.append(&format!("  {n}"), 0.0, fmt);
        }
    }
    job
}

/// Paints one line of text anchored at `pos`, in display order. Returns its rect.
pub(super) fn paint_text(ui: &Ui, p: &egui::Painter, pos: Pos2, anchor: Align2, text: &str, font: FontId, color: Color32) -> Rect {
    let g = if bidi::has_rtl(text) {
        ui.fonts_mut(|f| f.layout_job(bidi::job(text, font, color)))
    } else {
        ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, color))
    };
    let rect = anchor.anchor_size(pos, g.size());
    p.galley(rect.min, g, color);
    rect
}

/// The anchor on the reading-start side: left, or right in Arabic.
pub(super) fn start_anchor(vertical: egui::Align) -> Align2 {
    let h = if i18n::rtl() { egui::Align::Max } else { egui::Align::Min };
    Align2([h, vertical])
}

/// Mirrors `r` inside `within` for right-to-left layout.
pub(super) fn flip(within: Rect, r: Rect) -> Rect {
    if i18n::rtl() {
        Rect::from_min_max(Pos2::new(within.left() + within.right() - r.right(), r.top()), Pos2::new(within.left() + within.right() - r.left(), r.bottom()))
    } else {
        r
    }
}

/// "Pin (Ctrl+P)", or just "Pin" when the shortcut is off.
fn hint(what: &str, keys: &str) -> String {
    if keys.trim().is_empty() {
        tr(what).to_string()
    } else {
        format!("{} ({keys})", tr(what))
    }
}

/// A quiet text button with a painted ▾, for menus.
fn mode_button(ui: &mut Ui, label: &str, emphasised: bool) -> egui::Response {
    let v = ui.visuals();
    let color = if emphasised { accent() } else { v.weak_text_color() };
    let g = ui.fonts_mut(|f| f.layout_job(bidi::job(label, FontId::proportional(13.0), color)));
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 8.0 + 10.0 + 18.0, 28.0), Sense::click());
    let v = ui.visuals();
    if emphasised {
        ui.painter().rect_filled(rect, CornerRadius::same(8), accent().gamma_multiply(0.12));
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.weak_bg_fill);
    }
    let color = if resp.hovered() && !emphasised { v.text_color() } else { color };
    let text_at = flip(rect, Rect::from_min_size(Pos2::new(rect.left() + 9.0, rect.center().y - g.size().y / 2.0), g.size())).min;
    ui.painter().galley(text_at, g, color);
    icons::paint(ui.painter(), flip(rect, Rect::from_center_size(Pos2::new(rect.right() - 14.0, rect.center().y), Vec2::splat(10.0))), Icon::Dropdown, color);
    resp
}

/// Rounded tinted label: kind, state or status.
pub(super) fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    let text = text.into();
    let text = tr(&text).to_string();
    if let Some((icon, rest)) = icons::from_prefix(&text) {
        return icons::chip(ui, icon, rest, color);
    }
    Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| ui.label(txt(&text, 12.0, Some(color))))
        .response
}

/// Wide enough for the longest help line, no wider.
const MODE_MENU_W: f32 = 300.0;

/// One search-mode choice: name and help in a single full-width target, the
/// active one tinted, edged and ticked.
fn mode_option(ui: &mut Ui, m: SearchMode, active: bool) -> egui::Response {
    let w = MODE_MENU_W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, active, tr(mode_label(m))));
    let v = ui.visuals();
    let p = ui.painter();
    let a = accent();
    let r = CornerRadius::same(8);
    if active {
        p.rect_filled(rect, r, a.gamma_multiply(if v.dark_mode { 0.22 } else { 0.10 }));
        p.rect_filled(flip(rect, Rect::from_min_size(rect.min + Vec2::new(0.0, 10.0), Vec2::new(3.0, rect.height() - 20.0))), CornerRadius::same(2), a);
    }
    if resp.hovered() {
        p.rect_filled(rect, r, if active { a.gamma_multiply(0.06) } else { v.widgets.hovered.weak_bg_fill });
    }
    let title = if active { a } else { v.strong_text_color() };
    let at = |y: f32| flip(rect, Rect::from_min_size(Pos2::new(rect.left() + 14.0, rect.top() + y), Vec2::ZERO)).min;
    paint_text(ui, p, at(8.0), start_anchor(egui::Align::Min), tr(mode_label(m)), FontId::proportional(14.0), title);
    paint_text(ui, p, at(27.0), start_anchor(egui::Align::Min), tr(mode_help(m)), FontId::proportional(11.5), v.weak_text_color());
    if active {
        // Level with the title, clear of the longer help line.
        icons::paint(p, flip(rect, Rect::from_center_size(Pos2::new(rect.right() - 18.0, rect.top() + 16.0), Vec2::splat(14.0))), Icon::Check, a);
    }
    resp
}

pub(super) fn primary_button(ui: &mut Ui, en: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding.x = 16.0;
        ui.add(egui::Button::new(txt(tr(en), 14.0, Some(Color32::WHITE))).fill(accent()).min_size(Vec2::new(0.0, 32.0)))
    })
    .inner
}

/// `text` with every occurrence of the query terms highlighted, cut to `max_rows`.
fn highlighted(text: &str, terms: &[String], font: FontId, color: Color32, max_width: f32, max_rows: usize) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat { font_id: font.clone(), color, ..Default::default() };
    let hl = TextFormat { font_id: font, color, background: accent().gamma_multiply(0.28), ..Default::default() };
    let lower = text.to_lowercase();
    // Lower-casing can change byte offsets outside ASCII; skip highlighting then.
    let mut marks: Vec<(usize, usize)> = Vec::new();
    if lower.len() == text.len() {
        for t in terms.iter().filter(|t| !t.is_empty()) {
            let mut from = 0;
            while let Some(i) = lower[from..].find(t.as_str()) {
                marks.push((from + i, from + i + t.len()));
                from += i + t.len();
            }
        }
    }
    marks.sort();
    let mut pos = 0;
    for (a, b) in marks {
        if a < pos || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        job.append(&text[pos..a], 0.0, plain.clone());
        job.append(&text[a..b], 0.0, hl.clone());
        pos = b;
    }
    job.append(&text[pos..], 0.0, plain);
    job.wrap = TextWrapping { max_width, max_rows, break_anywhere: max_rows == 1, overflow_character: Some('…') };
    job
}

/// What a row's or card's hover buttons asked for.
#[derive(Clone, Copy)]
enum ItemAction {
    Select(u64),
    Copy(u64),
    Delete(u64),
    Pin(u64),
    Open(u64),
}

/// Domain of a URL, e.g. `huggingface.co`.
fn domain(url: &str) -> &str {
    let rest = url.trim().split("://").nth(1).unwrap_or(url.trim());
    rest.split(['/', '?', '#']).next().unwrap_or(rest).trim_start_matches("www.")
}

/// Second line of an item row: the most useful detail for its kind.
fn subtitle(it: &Item) -> String {
    let detail = subtitle_detail(it);
    if it.tags.is_empty() {
        return detail;
    }
    let tags = it.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ");
    if detail.is_empty() {
        tags
    } else {
        format!("{tags}  ·  {detail}")
    }
}

fn subtitle_detail(it: &Item) -> String {
    match it.kind {
        Kind::Link => domain(&it.text).to_string(),
        Kind::Image => {
            let mut s = format!("{} × {}", it.width, it.height);
            if let Some(o) = it.ocr.as_deref().filter(|o| !o.trim().is_empty()) {
                let first: String = o.split_whitespace().take(8).collect::<Vec<_>>().join(" ");
                s = format!("{s}  ·  {first}");
            }
            s
        }
        Kind::Files => {
            let n = it.text.lines().count();
            if n == 1 {
                std::path::Path::new(it.text.trim()).parent().map(|p| p.display().to_string()).unwrap_or_default()
            } else {
                trf("{n} files", &[("n", &n)])
            }
        }
        Kind::Text => {
            if let Some(src) = &it.source {
                return src.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            }
            let mut lines = it.text.lines().map(str::trim).filter(|l| !l.is_empty());
            lines.next();
            lines.next().map(|l| l.chars().take(140).collect()).unwrap_or_default()
        }
    }
}

fn row_title(it: &Item) -> String {
    match it.kind {
        Kind::Link => {
            let t = it.text.trim();
            t.split("://").nth(1).unwrap_or(t).trim_start_matches("www.").to_string()
        }
        Kind::Files => {
            let first = it.text.lines().next().unwrap_or("");
            std::path::Path::new(first).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| first.to_string())
        }
        _ => it.title(),
    }
}

enum Line {
    Header(&'static str, usize),
    Row(usize),
}

// ----- the window ------------------------------------------------------------------

impl App {
    fn keyboard(&mut self, ctx: &egui::Context) {
        if self.draft.is_some() {
            // While a shortcut field listens, Esc is its "cancel".
            if self.recording.is_none() && ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.close_settings(ctx);
            }
            return;
        }
        // Typing in a field other than search (a tag, say) isn't a command.
        // The tag field counts even before egui has moved focus to it.
        let typing = self.tag_input.is_some() || ctx.memory(|m| m.focused()).is_some_and(|id| id != search_id());
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        let (down, up, left, right) = if typing {
            (false, false, false, false)
        } else {
            ctx.input(|i| (i.key_pressed(Key::ArrowDown), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight)))
        };
        let k = self.cfg.keys.clone();
        let on = |s: &str| !typing && shortcut::pressed(ctx, s);
        let (enter, del, pin, tag, view) = (on(&k.copy), on(&k.delete), on(&k.pin), on(&k.tag), on(&k.view));
        if view {
            self.set_view(self.cfg.view.next());
        }
        // In the grid ↑ ↓ move a whole row; ← → move one card, unless they
        // would be moving the cursor in a typed query.
        let grid = self.cfg.view == ViewMode::Grid;
        let step = if grid { self.grid_cols.max(1) as i32 } else { 1 };
        if down {
            self.move_selection(step);
        }
        if up {
            self.move_selection(-step);
        }
        if grid && self.query.is_empty() {
            if right {
                self.move_selection(1);
            }
            if left {
                self.move_selection(-1);
            }
        }
        if let Some(id) = self.selected {
            if enter {
                self.copy(id);
            }
            if del {
                self.delete(id);
            }
            if pin {
                self.toggle_pin(id);
            }
            if tag {
                self.tag_input = Some((id, String::new()));
                self.tag_focus = true;
                self.details_open = true;
            }
        }
        if esc {
            if self.tag_input.is_some() {
                self.tag_input = None;
            } else if self.details_open && self.cfg.view != ViewMode::Detail {
                self.details_open = false;
            } else if self.query.is_empty() {
                self.win.hide();
            } else {
                self.query.clear();
                self.query_vec = None;
                self.dirty = true;
            }
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        egui::Panel::top("top")
            .show_separator_line(false)
            .frame(Frame::new().inner_margin(Margin { left: 16, right: 12, top: 12, bottom: 6 }).fill(fill))
            .show(ui, |ui| {
                row(ui, |ui| {
                    let actions_w = 4.0 * 40.0;
                    let w = (ui.available_width() - actions_w).max(200.0);
                    self.search_field(ui, w);
                    far(ui, |ui| {
                        if icons::icon_button(ui, Icon::Settings, "Settings").clicked() {
                            self.open_settings(SettingsPage::General);
                        }
                        let import = icons::icon_button(ui, Icon::Add, "Add files or a folder to search (you can also drop them on the window)");
                        egui::Popup::menu(&import).show(|ui| {
                            if icons::menu_item(ui, Icon::File, "Add files…").clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_files() {
                                    self.import(f);
                                }
                            }
                            if icons::menu_item(ui, Icon::Folder, "Add a folder…").clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_folder() {
                                    self.import(vec![f]);
                                }
                            }
                        });
                        let view = self.cfg.view;
                        let tip = trf("View: {view} · click for {next}", &[("view", &tr(view.label())), ("next", &tr(view.next().label()))]);
                        if icons::icon_button(ui, view_icon(view), &tip).clicked() {
                            self.set_view(view.next());
                        }
                        let rec = self.flags.enabled.load(Ordering::SeqCst);
                        let (icon, tip) = if rec { (Icon::Pause, "Pause recording") } else { (Icon::Play, "Resume recording") };
                        let (fill, fg) = if rec { (None, None) } else { (Some(AMBER), Some(Color32::WHITE)) };
                        if icon_button_ex(ui, icon, tip, Vec2::splat(34.0), 16.0, fill, fg).clicked() {
                            self.set_recording(!rec);
                        }
                    });
                });
                ui.add_space(4.0);
                self.filter_chips(ui);
            });
    }

    fn search_field(&mut self, ui: &mut Ui, width: f32) {
        let focused_before = ui.memory(|m| m.focused()).is_some_and(|id| id == search_id());
        let bg = if ui.visuals().dark_mode { Color32::from_gray(22) } else { Color32::WHITE };
        let out = Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(10))
            .stroke(if focused_before { Stroke::new(1.5, accent()) } else { Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color.gamma_multiply(0.0)) })
            .inner_margin(if i18n::rtl() { Margin { left: 6, right: 12, top: 0, bottom: 0 } } else { Margin { left: 12, right: 6, top: 0, bottom: 0 } })
            .show(ui, |ui| {
                ui.set_width(width - 18.0);
                ui.set_height(40.0);
                ui.with_layout(row_layout(), |ui| {
                    icons::glyph(ui, Icon::Search, 16.0, ui.visuals().weak_text_color());
                    let hint = match &self.backend {
                        BackendState::Ready { images: true, .. } => "Search text and images by meaning…",
                        BackendState::Ready { .. } => "Search by words or meaning…",
                        _ => "Search your clipboard…",
                    };
                    // Arabic is shown in reading order (the cursor still follows typing order).
                    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let color = ui.visuals().text_color();
                        let job = if bidi::has_rtl(text.as_str()) {
                            bidi::job(text.as_str(), FontId::proportional(16.0), color)
                        } else {
                            LayoutJob::simple_singleline(text.as_str().to_string(), FontId::proportional(16.0), color)
                        };
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let te = egui::TextEdit::singleline(&mut self.query)
                        .id(search_id())
                        .frame(Frame::NONE)
                        .hint_text(txt(tr(hint), 16.0, Some(ui.visuals().weak_text_color())))
                        .font(FontId::proportional(16.0))
                        .align(if i18n::rtl() { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER })
                        .layouter(&mut layouter)
                        .desired_width((ui.available_width() - 130.0).max(80.0));
                    let resp = ui.add(te);
                    if self.focus_search.swap(false, Ordering::SeqCst) {
                        resp.request_focus();
                    }
                    if resp.changed() {
                        self.query_changed = Some(Instant::now());
                        self.dirty = true;
                    }
                    far(ui, |ui| {
                        // Search mode, quiet until needed.
                        let mode = self.cfg.search_mode;
                        let non_default = mode != SearchMode::Hybrid;
                        let b = mode_button(ui, tr(mode_label(mode)), non_default);
                        egui::Popup::menu(&b).show(|ui| {
                            ui.set_width(MODE_MENU_W);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for m in [SearchMode::Hybrid, SearchMode::Semantic, SearchMode::Keyword] {
                                if mode_option(ui, m, mode == m).clicked() {
                                    self.cfg.search_mode = m;
                                    self.save_config();
                                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                                    self.dirty = true;
                                }
                            }
                        });
                        if !self.query.is_empty() && icon_button_ex(ui, Icon::Close, "Clear (Esc)", Vec2::splat(26.0), 11.0, None, None).clicked() {
                            self.query.clear();
                            self.query_vec = None;
                            self.dirty = true;
                        }
                    });
                });
            });
        let _ = out;
    }

    fn filter_chips(&mut self, ui: &mut Ui) {
        row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for f in Filter::ALL {
                let n = self.items.iter().filter(|i| f.accepts(i)).count();
                let selected = self.filter == f;
                // Empty categories add noise; keep them out unless selected.
                if n == 0 && f != Filter::All && !selected {
                    continue;
                }
                let (fg, weak) = if selected { (accent(), accent().gamma_multiply(0.75)) } else { (ui.visuals().text_color(), ui.visuals().weak_text_color()) };
                let job = pill_job(tr(f.label()), 13.5, fg, Some((n.to_string(), weak)));
                if icons::pill(ui, filter_icon(f), job, accent(), selected).clicked() {
                    self.filter = f;
                    self.dirty = true;
                }
            }
            self.tag_chips(ui);
            if !self.query.trim().is_empty() {
                far(ui, |ui| {
                    ui.add_space(4.0);
                    let n = self.results.len();
                    let text = if n == 1 { tr("1 result").to_string() } else { trf("{n} results", &[("n", &n)]) };
                    ui.label(txt(&text, 12.5, Some(ui.visuals().weak_text_color())));
                });
            }
        });
    }

    /// The most used tags as filters after the kind chips; the rest in a menu.
    fn tag_chips(&mut self, ui: &mut Ui) {
        let tags = self.all_tags();
        if tags.is_empty() {
            return;
        }
        ui.add_space(6.0);
        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, ui.visuals().weak_text_color().gamma_multiply(0.3));
        ui.add_space(6.0);
        const SHOWN: usize = 6;
        let mut clicked: Option<String> = None;
        let mut delete: Option<String> = None;
        let active = self.tag_filter.clone();
        let is_active = |t: &str| active.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(t));
        // The active tag always shows, even when it isn't among the most used.
        let mut shown: Vec<&(String, usize)> = tags.iter().take(SHOWN).collect();
        if let Some(a) = tags.iter().skip(SHOWN).find(|(t, _)| is_active(t)) {
            shown.push(a);
        }
        for (t, n) in &shown {
            let sel = is_active(t);
            let c = tag_color(t);
            let job = pill_job(t, 13.0, if sel { c } else { ui.visuals().text_color() }, Some((n.to_string(), if sel { c.gamma_multiply(0.75) } else { ui.visuals().weak_text_color() })));
            let r = tip(icons::pill(ui, Icon::Tag, job, c, sel), tr(if sel { "Show all tags again" } else { "Only items with this tag · right-click for more" }));
            if r.clicked() {
                clicked = Some(t.clone());
            }
            r.context_menu(|ui| {
                if ui.button(txt(&trf("Remove #{tag} from all {n} items", &[("tag", t), ("n", n)]), 14.0, None)).clicked() {
                    delete = Some(t.clone());
                }
            });
        }
        let rest: Vec<&(String, usize)> = tags.iter().skip(SHOWN).filter(|(t, _)| !is_active(t)).collect();
        if !rest.is_empty() {
            let more = mode_button(ui, &trf("+{n} tags", &[("n", &rest.len())]), false);
            egui::Popup::menu(&more).show(|ui| {
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for (t, n) in rest {
                        if icons::menu_item(ui, Icon::Tag, &format!("{t}   {n}")).clicked() {
                            clicked = Some(t.clone());
                        }
                    }
                });
            });
        }
        if let Some(t) = clicked {
            self.tag_filter = if is_active(&t) { None } else { Some(t) };
            self.dirty = true;
        }
        if let Some(t) = delete {
            self.delete_tag(&t);
        }
    }

    /// Tag pills with remove buttons, then an input for adding more.
    fn tag_editor(&mut self, ui: &mut Ui, it: &Item) {
        let mut remove: Option<String> = None;
        let mut add: Option<String> = None;
        let mut filter: Option<String> = None;
        let mut close_input = false;
        row_wrapped(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for t in &it.tags {
                let c = tag_color(t);
                Frame::new().fill(c.gamma_multiply(if ui.visuals().dark_mode { 0.25 } else { 0.13 })).corner_radius(CornerRadius::same(12)).inner_margin(Margin { left: 10, right: 4, top: 2, bottom: 2 }).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if tip(ui.add(egui::Label::new(txt(&format!("# {t}"), 12.5, Some(c))).sense(Sense::click())), tr("Show only items with this tag")).clicked() {
                        filter = Some(t.clone());
                    }
                    if icon_button_ex(ui, Icon::Close, "Remove tag", Vec2::splat(18.0), 8.0, None, Some(c)).clicked() {
                        remove = Some(t.clone());
                    }
                });
            }
            let editing = self.tag_input.as_ref().is_some_and(|(id, _)| *id == it.id);
            if editing {
                let known = self.all_tags();
                let id = egui::Id::unique(("tag-input", it.id));
                // Read Enter while the field still has focus: egui reports the
                // lost focus a frame later, when the key is no longer "pressed".
                let enter = ui.memory(|m| m.has_focus(id)) && ui.input(|i| i.key_pressed(Key::Enter));
                let (_, text) = self.tag_input.as_mut().unwrap();
                let r = ui.add(egui::TextEdit::singleline(text).id(id).hint_text(txt(tr("tag, another…"), 13.0, None)).desired_width(150.0).font(FontId::proportional(13.0)));
                if std::mem::take(&mut self.tag_focus) || enter {
                    r.request_focus();
                }
                // Existing tags that start with what's typed, one click to add.
                let typed = text.trim().trim_start_matches('#').to_lowercase();
                if enter {
                    // Enter adds and keeps the field open for the next tag.
                    add = Some(std::mem::take(text));
                } else if r.lost_focus() {
                    // Clicking away keeps what was typed and closes the field.
                    add = Some(std::mem::take(text));
                    close_input = true;
                }
                let mut suggestions = known.iter().filter(|(k, _)| !it.has_tag(k) && (typed.is_empty() || k.to_lowercase().starts_with(&typed))).take(5).peekable();
                if suggestions.peek().is_some() {
                    for (k, _) in suggestions {
                        if ui.add(egui::Button::new(txt(&format!("+ {k}"), 12.0, Some(tag_color(k)))).frame_when_inactive(false)).clicked() {
                            add = Some(k.clone());
                        }
                    }
                }
            } else {
                let job = pill_job(tr(if it.tags.is_empty() { "Add tag" } else { "Tag" }), 12.5, ui.visuals().weak_text_color(), None);
                if tip(icons::pill(ui, Icon::Add, job, accent(), false), &hint("Add a tag", &self.cfg.keys.tag)).clicked() {
                    self.tag_input = Some((it.id, String::new()));
                    self.tag_focus = true;
                }
            }
        });
        if close_input {
            self.tag_input = None;
        }
        if let Some(t) = remove {
            self.remove_tag(it.id, &t);
        }
        if let Some(t) = add {
            self.add_tags(it.id, &t);
        }
        if let Some(t) = filter {
            self.tag_filter = Some(t);
            self.dirty = true;
        }
    }

    pub(super) fn model_status(&self) -> (Color32, String) {
        match (&self.backend, &self.cfg.backend) {
            (BackendState::Loading, _) => (AMBER, tr("Loading search model…").into()),
            (BackendState::Ready { .. }, EmbedBackend::SigLip { repo }) => (GREEN, trf("Built-in · {model}", &[("model", &repo.rsplit('/').next().unwrap_or(repo))])),
            (BackendState::Ready { .. }, EmbedBackend::Server(s)) => {
                let name = if !s.model.trim().is_empty() {
                    s.model.trim().to_string()
                } else {
                    self.model_list.found.default.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| trf("{server} (default model)", &[("server", &domain(&s.base_url))]))
                };
                (GREEN, name)
            }
            (BackendState::Off, EmbedBackend::None) => (Color32::GRAY, tr("Words only · set up meaning search").into()),
            (BackendState::Off, _) => (RED, tr("Search model unavailable · fix in Settings").into()),
            (BackendState::Ready { .. }, EmbedBackend::None) => (Color32::GRAY, tr("Words only").into()),
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(Frame::new().inner_margin(Margin::symmetric(16, 6)).fill(fill))
            .show(ui, |ui| {
                row(ui, |ui| {
                    let weak = ui.visuals().weak_text_color();
                    let (dot, text) = self.model_status();
                    let color = if dot == Color32::GRAY { weak } else { dot };
                    let job = pill_job(&text, 12.0, color, None);
                    let r = tip(icons::pill(ui, Icon::Dot, job, color, false), tr("Search model: click to change"));
                    if r.clicked() {
                        self.open_settings(SettingsPage::Search);
                    }
                    if self.queue > 0 {
                        ui.add(egui::Spinner::new().size(11.0).color(weak));
                        ui.label(txt(&trf("Indexing · {n} left", &[("n", &self.queue)]), 12.0, Some(weak)));
                    }
                    if !self.status.is_empty() {
                        ui.label(txt(tr(&self.status), 12.0, Some(weak)));
                    }
                    if !self.flags.enabled.load(Ordering::SeqCst) {
                        chip(ui, "⏸ Recording paused", AMBER);
                    }
                    if let Some((e, _)) = &self.error {
                        icons::glyph(ui, Icon::Warning, 13.0, RED);
                        let r = ui.add(egui::Label::new(RichText::new(e.as_str()).size(12.0).color(RED)).truncate().sense(Sense::click()));
                        if tip(r, tr("Click to dismiss")).clicked() {
                            self.error = None;
                        }
                    }
                    far(ui, |ui| {
                        let pinned = self.items.iter().filter(|i| i.pinned).count();
                        let n = self.items.len();
                        let s = if pinned > 0 { trf("{n} items · {p} pinned", &[("n", &n), ("p", &pinned)]) } else { trf("{n} items", &[("n", &n)]) };
                        ui.label(txt(&s, 12.0, Some(weak)));
                    });
                });
            });
    }

    /// Everything below the top bar: list, grid or list + details, per the view mode.
    fn main_area(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        // Panels swap sides right to left: the list starts on the reading side.
        let (start, end) = if i18n::rtl() { (egui::Panel::right("list"), egui::Panel::left("details")) } else { (egui::Panel::left("list"), egui::Panel::right("details")) };
        if self.cfg.view == ViewMode::Detail {
            start
                .resizable(true)
                .show_separator_line(false)
                .default_size(ui.available_width() * 0.42)
                .size_range(300.0..=900.0)
                .frame(Frame::new().inner_margin(Margin { left: 8, right: 8, top: 4, bottom: 4 }).fill(fill))
                .show(ui, |ui| self.list(ui));
            let detail_fill = ui.visuals().extreme_bg_color;
            egui::CentralPanel::default()
                .frame(Frame::new().inner_margin(Margin { left: 24, right: 20, top: 14, bottom: 14 }).fill(detail_fill))
                .show(ui, |ui| self.details(ui, false));
            return;
        }
        if self.details_open && self.selected.is_some() {
            let detail_fill = ui.visuals().extreme_bg_color;
            end
                .resizable(true)
                .show_separator_line(false)
                .default_size(ui.available_width() * 0.45)
                .size_range(320.0..=1100.0)
                .frame(Frame::new().inner_margin(Margin { left: 20, right: 20, top: 14, bottom: 14 }).fill(detail_fill))
                .show(ui, |ui| self.details(ui, true));
        }
        let margin = if self.cfg.view == ViewMode::Grid { Margin { left: 16, right: 12, top: 8, bottom: 4 } } else { Margin { left: 8, right: 8, top: 4, bottom: 4 } };
        egui::CentralPanel::default().frame(Frame::new().inner_margin(margin).fill(fill)).show(ui, |ui| {
            if self.cfg.view == ViewMode::Grid {
                self.grid(ui)
            } else {
                self.list(ui)
            }
        });
    }

    pub(super) fn set_view(&mut self, v: ViewMode) {
        self.cfg.view = v;
        self.details_open = false;
        self.scroll_to_selected = true;
        self.save_config();
    }

    fn apply_actions(&mut self, actions: Vec<ItemAction>) {
        for a in actions {
            match a {
                ItemAction::Select(id) => self.selected = Some(id),
                ItemAction::Copy(id) => {
                    self.selected = Some(id);
                    self.copy(id);
                }
                ItemAction::Delete(id) => self.delete(id),
                ItemAction::Pin(id) => self.toggle_pin(id),
                ItemAction::Open(id) => {
                    self.selected = Some(id);
                    self.details_open = true;
                }
            }
        }
        if self.thumbs.len() > MAX_THUMBS {
            for id in self.thumbs.drain().map(|(id, _)| id).collect::<Vec<_>>() {
                self.loader.forget(id, false);
            }
        }
    }

    fn list(&mut self, ui: &mut Ui) {
        if self.results.is_empty() {
            self.empty_list(ui);
            return;
        }
        let compact = self.cfg.density == Density::Compact;
        let row_h = if compact { 40.0 } else { 58.0 };
        let header_h = 30.0;
        let searching = !self.query.trim().is_empty();
        // The details pane is always beside the list in Detail view, so rows
        // there don't need an "open" button.
        let can_open = self.cfg.view != ViewMode::Detail;

        // Rows, with date/pinned headers when browsing.
        let mut lines: Vec<Line> = Vec::with_capacity(self.results.len() + 8);
        if !searching && self.cfg.group_by_date {
            let mut last: Option<&'static str> = None;
            for (r, h) in self.results.iter().enumerate() {
                let g = group_label(&self.items[h.idx]);
                if last != Some(g) {
                    lines.push(Line::Header(g, 0));
                    last = Some(g);
                }
                if let Some(Line::Header(_, n)) = lines.iter_mut().rev().find(|l| matches!(l, Line::Header(..))) {
                    *n += 1;
                }
                lines.push(Line::Row(r));
            }
        } else {
            lines.extend((0..self.results.len()).map(Line::Row));
        }
        let mut ys = Vec::with_capacity(lines.len());
        let mut y = 0.0;
        for l in &lines {
            ys.push(y);
            y += match l {
                Line::Header(..) => header_h,
                Line::Row(_) => row_h + 2.0,
            };
        }
        let total = y;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
            if let Some(i) = lines.iter().position(|l| matches!(l, Line::Row(r) if Some(self.items[self.results[*r].idx].id) == self.selected)) {
                let (off, view_h) = self.list_view;
                let top = ys[i] - if i > 0 && matches!(lines[i - 1], Line::Header(..)) { header_h } else { 0.0 };
                if top < off {
                    sa = sa.vertical_scroll_offset(top);
                } else if ys[i] + row_h > off + view_h {
                    sa = sa.vertical_scroll_offset(ys[i] + row_h - view_h);
                }
            }
        }

        let max_sem = self.results.iter().filter_map(|h| h.semantic).fold(f32::MIN, f32::max);
        let terms: Vec<String> = if searching { self.query.to_lowercase().split_whitespace().map(String::from).collect() } else { vec![] };
        let mut actions = Vec::new();
        let (items, results, thumbs, loader, selected) = (&self.items, &self.results, &self.thumbs, &mut self.loader, self.selected);
        let out = sa.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            for (i, l) in lines.iter().enumerate() {
                let h = if matches!(l, Line::Header(..)) { header_h } else { row_h };
                if ys[i] + h < viewport.min.y || ys[i] > viewport.max.y {
                    continue;
                }
                let rect = Rect::from_min_size(Pos2::new(origin.x, origin.y + ys[i]), Vec2::new(width, h));
                match l {
                    Line::Header(label, n) => {
                        let p = ui.painter();
                        let weak = ui.visuals().weak_text_color();
                        let (label_x, count_x) = if i18n::rtl() { (rect.right() - 12.0, rect.left() + 12.0) } else { (rect.left() + 12.0, rect.right() - 12.0) };
                        paint_text(ui, p, Pos2::new(label_x, rect.bottom() - 7.0), start_anchor(egui::Align::Max), &tr(label).to_uppercase(), FontId::proportional(11.5), weak);
                        let end = if i18n::rtl() { Align2::LEFT_BOTTOM } else { Align2::RIGHT_BOTTOM };
                        p.text(Pos2::new(count_x, rect.bottom() - 7.0), end, n.to_string(), FontId::proportional(11.5), weak.gamma_multiply(0.7));
                    }
                    Line::Row(r) => {
                        let hit = &results[*r];
                        let it = &items[hit.idx];
                        let thumb = thumb_for(it, thumbs, loader);
                        let resp = ui.interact(rect, egui::Id::unique(("row", it.id)), Sense::click());
                        let hot = ui.rect_contains_pointer(rect);
                        paint_row(ui, rect, hot, it, hit, thumb, Some(it.id) == selected, compact, &terms, max_sem);
                        if resp.clicked() {
                            actions.push(ItemAction::Select(it.id));
                        }
                        if resp.double_clicked() {
                            actions.push(ItemAction::Copy(it.id));
                        }
                        if hot {
                            // Hover actions replace the time at the right edge.
                            let pin = if it.pinned { (Icon::PinFilled, "Unpin") } else { (Icon::Pin, "Pin") };
                            let mut buttons = vec![
                                (Icon::Delete, "Delete", ItemAction::Delete(it.id)),
                                (pin.0, pin.1, ItemAction::Pin(it.id)),
                                (Icon::Copy, "Copy", ItemAction::Copy(it.id)),
                            ];
                            if can_open {
                                buttons.push((Icon::Open, "Open details", ItemAction::Open(it.id)));
                            }
                            let size = 28.0;
                            let mut x = rect.right() - 8.0 - buttons.len() as f32 * (size + 2.0);
                            for (icon, tip, action) in buttons {
                                let b = flip(rect, Rect::from_min_size(Pos2::new(x, rect.center().y - size / 2.0), Vec2::splat(size)));
                                if icon_button_at(ui, b, egui::Id::unique(("row-action", it.id, tip)), icon, tip).clicked() {
                                    actions.push(action);
                                }
                                x += size + 2.0;
                            }
                        }
                    }
                }
            }
        });
        self.list_view = (out.state.clamped_offset().y, out.inner_rect.height());
        self.apply_actions(actions);
    }

    /// Cards in as many columns as fit, like a pinboard.
    fn grid(&mut self, ui: &mut Ui) {
        if self.results.is_empty() {
            self.empty_list(ui);
            return;
        }
        let compact = self.cfg.density == Density::Compact;
        let gap = 14.0;
        let card_h = if compact { 150.0 } else { 210.0 };
        // Leave room for the scroll bar so cards don't jump when it appears.
        let width = ui.available_width() - 12.0;
        let cols = ((width + gap) / (240.0 + gap)).floor().max(1.0) as usize;
        let card_w = (width - gap * (cols as f32 - 1.0)) / cols as f32;
        self.grid_cols = cols;
        let rows = self.results.len().div_ceil(cols);
        let pitch = card_h + gap;
        let total = rows as f32 * pitch;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
            if let Some(i) = self.selected.and_then(|id| self.results.iter().position(|h| self.items[h.idx].id == id)) {
                let (off, view_h) = self.list_view;
                let top = (i / cols) as f32 * pitch;
                if top < off {
                    sa = sa.vertical_scroll_offset(top);
                } else if top + card_h > off + view_h {
                    sa = sa.vertical_scroll_offset(top + card_h + gap - view_h);
                }
            }
        }

        let searching = !self.query.trim().is_empty();
        let terms: Vec<String> = if searching { self.query.to_lowercase().split_whitespace().map(String::from).collect() } else { vec![] };
        let mut actions = Vec::new();
        let (items, results, thumbs, loader, selected) = (&self.items, &self.results, &self.thumbs, &mut self.loader, self.selected);
        let out = sa.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let first = ((viewport.min.y / pitch).floor().max(0.0) as usize) * cols;
            let last = (((viewport.max.y / pitch).ceil() as usize + 1) * cols).min(results.len());
            for (r, hit) in results.iter().enumerate().take(last).skip(first) {
                let (row, col) = (r / cols, r % cols);
                // Cards fill from the reading-start side.
                let col = if i18n::rtl() { cols - 1 - col } else { col };
                let rect = Rect::from_min_size(Pos2::new(origin.x + col as f32 * (card_w + gap), origin.y + row as f32 * pitch + 2.0), Vec2::new(card_w, card_h));
                let it = &items[hit.idx];
                let thumb = thumb_for(it, thumbs, loader);
                let resp = ui.interact(rect, egui::Id::unique(("card", it.id)), Sense::click());
                let hot = ui.rect_contains_pointer(rect);
                paint_card(ui, rect, hot, it, thumb, Some(it.id) == selected, &terms);
                if resp.clicked() {
                    actions.push(ItemAction::Select(it.id));
                }
                if resp.double_clicked() {
                    actions.push(ItemAction::Copy(it.id));
                }
                if hot {
                    let size = 30.0;
                    let y = rect.bottom() - CARD_FOOTER / 2.0 - size / 2.0;
                    let at = |x: f32| flip(rect, Rect::from_min_size(Pos2::new(x, y), Vec2::splat(size)));
                    let pin = if it.pinned { (Icon::PinFilled, "Unpin") } else { (Icon::Pin, "Pin") };
                    for (b, icon, tip, action) in [
                        (at(rect.left() + 8.0), Icon::Delete, "Delete", ItemAction::Delete(it.id)),
                        (at(rect.right() - 8.0 - 3.0 * size - 8.0), pin.0, pin.1, ItemAction::Pin(it.id)),
                        (at(rect.right() - 8.0 - 2.0 * size - 4.0), Icon::Copy, "Copy", ItemAction::Copy(it.id)),
                        (at(rect.right() - 8.0 - size), Icon::Open, "Open details", ItemAction::Open(it.id)),
                    ] {
                        if icon_button_at(ui, b, egui::Id::unique(("card-action", it.id, tip)), icon, tip).clicked() {
                            actions.push(action);
                        }
                    }
                }
            }
        });
        self.list_view = (out.state.clamped_offset().y, out.inner_rect.height());
        self.apply_actions(actions);
    }

    fn empty_list(&mut self, ui: &mut Ui) {
        ui.add_space(60.0);
        // "text", "links"… in English; the Arabic nouns need no case change.
        let kind = if i18n::rtl() { tr(self.filter.label()).to_string() } else { self.filter.label().to_lowercase() };
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            let strong = ui.visuals().strong_text_color();
            if self.items.is_empty() {
                icons::glyph(ui, Icon::Clipboard, 40.0, weak);
                ui.add_space(8.0);
                lbl(ui, "Your clipboard history will appear here", 16.0, strong);
                ui.add_space(4.0);
                let help = trf("Copy anything, or drop files here.\nPress {key} to open this window from anywhere.", &[("key", &self.cfg.show_hotkey)]);
                ui.label(txt(&help, 14.0, Some(weak)));
            } else if !self.query.trim().is_empty() {
                icons::glyph(ui, Icon::Search, 36.0, weak);
                ui.add_space(8.0);
                ui.label(txt(&trf("No matches for “{q}”", &[("q", &self.query.trim())]), 15.0, Some(strong)));
                ui.add_space(4.0);
                match (&self.backend, self.cfg.search_mode) {
                    (BackendState::Off, _) => {
                        lbl(ui, "Searching by meaning finds related words too.", 14.0, weak);
                        if ui.link(txt(tr("Set up meaning search"), 14.0, None)).clicked() {
                            self.open_settings(SettingsPage::Search);
                        }
                    }
                    (_, SearchMode::Keyword) => {
                        if ui.link(txt(tr("Try “Smart” search instead"), 14.0, None)).clicked() {
                            self.cfg.search_mode = SearchMode::Hybrid;
                            self.save_config();
                            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                            self.dirty = true;
                        }
                    }
                    _ => {
                        lbl(ui, "Try fewer or different words.", 14.0, weak);
                    }
                }
                if self.filter != Filter::All && ui.link(txt(&trf("Search all items, not just {kind}", &[("kind", &kind)]), 14.0, None)).clicked() {
                    self.filter = Filter::All;
                    self.dirty = true;
                }
            } else {
                ui.label(txt(&trf("No {kind} yet", &[("kind", &kind)]), 14.0, Some(weak)));
            }
        });
    }

    /// The selected item in full. `closeable` when it is a side pane in Grid or
    /// List view, which gets a back button.
    fn details(&mut self, ui: &mut Ui, closeable: bool) {
        let ctx = ui.ctx().clone();
        let Some(it) = self.selected.and_then(|id| self.item(id)).cloned() else {
            self.empty_details(ui);
            return;
        };
        // Header: what it is at the reading start, what you can do at the far end.
        row(ui, |ui| {
            if closeable && icons::icon_button_ex(ui, Icon::Back, "Close details (Esc)", Vec2::splat(30.0), 20.0, None, None).clicked() {
                self.details_open = false;
            }
            icons::chip(ui, icons::kind(it.kind), it.kind.label(), kind_color(it.kind));
            if it.pinned {
                icons::chip(ui, Icon::PinFilled, "Pinned", accent());
            }
            if it.origin == Origin::Imported {
                chip(ui, "📂 From your files", Color32::GRAY);
            }
            far(ui, |ui| {
                let keys = self.cfg.keys.clone();
                if icons::icon_button(ui, Icon::Delete, &hint("Delete", &keys.delete)).clicked() {
                    self.delete(it.id);
                }
                let (pin, tip) = if it.pinned { (Icon::PinFilled, hint("Unpin", &keys.pin)) } else { (Icon::Pin, hint("Pin: keep forever", &keys.pin)) };
                let pin_color = it.pinned.then(accent);
                if icon_button_ex(ui, pin, &tip, Vec2::splat(34.0), 16.0, None, pin_color).clicked() {
                    self.toggle_pin(it.id);
                }
                if it.kind == Kind::Image && icons::icon_button(ui, Icon::Save, "Save image as…").clicked() {
                    if let (Some(src), Some(dst)) = (&it.image, rfd::FileDialog::new().set_file_name(format!("clip-{}.png", it.id)).save_file()) {
                        if let Err(e) = image::open(src).and_then(|i| i.save(&dst)) {
                            self.set_error(format!("save failed: {e}"));
                        }
                    }
                }
                if let Some(p) = &it.source {
                    if icons::icon_button(ui, Icon::Folder, "Show in folder").clicked() {
                        reveal(p);
                    }
                }
                if it.kind == Kind::Link && icons::icon_button(ui, Icon::Browser, "Open in browser").clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(it.text.trim()));
                }
                let copy_tip = trf("{action} · or double-click in the list", &[("action", &hint("Copy", &keys.copy))]);
                if icon_button_ex(ui, Icon::Copy, &copy_tip, Vec2::new(44.0, 34.0), 16.0, Some(accent()), Some(Color32::WHITE)).clicked() {
                    self.copy(it.id);
                }
            });
        });
        ui.add_space(2.0);
        let meta = self.meta_line(&it);
        let weak = ui.visuals().weak_text_color();
        row(ui, |ui| ui.label(txt(&meta, 12.5, Some(weak))));
        ui.add_space(6.0);
        self.tag_editor(ui, &it);
        ui.add_space(10.0);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match it.kind {
            Kind::Image => self.image_details(ui, &it),
            Kind::Files => {
                for line in it.text.lines() {
                    let p = PathBuf::from(line);
                    row(ui, |ui| {
                        icons::glyph(ui, if p.is_dir() { Icon::Folder } else { Icon::File }, 15.0, ui.visuals().text_color());
                        let name = p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| line.to_string());
                        if tip(ui.link(txt(&name, 14.5, None)), line).clicked() {
                            reveal(&p);
                        }
                        let parent = p.parent().map(|x| x.display().to_string()).unwrap_or_default();
                        ui.label(txt(&parent, 12.0, Some(ui.visuals().weak_text_color())));
                    });
                }
            }
            Kind::Link => {
                ui.hyperlink_to(RichText::new(it.text.trim()).size(16.0), it.text.trim());
            }
            Kind::Text => {
                let d = self.detail_text(&it);
                if d.monospace {
                    let code_bg = ui.visuals().panel_fill;
                    Frame::new().fill(code_bg).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(12)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(egui::Label::new(RichText::new(d.shown.as_str()).monospace().size(13.0)).selectable(true));
                    });
                } else if bidi::has_rtl(&d.shown) {
                    // Arabic: wrapped in reading order (not selectable; copy takes all of it).
                    let shown = d.shown.clone();
                    let c = ui.visuals().text_color();
                    bidi::paragraph(ui, &shown, FontId::proportional(15.0), c);
                } else {
                    ui.add(egui::Label::new(RichText::new(d.shown.as_str()).size(15.0)).selectable(true).wrap());
                }
            }
        });
    }

    fn meta_line(&mut self, it: &Item) -> String {
        let mut parts = vec![when(it.created)];
        match it.kind {
            Kind::Text => {
                let d = self.detail_text(it);
                let (c, w, code) = (d.chars, d.words, d.monospace);
                if code {
                    parts.push(trf("{n} lines of code", &[("n", &it.text.lines().count())]));
                }
                parts.push(trf("{n} characters", &[("n", &c)]));
                parts.push(if w == 1 { tr("1 word").to_string() } else { trf("{n} words", &[("n", &w)]) });
            }
            Kind::Image => {
                parts.push(format!("{} × {} px", it.width, it.height));
                match it.ocr.as_deref() {
                    Some(o) if !o.trim().is_empty() => parts.push(tr("contains text").into()),
                    Some(_) => parts.push(tr("no text found").into()),
                    None if self.cfg.ocr_images => parts.push(tr("reading text…").into()),
                    None => {}
                }
            }
            Kind::Link => parts.push(domain(&it.text).to_string()),
            Kind::Files => parts.push(trf("{n} item(s)", &[("n", &it.text.lines().count())])),
        }
        if let Some(src) = &it.source {
            parts.push(src.display().to_string());
        }
        parts.join("  ·  ")
    }

    fn image_details(&mut self, ui: &mut Ui, it: &Item) {
        match self.preview(it) {
            Preview::Ready(tex) => {
                let size = tex.size_vec2();
                let max_h = (ui.available_height() - 140.0).max(220.0);
                let scale = (ui.available_width() / size.x).min(max_h / size.y).min(1.0);
                ui.add(egui::Image::new((tex.id(), size * scale)).corner_radius(8.0));
            }
            Preview::Loading => {
                ui.add_space(24.0);
                ui.add(egui::Spinner::new().size(22.0));
            }
            Preview::Missing => {
                lbl(ui, "The image file is missing.", 14.0, ui.visuals().weak_text_color());
            }
        }
        if let Some(t) = it.ocr.as_deref().filter(|t| !t.trim().is_empty()) {
            ui.add_space(16.0);
            row(ui, |ui| {
                lbl(ui, "TEXT IN IMAGE", 11.5, ui.visuals().weak_text_color());
                if icon_button_ex(ui, Icon::Copy, "Copy text", Vec2::splat(26.0), 13.0, None, None).clicked() {
                    self.flags.ignore_next.store(true, Ordering::SeqCst);
                    let _ = clipboard::copy_text(t);
                    self.toast(tr("Copied text"));
                }
            });
            if bidi::has_rtl(t) {
                let c = ui.visuals().text_color();
                bidi::paragraph(ui, t, FontId::proportional(14.0), c);
            } else {
                ui.add(egui::Label::new(RichText::new(t).size(14.0)).selectable(true));
            }
        }
    }

    fn empty_details(&self, ui: &mut Ui) {
        ui.add_space(80.0);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            lbl(ui, "Select an item to see it here", 14.0, weak);
            ui.add_space(18.0);
            let k = &self.cfg.keys;
            let rows = [("Up+Down", "move"), (k.copy.as_str(), "copy & hide"), (k.pin.as_str(), "pin"), (k.tag.as_str(), "tag"), (k.delete.as_str(), "delete"), ("Escape", "clear / hide")];
            for (keys, what) in rows.into_iter().filter(|(k, _)| !k.trim().is_empty()) {
                row(ui, |ui| {
                    // Keys hug a shared centre line from one side, actions from the other.
                    let half = ui.available_width() / 2.0;
                    let (first, _) = ui.allocate_exact_size(Vec2::new(half - 8.0, 28.0), Sense::hover());
                    let mut caps = ui.new_child(egui::UiBuilder::new().max_rect(first).layout(far_layout()));
                    // A right-to-left child lays the caps out backwards; undo that.
                    let keys = if i18n::rtl() { keys.to_string() } else { keys.split('+').rev().collect::<Vec<_>>().join("+") };
                    shortcut::keycaps(&mut caps, &keys, false);
                    ui.add_space(8.0);
                    lbl(ui, what, 12.5, weak);
                });
            }
        });
    }

    fn toasts(&mut self, ctx: &egui::Context) {
        if let Some((t, at, how_long)) = &self.toast {
            if at.elapsed() > *how_long {
                self.toast = None;
                return;
            }
            let dark = ctx.theme() == egui::Theme::Dark;
            egui::Area::new(egui::Id::unique("toast"))
                .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -48.0))
                .show(ctx, |ui| {
                    Frame::new()
                        .fill(if dark { Color32::from_gray(235) } else { Color32::from_gray(40) })
                        .corner_radius(CornerRadius::same(18))
                        .inner_margin(Margin::symmetric(16, 8))
                        .show(ui, |ui| ui.label(txt(t.as_str(), 14.0, Some(if dark { Color32::from_gray(20) } else { Color32::WHITE }))));
                });
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if let Some((_, at)) = &self.error {
            if at.elapsed() > Duration::from_secs(20) {
                self.error = None;
            }
        }
    }
}

/// The item's thumbnail texture, queuing a decode the first time it's seen.
fn thumb_for(it: &Item, thumbs: &HashMap<u64, Option<TextureHandle>>, loader: &mut ImageLoader) -> Option<TextureHandle> {
    match (it.kind, thumbs.get(&it.id), &it.image) {
        (Kind::Image, Some(t), _) => t.clone(),
        (Kind::Image, None, Some(p)) => {
            loader.request(it.id, p, false);
            None
        }
        _ => None,
    }
}

/// `hot` while the pointer is over the row: its hover buttons then take the
/// right edge, so the time and badges are left out.
#[allow(clippy::too_many_arguments)]
fn paint_row(ui: &Ui, rect: Rect, hot: bool, it: &Item, hit: &Hit, thumb: Option<TextureHandle>, selected: bool, compact: bool, terms: &[String], max_sem: f32) {
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let a = accent();
    // Laid out left to right, then mirrored for Arabic.
    let m = |r: Rect| flip(rect, r);
    let rtl = i18n::rtl();
    if selected {
        p.rect_filled(rect, CornerRadius::same(8), a.gamma_multiply(if v.dark_mode { 0.22 } else { 0.12 }));
        p.rect_filled(m(Rect::from_min_size(rect.min + Vec2::new(0.0, 8.0), Vec2::new(3.0, rect.height() - 16.0))), CornerRadius::same(2), a);
    } else if hot {
        p.rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6));
    }

    let av = if compact { 26.0 } else { 38.0 };
    let av_rect = m(Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.center().y - av / 2.0), Vec2::splat(av)));
    match thumb {
        Some(t) => {
            let s = t.size_vec2();
            let k = (av / s.x).max(av / s.y);
            // Fill the square, cropping the longer side.
            let uv_w = (av / (s.x * k)).min(1.0);
            let uv_h = (av / (s.y * k)).min(1.0);
            let uv = Rect::from_min_max(Pos2::new((1.0 - uv_w) / 2.0, (1.0 - uv_h) / 2.0), Pos2::new((1.0 + uv_w) / 2.0, (1.0 + uv_h) / 2.0));
            egui::Image::new((t.id(), Vec2::splat(av))).uv(uv).corner_radius(6.0).paint_at(ui, av_rect);
        }
        None => {
            let c = kind_color(it.kind);
            p.rect_filled(av_rect, CornerRadius::same(8), c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }));
            let code = it.kind == Kind::Text && looks_like_code(&it.text.chars().take(600).collect::<String>());
            if code {
                p.text(av_rect.center(), Align2::CENTER_CENTER, "</>", FontId::monospace(av * 0.3), c);
            } else {
                icons::paint(&p, Rect::from_center_size(av_rect.center(), Vec2::splat(av * 0.46)), icons::kind(it.kind), c);
            }
        }
    }

    let x = rect.left() + 12.0 + av + 12.0;
    let right_w = if hot { 134.0 } else { 64.0 };
    let text_w = (rect.right() - x - right_w - 8.0).max(40.0);
    // The text column, mirrored with the rest; lines start at its reading edge.
    let column = m(Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(text_w, rect.height())));
    let title_color = v.strong_text_color();
    let weak = v.weak_text_color();
    let sub = if compact { String::new() } else { subtitle(it) };
    let title = row_title(it);
    let title = if title.is_empty() { tr("(empty)").to_string() } else { title };

    let tg = one_line(ui, &title, terms, FontId::proportional(14.0), title_color, text_w);
    // One line sits on the row's midline, beside the icon; two lines share it.
    let title_y = if sub.is_empty() { rect.center().y - tg.size().y / 2.0 } else { rect.top() + 10.0 };
    let line_x = |w: f32| if rtl { column.right() - w } else { column.left() };
    p.galley(Pos2::new(line_x(tg.size().x), title_y), tg, title_color);
    if !sub.is_empty() {
        let sg = one_line(ui, &sub, terms, FontId::proportional(12.5), weak, text_w);
        p.galley(Pos2::new(line_x(sg.size().x), rect.top() + 32.0), sg, weak);
    }

    if hot {
        return;
    }
    // Far edge: time, then pinned star and relevance.
    let one_line_row = compact || sub.is_empty();
    let time_y = if one_line_row { rect.center().y } else { rect.top() + 18.0 };
    let time_at = m(Rect::from_min_size(Pos2::new(rect.right() - 12.0, time_y), Vec2::ZERO)).min;
    let time_anchor = if rtl { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER };
    paint_text(ui, &p, time_at, time_anchor, &row_time(it.created), FontId::proportional(11.5), weak);
    let mut ix = rect.right() - 12.0;
    // On one line the badges sit before the time instead of under it.
    let iy = if one_line_row { rect.center().y } else { rect.top() + 40.0 };
    if one_line_row {
        ix -= 56.0;
    }
    if it.pinned {
        icons::paint(&p, m(Rect::from_center_size(Pos2::new(ix - 6.5, iy), Vec2::splat(13.0))), Icon::PinFilled, a);
        ix -= 18.0;
    }
    if let Some(s) = hit.semantic {
        // Three bars: how close this match is to the best one.
        let ratio = if max_sem > 0.0 { s / max_sem } else { 0.0 };
        let level = if ratio > 0.93 { 3 } else if ratio > 0.8 { 2 } else { 1 };
        for b in 0..3 {
            let h = 4.0 + b as f32 * 3.0;
            let r = m(Rect::from_min_size(Pos2::new(ix - 18.0 + b as f32 * 6.0, iy + 6.0 - h), Vec2::new(4.0, h)));
            p.rect_filled(r, CornerRadius::same(1), if b < level { a } else { weak.gamma_multiply(0.35) });
        }
    }
}

/// One line of item text cut to `width`, query terms highlighted; Arabic in
/// reading order.
fn one_line(ui: &Ui, text: &str, terms: &[String], font: FontId, color: Color32, width: f32) -> Arc<egui::Galley> {
    if bidi::has_rtl(text) {
        let first = text.lines().next().unwrap_or("");
        // In the Arabic UI a line with Arabic in it reads right to left, even when
        // it starts with a Latin tag or word.
        let base = if i18n::rtl() { Some(true) } else { None };
        bidi::lines(ui, first, terms, font, color, accent().gamma_multiply(0.28), width, 1, base).into_iter().next().map(|(g, _)| g).unwrap_or_else(|| ui.fonts_mut(|f| f.layout_no_wrap(String::new(), FontId::proportional(12.0), color)))
    } else {
        ui.fonts_mut(|f| f.layout_job(highlighted(text, terms, font, color, width, 1)))
    }
}

/// Height of the action strip a card shows while hovered.
const CARD_FOOTER: f32 = 42.0;
/// Height of a card's tags row, above the action strip.
const TAG_ROW: f32 = 28.0;

/// A grid card: kind and age on top, as much of the content as fits below,
/// and delete / pin / copy / open buttons in a footer strip while hovered.
fn paint_card(ui: &Ui, rect: Rect, hot: bool, it: &Item, thumb: Option<TextureHandle>, selected: bool, terms: &[String]) {
    let v = ui.visuals();
    let a = accent();
    let radius = CornerRadius::same(10);
    let p = ui.painter();
    let m = |r: Rect| flip(rect, r);
    let rtl = i18n::rtl();
    let shadow = egui::epaint::Shadow { offset: [0, 2], blur: if hot { 14 } else { 8 }, spread: 0, color: Color32::from_black_alpha(match (v.dark_mode, hot) {
        (true, _) => 90,
        (false, true) => 34,
        (false, false) => 20,
    }) };
    p.add(shadow.as_shape(rect, radius));
    p.rect_filled(rect, radius, v.extreme_bg_color);

    // Header: kind chip at the reading start, pin and age at the far end.
    let pad = 16.0;
    let c = kind_color(it.kind);
    let cg = ui.fonts_mut(|f| f.layout_job(bidi::job(tr(it.kind.label()), FontId::proportional(12.5), c)));
    let chip_rect = m(Rect::from_min_size(Pos2::new(rect.left() + pad, rect.top() + 14.0), Vec2::new(cg.size().x + 12.0 + 6.0 + 16.0, cg.size().y + 6.0)));
    p.rect_filled(chip_rect, CornerRadius::same(7), c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }));
    icons::paint_icon_text(p, chip_rect.shrink2(Vec2::new(8.0, 3.0)), icons::kind(it.kind), 12.0, cg, c);
    let weak = v.weak_text_color();
    let age_at = m(Rect::from_min_size(Pos2::new(rect.right() - pad, chip_rect.center().y), Vec2::ZERO)).min;
    let age = paint_text(ui, p, age_at, if rtl { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER }, &ago(it.created), FontId::proportional(12.0), weak);
    if it.pinned {
        let x = if rtl { age.right() + 12.0 } else { age.left() - 12.0 };
        icons::paint(p, Rect::from_center_size(Pos2::new(x, chip_rect.center().y), Vec2::splat(12.0)), Icon::PinFilled, a);
    }

    // Bottom up: the button strip (empty until hovered), the tags row above it
    // when there are tags, then the content. Hovering only fills the strip,
    // so nothing else moves, shrinks or hides.
    let strip_top = rect.bottom() - CARD_FOOTER;
    let tags_top = if it.tags.is_empty() { strip_top } else { strip_top - TAG_ROW };
    if !it.tags.is_empty() {
        let row = Rect::from_min_max(Pos2::new(rect.left() + pad, tags_top), Pos2::new(rect.right() - pad, strip_top - 2.0));
        let tp = ui.painter_at(row);
        let mut x = row.left();
        for t in &it.tags {
            let c = tag_color(t);
            let g = ui.fonts_mut(|f| f.layout_job(bidi::job(&format!("# {t}"), FontId::proportional(11.5), c)));
            let r = flip(rect, Rect::from_min_size(Pos2::new(x, row.center().y - (g.size().y + 6.0) / 2.0), g.size() + Vec2::new(14.0, 6.0)));
            tp.rect_filled(r, CornerRadius::same(9), c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }));
            x += r.width() + 6.0;
            tp.galley(r.min + Vec2::new(7.0, 3.0), g, c);
        }
    }
    let bottom = tags_top - 4.0;
    let body = Rect::from_min_max(Pos2::new(rect.left() + pad, chip_rect.bottom() + 12.0), Pos2::new(rect.right() - pad, bottom));
    let bp = ui.painter_at(body);
    match (it.kind, thumb) {
        (Kind::Image, Some(t)) => {
            let s = t.size_vec2();
            let k = (body.width() / s.x).min(body.height() / s.y).min(2.0);
            let img = flip(body, Rect::from_min_size(body.min, s * k));
            egui::Image::new((t.id(), s * k)).corner_radius(6.0).paint_at(ui, img);
            bp.rect_stroke(img, CornerRadius::same(6), Stroke::new(1.0, weak.gamma_multiply(0.25)), egui::StrokeKind::Inside);
        }
        (Kind::Image, None) => {
            let at = flip(body, Rect::from_min_size(body.left_top(), Vec2::ZERO)).min;
            bp.text(at, if rtl { Align2::RIGHT_TOP } else { Align2::LEFT_TOP }, format!("{} × {}", it.width, it.height), FontId::proportional(13.0), weak);
        }
        _ => {
            let code = it.kind == Kind::Text && looks_like_code(&it.text.chars().take(600).collect::<String>());
            let (font, line_h) = if code { (FontId::monospace(12.5), 17.0) } else { (FontId::proportional(14.5), 20.0) };
            let color = if it.kind == Kind::Link { v.hyperlink_color } else { v.strong_text_color() };
            let text: String = match it.kind {
                Kind::Files => it.text.lines().map(|l| std::path::Path::new(l).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| l.to_string())).collect::<Vec<_>>().join("\n"),
                _ => it.text.lines().take(14).collect::<Vec<_>>().join("\n").chars().take(900).collect(),
            };
            let text = if text.trim().is_empty() { tr("(empty)").to_string() } else { text };
            let rows = ((body.height() / line_h).floor() as usize).max(1);
            if bidi::has_rtl(&text) {
                // Each line lines up with its own reading edge.
                let lines = bidi::lines(ui, text.trim_end(), terms, font, color, a.gamma_multiply(0.28), body.width(), rows, None);
                bidi::paint(&bp, body, lines, color, None);
            } else {
                let g = ui.fonts_mut(|f| f.layout_job(highlighted(text.trim_end(), terms, font, color, body.width(), rows)));
                bp.galley(body.min, g, color);
            }
        }
    }

    if hot {
        let footer = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - CARD_FOOTER), rect.max);
        let r = CornerRadius { nw: 0, ne: 0, sw: 10, se: 10 };
        p.rect_filled(footer, r, if v.dark_mode { Color32::from_gray(44) } else { Color32::from_gray(238) });
    }
    // Outline last, so the footer doesn't cover it.
    if selected {
        p.rect_stroke(rect, radius, Stroke::new(2.0, a), egui::StrokeKind::Inside);
    } else if hot {
        p.rect_stroke(rect, radius, Stroke::new(1.0, weak.gamma_multiply(0.35)), egui::StrokeKind::Inside);
    }
}

pub(super) fn reveal(p: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = if p.is_dir() {
            std::process::Command::new("explorer").arg(p).spawn()
        } else {
            std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn()
        };
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::embed::server::stop_launched_server();
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.window_logic(ctx);
        self.handle_events(ctx);
        self.focused.store(ctx.input(|i| i.viewport().focused.unwrap_or(false)), Ordering::SeqCst);
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            self.import(dropped);
        }
        if self.query_changed.is_some() {
            ctx.request_repaint_after(QUERY_DEBOUNCE);
        }
        if self.research_pending {
            ctx.request_repaint_after(RESEARCH_EVERY);
        }
        if self.queue > 0 || self.backend == BackendState::Loading {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.keyboard(&ctx);
        self.refresh_results();
        if self.draft.is_some() {
            self.settings_ui(ui);
        } else {
            self.top_bar(ui);
            self.status_bar(ui);
            self.main_area(ui);
        }
        self.toasts(&ctx);
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("drop")));
            p.rect_filled(r, 0.0, Color32::from_black_alpha(160));
            let g = ctx.fonts_mut(|f| f.layout_job(bidi::job(tr("Drop to add to your searchable library"), FontId::proportional(22.0), Color32::WHITE)));
            let row = Rect::from_center_size(r.center(), Vec2::new(26.0 + 6.0 + g.size().x, g.size().y.max(26.0)));
            icons::paint_icon_text(&p, row, Icon::Folder, 26.0, g, Color32::WHITE);
        }
    }
}
