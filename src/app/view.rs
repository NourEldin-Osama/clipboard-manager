//! Main window: search bar, filter chips, grouped list, details pane, status bar.
//!
//! Structure comes from alignment, background tone and type weight rather than
//! borders: the list and details panes differ by fill, rows by tint, groups by
//! small headers on the same left edge as the content.

use std::sync::atomic::AtomicUsize;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Align, Align2, Frame, Layout, Margin, Pos2, Rect, Ui};

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

pub(super) fn kind_icon(k: Kind) -> &'static str {
    match k {
        Kind::Text => "📝",
        Kind::Link => "🔗",
        Kind::Image => "🖼",
        Kind::Files => "📁",
    }
}

fn filter_icon(f: Filter) -> &'static str {
    match f {
        Filter::All => "📋",
        Filter::Text => "📝",
        Filter::Links => "🔗",
        Filter::Images => "🖼",
        Filter::Files => "📁",
        Filter::Pinned => "★",
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

fn local_offset() -> time::UtcOffset {
    static OFFSET: std::sync::OnceLock<time::UtcOffset> = std::sync::OnceLock::new();
    *OFFSET.get_or_init(|| time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC))
}

fn local(ts: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(ts).unwrap_or(time::OffsetDateTime::UNIX_EPOCH).to_offset(local_offset())
}

/// Whole days between the item's local date and today (0 = today).
fn days_ago(ts: i64) -> i64 {
    let today = local(now()).date();
    (today - local(ts).date()).whole_days()
}

fn fmt(dt: time::OffsetDateTime, f: &[time::format_description::BorrowedFormatItem<'_>]) -> String {
    dt.format(f).unwrap_or_default()
}

/// Short time for the right edge of a row; the group header already gives the day.
fn row_time(ts: i64) -> String {
    let dt = local(ts);
    match days_ago(ts) {
        0 | 1 => fmt(dt, time::macros::format_description!("[hour]:[minute]")),
        2..=6 => fmt(dt, time::macros::format_description!("[weekday repr:short]")),
        _ if dt.year() == local(now()).year() => fmt(dt, time::macros::format_description!("[day padding:none] [month repr:short]")),
        _ => fmt(dt, time::macros::format_description!("[day padding:none] [month repr:short] [year]")),
    }
}

/// "Today at 09:59", "Yesterday at 18:20", "Mon 14 Oct at 10:00".
pub(super) fn when(ts: i64) -> String {
    let dt = local(ts);
    let hm = fmt(dt, time::macros::format_description!("[hour]:[minute]"));
    match days_ago(ts) {
        0 => format!("Today at {hm}"),
        1 => format!("Yesterday at {hm}"),
        _ => format!("{} at {hm}", fmt(dt, time::macros::format_description!("[weekday repr:short] [day padding:none] [month repr:short] [year]"))),
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

/// A borderless icon button that only shows a background on hover.
pub(super) fn icon_button(ui: &mut Ui, icon: &str, tip: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(icon).size(16.0)).frame_when_inactive(false).min_size(Vec2::splat(34.0)))
        .on_hover_text(tip)
}

/// Rounded tinted label: kind, state or status.
pub(super) fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| ui.label(RichText::new(text.into()).size(12.0).color(color)))
        .response
}

pub(super) fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().color(Color32::WHITE))
            .fill(accent())
            .min_size(Vec2::new(0.0, 32.0)),
    )
}

/// `text` with every occurrence of the query terms highlighted.
fn highlighted(text: &str, terms: &[String], font: FontId, color: Color32, max_width: f32) -> LayoutJob {
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
    job.wrap = TextWrapping { max_width, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    job
}

/// Domain of a URL, e.g. `huggingface.co`.
fn domain(url: &str) -> &str {
    let rest = url.trim().split("://").nth(1).unwrap_or(url.trim());
    rest.split(['/', '?', '#']).next().unwrap_or(rest).trim_start_matches("www.")
}

/// Second line of an item row: the most useful detail for its kind.
fn subtitle(it: &Item) -> String {
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
                format!("{n} files")
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
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.close_settings(ctx);
            }
            return;
        }
        let (down, up, enter, esc, del, pin) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.modifiers.shift && i.key_pressed(Key::Delete),
                i.modifiers.command && i.key_pressed(Key::P),
            )
        });
        if down {
            self.move_selection(1);
        }
        if up {
            self.move_selection(-1);
        }
        if let Some(id) = self.selected {
            if enter {
                self.copy(id, ctx);
            }
            if del {
                self.delete(id);
            }
            if pin {
                self.toggle_pin(id);
            }
        }
        if esc {
            if self.query.is_empty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
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
                ui.horizontal(|ui| {
                    let actions_w = 3.0 * 40.0;
                    let w = (ui.available_width() - actions_w).max(200.0);
                    self.search_field(ui, w);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, "⚙", "Settings").clicked() {
                            self.open_settings(SettingsPage::Search);
                        }
                        let import = icon_button(ui, "+", "Add files or a folder to search (you can also drop them on the window)");
                        egui::Popup::menu(&import).show(|ui| {
                            if ui.button("📄  Add files…").clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_files() {
                                    self.import(f);
                                }
                            }
                            if ui.button("📁  Add a folder…").clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_folder() {
                                    self.import(vec![f]);
                                }
                            }
                        });
                        let rec = self.flags.enabled.load(Ordering::SeqCst);
                        let (icon, tip) = if rec { ("⏸", "Pause recording") } else { ("▶", "Resume recording") };
                        let b = egui::Button::new(RichText::new(icon).size(16.0).color(if rec { ui.visuals().text_color() } else { Color32::WHITE }))
                            .frame_when_inactive(!rec)
                            .fill(if rec { Color32::TRANSPARENT } else { AMBER })
                            .min_size(Vec2::splat(34.0));
                        if ui.add(b).on_hover_text(tip).clicked() {
                            self.cfg.monitor_clipboard = !rec;
                            self.flags.enabled.store(!rec, Ordering::SeqCst);
                            self.save_config();
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
            .inner_margin(Margin { left: 12, right: 6, top: 0, bottom: 0 })
            .show(ui, |ui| {
                ui.set_width(width - 18.0);
                ui.set_height(40.0);
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("🔍").size(15.0).color(ui.visuals().weak_text_color()));
                    let hint = match &self.backend {
                        BackendState::Ready { images: true, .. } => "Search text and images by meaning…",
                        BackendState::Ready { .. } => "Search by words or meaning…",
                        _ => "Search your clipboard…",
                    };
                    let te = egui::TextEdit::singleline(&mut self.query)
                        .id(search_id())
                        .frame(Frame::NONE)
                        .hint_text(hint)
                        .font(FontId::proportional(16.0))
                        .desired_width((ui.available_width() - 130.0).max(80.0));
                    let resp = ui.add(te);
                    if self.focus_search.swap(false, Ordering::SeqCst) {
                        resp.request_focus();
                    }
                    if resp.changed() {
                        self.query_changed = Some(Instant::now());
                        self.dirty = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Search mode, quiet until needed.
                        let mode = self.cfg.search_mode;
                        let non_default = mode != SearchMode::Hybrid;
                        let text = RichText::new(format!("{} ▾", mode_label(mode))).size(13.0).color(if non_default { accent() } else { ui.visuals().weak_text_color() });
                        let b = ui.add(egui::Button::new(text).frame_when_inactive(non_default).fill(if non_default { accent().gamma_multiply(0.12) } else { Color32::TRANSPARENT }));
                        egui::Popup::menu(&b).show(|ui| {
                            ui.set_min_width(300.0);
                            for m in [SearchMode::Hybrid, SearchMode::Semantic, SearchMode::Keyword] {
                                let label = RichText::new(mode_label(m)).strong();
                                if ui.selectable_label(mode == m, label).clicked() {
                                    self.cfg.search_mode = m;
                                    self.save_config();
                                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                                    self.dirty = true;
                                }
                                ui.label(RichText::new(mode_help(m)).small().weak());
                                ui.add_space(4.0);
                            }
                        });
                        if !self.query.is_empty() && ui.add(egui::Button::new(RichText::new("✖").size(12.0)).frame_when_inactive(false)).on_hover_text("Clear (Esc)").clicked() {
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
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for f in Filter::ALL {
                let n = self.items.iter().filter(|i| f.accepts(i)).count();
                let selected = self.filter == f;
                // Empty categories add noise; keep them out unless selected.
                if n == 0 && f != Filter::All && !selected {
                    continue;
                }
                let (fg, weak) = if selected { (accent(), accent().gamma_multiply(0.75)) } else { (ui.visuals().text_color(), ui.visuals().weak_text_color()) };
                let mut job = LayoutJob::default();
                job.append(&format!("{}  {}", filter_icon(f), f.label()), 0.0, TextFormat { font_id: FontId::proportional(13.5), color: fg, ..Default::default() });
                job.append(&format!("  {n}"), 0.0, TextFormat { font_id: FontId::proportional(12.0), color: weak, ..Default::default() });
                let b = egui::Button::new(job)
                    .frame_when_inactive(selected)
                    .fill(if selected { accent().gamma_multiply(0.13) } else { Color32::TRANSPARENT })
                    .corner_radius(CornerRadius::same(14));
                if ui.add(b).clicked() {
                    self.filter = f;
                    self.dirty = true;
                }
            }
            if !self.query.trim().is_empty() {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(4.0);
                    let n = self.results.len();
                    ui.label(RichText::new(format!("{n} {}", if n == 1 { "result" } else { "results" })).size(12.5).color(ui.visuals().weak_text_color()));
                });
            }
        });
    }

    pub(super) fn model_status(&self) -> (Color32, String) {
        match (&self.backend, &self.cfg.backend) {
            (BackendState::Loading, _) => (AMBER, "Loading search model…".into()),
            (BackendState::Ready { .. }, EmbedBackend::SigLip { repo }) => (GREEN, format!("Built-in · {}", repo.rsplit('/').next().unwrap_or(repo))),
            (BackendState::Ready { .. }, EmbedBackend::Server(s)) => {
                let name = if !s.model.trim().is_empty() {
                    s.model.trim().to_string()
                } else {
                    self.model_list.found.default.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| format!("{} (default model)", domain(&s.base_url)))
                };
                (GREEN, name)
            }
            (BackendState::Off, EmbedBackend::None) => (Color32::GRAY, "Words only · set up meaning search".into()),
            (BackendState::Off, _) => (RED, "Search model unavailable · fix in Settings".into()),
            (BackendState::Ready { .. }, EmbedBackend::None) => (Color32::GRAY, "Words only".into()),
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(Frame::new().inner_margin(Margin::symmetric(16, 6)).fill(fill))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let weak = ui.visuals().weak_text_color();
                    let (dot, text) = self.model_status();
                    let r = ui
                        .add(
                            egui::Button::new(RichText::new(format!("● {text}")).size(12.0).color(if dot == Color32::GRAY { weak } else { dot }))
                                .frame_when_inactive(false),
                        )
                        .on_hover_text("Search model: click to change");
                    if r.clicked() {
                        self.open_settings(SettingsPage::Search);
                    }
                    if self.queue > 0 {
                        ui.add(egui::Spinner::new().size(11.0).color(weak));
                        ui.label(RichText::new(format!("Indexing · {} left", self.queue)).size(12.0).color(weak));
                    }
                    if !self.status.is_empty() {
                        ui.label(RichText::new(&self.status).size(12.0).color(weak));
                    }
                    if !self.flags.enabled.load(Ordering::SeqCst) {
                        chip(ui, "⏸ Recording paused", AMBER);
                    }
                    if let Some((e, _)) = &self.error {
                        let r = ui.add(egui::Label::new(RichText::new(format!("⚠ {e}")).size(12.0).color(RED)).truncate().sense(Sense::click()));
                        if r.on_hover_text("Click to dismiss").clicked() {
                            self.error = None;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pinned = self.items.iter().filter(|i| i.pinned).count();
                        let mut s = format!("{} items", self.items.len());
                        if pinned > 0 {
                            s.push_str(&format!(" · {pinned} pinned"));
                        }
                        ui.label(RichText::new(s).size(12.0).color(weak));
                    });
                });
            });
    }

    fn list(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        let ctx = ui.ctx().clone();
        egui::Panel::left("list")
            .resizable(true)
            .show_separator_line(false)
            .default_size(ui.available_width() * 0.42)
            .size_range(300.0..=900.0)
            .frame(Frame::new().inner_margin(Margin { left: 8, right: 8, top: 4, bottom: 4 }).fill(fill))
            .show(ui, |ui| {
                if self.results.is_empty() {
                    self.empty_list(ui);
                    return;
                }
                let compact = self.cfg.density == Density::Compact;
                let row_h = if compact { 40.0 } else { 58.0 };
                let header_h = 30.0;
                let searching = !self.query.trim().is_empty();

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
                let mut clicked = None;
                let mut double = None;
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
                                p.text(Pos2::new(rect.left() + 12.0, rect.bottom() - 7.0), Align2::LEFT_BOTTOM, label.to_uppercase(), FontId::proportional(11.5), weak);
                                p.text(Pos2::new(rect.right() - 12.0, rect.bottom() - 7.0), Align2::RIGHT_BOTTOM, n.to_string(), FontId::proportional(11.5), weak.gamma_multiply(0.7));
                            }
                            Line::Row(r) => {
                                let hit = &results[*r];
                                let it = &items[hit.idx];
                                let thumb = match (it.kind, thumbs.get(&it.id), &it.image) {
                                    (Kind::Image, Some(t), _) => t.clone(),
                                    (Kind::Image, None, Some(p)) => {
                                        loader.request(it.id, p, false);
                                        None
                                    }
                                    _ => None,
                                };
                                let resp = ui.interact(rect, egui::Id::unique(("row", it.id)), Sense::click());
                                paint_row(ui, rect, &resp, it, hit, thumb, Some(it.id) == selected, compact, &terms, max_sem);
                                if resp.clicked() {
                                    clicked = Some(it.id);
                                }
                                if resp.double_clicked() {
                                    double = Some(it.id);
                                }
                            }
                        }
                    }
                });
                self.list_view = (out.state.clamped_offset().y, out.inner_rect.height());
                if let Some(id) = clicked {
                    self.selected = Some(id);
                }
                if let Some(id) = double {
                    self.copy(id, &ctx);
                }
                if self.thumbs.len() > MAX_THUMBS {
                    for id in self.thumbs.drain().map(|(id, _)| id).collect::<Vec<_>>() {
                        self.loader.forget(id, false);
                    }
                }
            });
    }

    fn empty_list(&mut self, ui: &mut Ui) {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            if self.items.is_empty() {
                ui.label(RichText::new("📋").size(40.0).color(weak));
                ui.add_space(8.0);
                ui.label(RichText::new("Your clipboard history will appear here").size(16.0).strong());
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Copy anything, or drop files here.\nPress {} to open this window from anywhere.", self.cfg.show_hotkey)).color(weak));
            } else if !self.query.trim().is_empty() {
                ui.label(RichText::new("🔍").size(36.0).color(weak));
                ui.add_space(8.0);
                ui.label(RichText::new(format!("No matches for “{}”", self.query.trim())).size(15.0).strong());
                ui.add_space(4.0);
                match (&self.backend, self.cfg.search_mode) {
                    (BackendState::Off, _) => {
                        ui.label(RichText::new("Searching by meaning finds related words too.").color(weak));
                        if ui.link("Set up meaning search").clicked() {
                            self.open_settings(SettingsPage::Search);
                        }
                    }
                    (_, SearchMode::Keyword) => {
                        if ui.link("Try “Smart” search instead").clicked() {
                            self.cfg.search_mode = SearchMode::Hybrid;
                            self.save_config();
                            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                            self.dirty = true;
                        }
                    }
                    _ => {
                        ui.label(RichText::new("Try fewer or different words.").color(weak));
                    }
                }
                if self.filter != Filter::All && ui.link(format!("Search all items, not just {}", self.filter.label().to_lowercase())).clicked() {
                    self.filter = Filter::All;
                    self.dirty = true;
                }
            } else {
                ui.label(RichText::new(format!("No {} yet", self.filter.label().to_lowercase())).color(weak));
            }
        });
    }

    fn details(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let fill = ui.visuals().extreme_bg_color;
        egui::CentralPanel::default().frame(Frame::new().inner_margin(Margin { left: 24, right: 20, top: 14, bottom: 14 }).fill(fill)).show(ui, |ui| {
            let Some(it) = self.selected.and_then(|id| self.item(id)).cloned() else {
                self.empty_details(ui);
                return;
            };
            // Header: what it is on the left, what you can do on the right.
            ui.horizontal(|ui| {
                chip(ui, format!("{}  {}", kind_icon(it.kind), it.kind.label()), kind_color(it.kind));
                if it.pinned {
                    chip(ui, "★ Pinned", accent());
                }
                if it.origin == Origin::Imported {
                    chip(ui, "📂 From your files", Color32::GRAY);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, "🗑", "Delete (Shift+Del)").clicked() {
                        self.delete(it.id);
                    }
                    let pin = if it.pinned { "★" } else { "☆" };
                    if icon_button(ui, pin, if it.pinned { "Unpin (Ctrl+P)" } else { "Pin: keep forever (Ctrl+P)" }).clicked() {
                        self.toggle_pin(it.id);
                    }
                    if it.kind == Kind::Image && icon_button(ui, "💾", "Save image as…").clicked() {
                        if let (Some(src), Some(dst)) = (&it.image, rfd::FileDialog::new().set_file_name(format!("clip-{}.png", it.id)).save_file()) {
                            if let Err(e) = image::open(src).and_then(|i| i.save(&dst)) {
                                self.set_error(format!("save failed: {e}"));
                            }
                        }
                    }
                    if let Some(p) = &it.source {
                        if icon_button(ui, "📂", "Show in folder").clicked() {
                            reveal(p);
                        }
                    }
                    if it.kind == Kind::Link && icon_button(ui, "🌐", "Open in browser").clicked() {
                        ctx.open_url(egui::OpenUrl::new_tab(it.text.trim()));
                    }
                    if primary_button(ui, "  Copy  ").on_hover_text("Enter · or double-click in the list").clicked() {
                        self.copy(it.id, &ctx);
                    }
                });
            });
            ui.add_space(2.0);
            ui.label(RichText::new(self.meta_line(&it)).size(12.5).color(ui.visuals().weak_text_color()));
            ui.add_space(14.0);

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match it.kind {
                Kind::Image => self.image_details(ui, &it),
                Kind::Files => {
                    for line in it.text.lines() {
                        let p = PathBuf::from(line);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if p.is_dir() { "📁" } else { "📄" }).size(15.0));
                            let name = p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| line.to_string());
                            if ui.link(RichText::new(name).size(14.5)).on_hover_text(line).clicked() {
                                reveal(&p);
                            }
                            ui.label(RichText::new(p.parent().map(|x| x.display().to_string()).unwrap_or_default()).size(12.0).weak());
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
                    } else {
                        ui.add(egui::Label::new(RichText::new(d.shown.as_str()).size(15.0)).selectable(true).wrap());
                    }
                }
            });
        });
    }

    fn meta_line(&mut self, it: &Item) -> String {
        let mut parts = vec![when(it.created)];
        match it.kind {
            Kind::Text => {
                let d = self.detail_text(it);
                let (c, w, code) = (d.chars, d.words, d.monospace);
                if code {
                    parts.push(format!("{} lines of code", it.text.lines().count()));
                }
                parts.push(format!("{c} characters"));
                parts.push(format!("{w} {}", if w == 1 { "word" } else { "words" }));
            }
            Kind::Image => {
                parts.push(format!("{} × {} px", it.width, it.height));
                match it.ocr.as_deref() {
                    Some(o) if !o.trim().is_empty() => parts.push("contains text".into()),
                    Some(_) => parts.push("no text found".into()),
                    None if self.cfg.ocr_images => parts.push("reading text…".into()),
                    None => {}
                }
            }
            Kind::Link => parts.push(domain(&it.text).to_string()),
            Kind::Files => parts.push(format!("{} item(s)", it.text.lines().count())),
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
                ui.label(RichText::new("The image file is missing.").color(ui.visuals().weak_text_color()));
            }
        }
        if let Some(t) = it.ocr.as_deref().filter(|t| !t.trim().is_empty()) {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("TEXT IN IMAGE").size(11.5).color(ui.visuals().weak_text_color()));
                if ui.add(egui::Button::new(RichText::new("📋 Copy text").size(12.0)).frame_when_inactive(false)).clicked() {
                    self.flags.ignore_next.store(true, Ordering::SeqCst);
                    let _ = clipboard::copy_text(t);
                    self.toast("Copied text");
                }
            });
            ui.add(egui::Label::new(RichText::new(t).size(14.0)).selectable(true));
        }
    }

    fn empty_details(&self, ui: &mut Ui) {
        ui.add_space(80.0);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            ui.label(RichText::new("Select an item to see it here").color(weak));
            ui.add_space(18.0);
            for (keys, what) in [("↑ ↓", "move"), ("Enter", "copy & hide"), ("Ctrl P", "pin"), ("Shift Del", "delete"), ("Esc", "clear / hide")] {
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 210.0).max(0.0) / 2.0);
                    ui.add_sized([86.0, 22.0], egui::Label::new(RichText::new(keys).monospace().size(12.0)));
                    ui.label(RichText::new(what).size(12.5).color(weak));
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
                        .show(ui, |ui| ui.label(RichText::new(t.as_str()).color(if dark { Color32::from_gray(20) } else { Color32::WHITE })));
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

#[allow(clippy::too_many_arguments)]
fn paint_row(ui: &Ui, rect: Rect, resp: &egui::Response, it: &Item, hit: &Hit, thumb: Option<TextureHandle>, selected: bool, compact: bool, terms: &[String], max_sem: f32) {
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let a = accent();
    if selected {
        p.rect_filled(rect, CornerRadius::same(8), a.gamma_multiply(if v.dark_mode { 0.22 } else { 0.12 }));
        p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(0.0, 8.0), Vec2::new(3.0, rect.height() - 16.0)), CornerRadius::same(2), a);
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6));
    }

    let av = if compact { 26.0 } else { 38.0 };
    let av_rect = Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.center().y - av / 2.0), Vec2::splat(av));
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
            let glyph = if code { "</>" } else { kind_icon(it.kind) };
            p.text(av_rect.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(if code { av * 0.32 } else { av * 0.42 }), c);
        }
    }

    let x = av_rect.right() + 12.0;
    let right_w = 64.0;
    let text_w = (rect.right() - x - right_w - 8.0).max(40.0);
    let title_color = v.strong_text_color();
    let weak = v.weak_text_color();
    let sub = if compact { String::new() } else { subtitle(it) };
    let title = row_title(it);
    let title = if title.is_empty() { "(empty)".to_string() } else { title };

    let title_job = highlighted(&title, terms, FontId::proportional(14.0), title_color, text_w);
    let tg = ui.fonts_mut(|f| f.layout_job(title_job));
    let title_y = if sub.is_empty() { rect.center().y - tg.size().y / 2.0 - if compact { 0.0 } else { 8.0 } } else { rect.top() + 10.0 };
    p.galley(Pos2::new(x, title_y), tg, title_color);
    if !sub.is_empty() {
        let sg = ui.fonts_mut(|f| f.layout_job(highlighted(&sub, terms, FontId::proportional(12.5), weak, text_w)));
        p.galley(Pos2::new(x, rect.top() + 32.0), sg, weak);
    }

    // Right edge: time, then pinned star and relevance.
    let rx = rect.right() - 12.0;
    let time_y = if compact || sub.is_empty() { rect.center().y - if compact { 0.0 } else { 8.0 } } else { rect.top() + 18.0 };
    p.text(Pos2::new(rx, time_y), Align2::RIGHT_CENTER, row_time(it.created), FontId::proportional(11.5), weak);
    let mut ix = rx;
    let iy = if compact { rect.center().y } else if sub.is_empty() { rect.center().y + 10.0 } else { rect.top() + 40.0 };
    if compact {
        ix -= 46.0;
    }
    if it.pinned {
        p.text(Pos2::new(ix, iy), Align2::RIGHT_CENTER, "★", FontId::proportional(13.0), a);
        ix -= 18.0;
    }
    if let Some(s) = hit.semantic {
        // Three bars: how close this match is to the best one.
        let ratio = if max_sem > 0.0 { s / max_sem } else { 0.0 };
        let level = if ratio > 0.93 { 3 } else if ratio > 0.8 { 2 } else { 1 };
        for b in 0..3 {
            let h = 4.0 + b as f32 * 3.0;
            let r = Rect::from_min_size(Pos2::new(ix - 18.0 + b as f32 * 6.0, iy + 6.0 - h), Vec2::new(4.0, h));
            p.rect_filled(r, CornerRadius::same(1), if b < level { a } else { weak.gamma_multiply(0.35) });
        }
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
            self.list(ui);
            self.details(ui);
        }
        self.toasts(&ctx);
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("drop")));
            p.rect_filled(r, 0.0, Color32::from_black_alpha(160));
            p.text(r.center(), Align2::CENTER_CENTER, "📂  Drop to add to your searchable library", FontId::proportional(22.0), Color32::WHITE);
        }
    }
}
