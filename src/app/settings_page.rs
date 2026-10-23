//! Settings as a full page with sections. Changes apply as you make them; only
//! the search model waits for "Apply", because a half-typed address would
//! otherwise reconnect on every keystroke.

use eframe::egui::{Frame, Margin, Pos2, Rect, Ui};

use super::icons::{self, Icon};
use super::shortcut;
use super::view::{
    accent, chip, far, flip, lbl, paint_text, primary_button, reveal, row, start_anchor, tip, txt,
    ACCENTS, AMBER, GREEN, RED,
};
use super::*;
use crate::config::{BackendKind, Density, Language, ServerProfiles, TimeFormat};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsPage {
    General,
    Search,
    Clipboard,
    Shortcuts,
    Appearance,
    Data,
}

impl SettingsPage {
    const ALL: [SettingsPage; 6] = [
        Self::General,
        Self::Search,
        Self::Clipboard,
        Self::Shortcuts,
        Self::Appearance,
        Self::Data,
    ];

    fn label(self) -> (Icon, &'static str) {
        match self {
            Self::General => (Icon::General, "General"),
            Self::Search => (Icon::Search, "Search & AI model"),
            Self::Clipboard => (Icon::Clipboard, "Clipboard"),
            Self::Shortcuts => (Icon::Keyboard, "Shortcuts"),
            Self::Appearance => (Icon::Palette, "Appearance"),
            Self::Data => (Icon::Lock, "Data & privacy"),
        }
    }
}

/// Picks one shortcut out of the settings.
type KeyField = fn(&mut Config) -> &mut String;

const HISTORY_SIZES: [usize; 5] = [500, 1_000, 5_000, 20_000, 100_000];
const VECTOR_SIZES: [(usize, &str); 4] = [
    (0, "Full · best quality"),
    (512, "512 · smaller"),
    (256, "256 · compact"),
    (128, "128 · tiny, fastest"),
];
const STRICTNESS: [(f32, &str); 3] = [
    (0.0, "Show all"),
    (0.15, "Related only"),
    (0.3, "Close only"),
];

// ----- widgets ---------------------------------------------------------------------

/// iOS-style switch.
fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let off = ui.visuals().widgets.inactive.bg_fill;
    let bg = Color32::from_rgba_premultiplied(
        (off.r() as f32 + (accent().r() as f32 - off.r() as f32) * t) as u8,
        (off.g() as f32 + (accent().g() as f32 - off.g() as f32) * t) as u8,
        (off.b() as f32 + (accent().b() as f32 - off.b() as f32) * t) as u8,
        255,
    );
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(11), bg);
    // Right to left, "on" is to the left.
    let travel = t * (rect.width() - 22.0);
    let x = if i18n::rtl() {
        rect.right() - 11.0 - travel
    } else {
        rect.left() + 11.0 + travel
    };
    p.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp
}

/// Options side by side; the chosen one is tinted. Called inside a setting's
/// control area, which fills from the far end, so options are added in reverse.
fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    ui.spacing_mut().item_spacing.x = 2.0;
    for (v, label) in options.iter().rev() {
        let sel = *value == *v;
        let text = txt(
            tr(label),
            13.0,
            Some(if sel {
                accent()
            } else {
                ui.visuals().text_color()
            }),
        );
        let b = egui::Button::new(text)
            .frame_when_inactive(sel)
            .fill(if sel {
                accent().gamma_multiply(0.14)
            } else {
                Color32::TRANSPARENT
            });
        if ui.add(b).clicked() && !sel {
            *value = *v;
            changed = true;
        }
    }
    changed
}

fn info(ui: &mut Ui, text: &str) {
    tip(
        icons::glyph(ui, Icon::Info, 14.0, ui.visuals().weak_text_color()),
        tr(text),
    );
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(18.0);
    let weak = ui.visuals().weak_text_color();
    row(ui, |ui| lbl(ui, &tr(title).to_uppercase(), 11.5, weak));
    ui.add_space(4.0);
}

/// One setting: label and ℹ on the left, control at the right edge. A value
/// changed from its default tints the row and offers a reset; the caller must
/// act on the returned click.
#[must_use = "a changed row shows a reset button; act on its click"]
fn setting(
    ui: &mut Ui,
    label: &str,
    help: &str,
    changed: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    setting_row(ui, label, help, changed, changed, control)
}

/// A row with nothing to reset: an action, a path, or a fixed value.
fn plain_setting(ui: &mut Ui, label: &str, help: &str, control: impl FnOnce(&mut Ui)) {
    setting_row(ui, label, help, false, false, control);
}

/// A row that marks the current choice in a list (a built-in model): tinted,
/// but with no reset, since there's no default to go back to.
fn choice_setting(
    ui: &mut Ui,
    label: &str,
    help: &str,
    chosen: bool,
    control: impl FnOnce(&mut Ui),
) {
    setting_row(ui, label, help, chosen, false, control);
}

fn setting_row(
    ui: &mut Ui,
    label: &str,
    help: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    let mut reset = false;
    let fill = if changed {
        accent().gamma_multiply(if ui.visuals().dark_mode { 0.14 } else { 0.07 })
    } else {
        Color32::TRANSPARENT
    };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            row(ui, |ui| {
                ui.set_min_height(30.0);
                let color = if changed {
                    accent()
                } else {
                    ui.visuals().strong_text_color()
                };
                lbl(ui, label, 14.0, color);
                if !help.is_empty() {
                    info(ui, help);
                }
                far(ui, |ui| {
                    // The reset button's space is kept when it's hidden, so controls stay in one column.
                    if resettable {
                        if icons::icon_button_ex(
                            ui,
                            Icon::Reset,
                            "Back to default",
                            Vec2::splat(28.0),
                            14.0,
                            None,
                            None,
                        )
                        .clicked()
                        {
                            reset = true;
                        }
                    } else {
                        ui.add_space(28.0 + ui.spacing().item_spacing.x);
                    }
                    control(ui);
                });
            });
        });
    reset
}

fn note(ui: &mut Ui, text: &str) {
    let weak = ui.visuals().weak_text_color();
    Frame::new()
        .inner_margin(Margin::symmetric(10, 0))
        .show(ui, |ui| {
            super::bidi::paragraph(ui, tr(text), FontId::proportional(12.5), weak);
        });
}

fn path_field(ui: &mut Ui, p: &mut PathBuf, ext: &[&str]) {
    if ui.button(txt(tr("Browse…"), 14.0, None)).clicked() {
        let mut dlg = rfd::FileDialog::new();
        if !ext.is_empty() {
            dlg = dlg.add_filter("file", ext).add_filter("all", &["*"]);
        }
        if let Some(f) = dlg.pick_file() {
            *p = f;
        }
    }
    let mut s = p.to_string_lossy().into_owned();
    if ui
        .add(egui::TextEdit::singleline(&mut s).desired_width(280.0))
        .changed()
    {
        *p = PathBuf::from(&s);
    }
}

// ----- the page --------------------------------------------------------------------

impl App {
    pub(super) fn open_settings(&mut self, page: SettingsPage) {
        self.draft = Some(self.cfg.clone());
        self.settings_page = page;
        self.recording = None;
        self.confirm = None;
    }

    /// Leaving settings applies everything, including model changes.
    pub(super) fn close_settings(&mut self, ctx: &egui::Context) {
        if let Some(d) = self.draft.take() {
            self.apply_settings(d, ctx, true);
        }
        self.recording = None;
        if self.hotkeys_paused {
            ctx.request_repaint();
        }
        self.confirm = None;
    }

    fn apply_settings(&mut self, d: Config, ctx: &egui::Context, prune: bool) {
        if d == self.cfg && !prune {
            return;
        }
        let backend_changed = d.backend != self.cfg.backend;
        let keys_changed = d.show_hotkey != self.cfg.show_hotkey
            || d.ignore_next_hotkey != self.cfg.ignore_next_hotkey;
        if d.ocr_images && !self.cfg.ocr_images {
            for it in &self.items {
                if it.kind == Kind::Image && it.ocr.is_none() {
                    self.worker.send(Job::Ocr(it.id));
                }
            }
        }
        let tray_changed = d.tray != self.cfg.tray;
        let login_changed = d.start_at_login != self.cfg.start_at_login;
        let look_changed = d.accent != self.cfg.accent || d.theme != self.cfg.theme;
        let language_changed = d.language != self.cfg.language;
        i18n::set(d.language, d.time_format);
        let search_changed =
            d.search_mode != self.cfg.search_mode || d.min_similarity != self.cfg.min_similarity;
        self.cfg = d;
        self.flags
            .enabled
            .store(self.cfg.monitor_clipboard, Ordering::SeqCst);
        self.flags
            .skip_secrets
            .store(self.cfg.skip_secrets, Ordering::SeqCst);
        if look_changed {
            set_accent(self.cfg.accent);
            style(ctx);
            apply_theme(ctx, self.cfg.theme);
        }
        self.save_config();
        if backend_changed {
            self.set_backend(self.cfg.backend.clone());
        }
        if keys_changed {
            self.register_hotkeys(ctx);
        }
        // The tray menu is built once, so a new language rebuilds it.
        if tray_changed || (language_changed && self.cfg.tray) {
            self.set_tray(self.cfg.tray);
        }
        if language_changed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(tr("Clipboard Manager").into()));
        }
        if login_changed {
            if let Err(e) = crate::autostart::set(self.cfg.start_at_login) {
                self.set_error(format!("{e:#}"));
                self.cfg.start_at_login = !self.cfg.start_at_login;
                self.save_config();
            }
        }
        if search_changed {
            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
        }
        if prune {
            self.prune();
        }
        self.dirty = true;
    }

    pub(super) fn settings_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let Some(mut d) = self.draft.take() else {
            return;
        };
        let mut close = false;
        let mut apply_model = false;
        let mut apply_keys = false;
        let model_pending = d.backend != self.cfg.backend;

        let panel = ui.visuals().panel_fill;
        egui::Panel::top("settings_top")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 12,
                        right: 16,
                        top: 12,
                        bottom: 8,
                    })
                    .fill(panel),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    if icons::icon_button_ex(
                        ui,
                        Icon::Back,
                        "Back (Esc)",
                        Vec2::splat(34.0),
                        22.0,
                        None,
                        None,
                    )
                    .clicked()
                    {
                        close = true;
                    }
                    ui.add_space(8.0);
                    let strong = ui.visuals().strong_text_color();
                    lbl(ui, "Settings", 20.0, strong);
                    far(ui, |ui| {
                        if model_pending {
                            if primary_button(ui, "Apply model").clicked() {
                                apply_model = true;
                            }
                            chip(ui, tr("Model changes not applied yet"), AMBER);
                        } else {
                            let weak = ui.visuals().weak_text_color();
                            lbl(ui, "Changes are saved automatically", 12.5, weak);
                            icons::glyph(ui, Icon::Check, 13.0, weak);
                        }
                    });
                });
            });

        // The page list sits on the reading-start side.
        let nav = if i18n::rtl() {
            egui::Panel::right("settings_nav")
        } else {
            egui::Panel::left("settings_nav")
        };
        nav.resizable(false)
            .exact_size(220.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin::symmetric(10, 8))
                    .fill(panel),
            )
            .show(ui, |ui| {
                for page in SettingsPage::ALL {
                    let (icon, name) = page.label();
                    let sel = self.settings_page == page;
                    let (rect, r) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
                    let v = ui.visuals();
                    let color = if sel {
                        accent()
                    } else if r.hovered() {
                        v.strong_text_color()
                    } else {
                        v.text_color()
                    };
                    if sel {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(8),
                            accent().gamma_multiply(0.13),
                        );
                    } else if r.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(8),
                            v.widgets.hovered.weak_bg_fill,
                        );
                    }
                    let icon_rect = flip(
                        rect,
                        Rect::from_center_size(
                            Pos2::new(rect.left() + 22.0, rect.center().y),
                            Vec2::splat(16.0),
                        ),
                    );
                    icons::paint(ui.painter(), icon_rect, icon, color);
                    let at = flip(
                        rect,
                        Rect::from_min_size(
                            Pos2::new(rect.left() + 42.0, rect.center().y),
                            Vec2::ZERO,
                        ),
                    )
                    .min;
                    paint_text(
                        ui,
                        ui.painter(),
                        at,
                        start_anchor(egui::Align::Center),
                        tr(name),
                        FontId::proportional(14.0),
                        color,
                    );
                    if r.clicked() {
                        self.settings_page = page;
                        self.confirm = None;
                        self.recording = None;
                    }
                }
            });

        let content = ui.visuals().extreme_bg_color;
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 28,
                        right: 20,
                        top: 8,
                        bottom: 16,
                    })
                    .fill(content),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // A readable column, next to the page list (on the right in Arabic).
                        let area = ui.available_rect_before_wrap();
                        let w = area.width().min(720.0);
                        let x = if i18n::rtl() {
                            area.right() - w
                        } else {
                            area.left()
                        };
                        let column = Rect::from_min_size(
                            Pos2::new(x, area.top()),
                            Vec2::new(w, area.height()),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| match self
                            .settings_page
                        {
                            SettingsPage::General => self.general_page(ui, &mut d),
                            SettingsPage::Search => self.search_page(ui, &mut d, &mut apply_model),
                            SettingsPage::Clipboard => clipboard_page(ui, &mut d),
                            SettingsPage::Shortcuts => {
                                self.shortcuts_page(ui, &mut d, &mut apply_keys)
                            }
                            SettingsPage::Appearance => appearance_page(ui, &mut d),
                            SettingsPage::Data => self.data_page(ui, &mut d),
                        });
                        ui.add_space(24.0);
                    });
            });

        if close {
            self.draft = Some(d);
            self.close_settings(&ctx);
            return;
        }
        // Everything except the model and hotkeys applies immediately.
        let mut live = d.clone();
        live.backend = self.cfg.backend.clone();
        live.show_hotkey = self.cfg.show_hotkey.clone();
        live.ignore_next_hotkey = self.cfg.ignore_next_hotkey.clone();
        if live != self.cfg {
            self.apply_settings(live, &ctx, false);
            // A setting the system refused (start at sign-in) snaps back.
            d.start_at_login = self.cfg.start_at_login;
        }
        if apply_keys {
            let mut c = self.cfg.clone();
            c.show_hotkey = d.show_hotkey.clone();
            c.ignore_next_hotkey = d.ignore_next_hotkey.clone();
            self.apply_settings(c, &ctx, false);
        }
        if apply_model {
            let mut c = self.cfg.clone();
            c.backend = d.backend.clone();
            self.apply_settings(c, &ctx, false);
        }
        // App::logic turns the global shortcuts back on in the next frame.
        if self.hotkeys_paused && self.recording.is_none() {
            ctx.request_repaint();
        }
        self.draft = Some(d);
    }

    fn search_page(&mut self, ui: &mut Ui, d: &mut Config, apply_model: &mut bool) {
        let def = Config::default();
        section(ui, "Search");
        if setting(ui, "How search works", "Smart: finds items that contain your words and items that mean the same thing, merged into one list.\nMeaning: only by meaning (needs a search model).\nExact words: only items containing what you type.", d.search_mode != def.search_mode, |ui| {
            segmented(ui, &mut d.search_mode, &[(SearchMode::Hybrid, "Smart"), (SearchMode::Semantic, "Meaning"), (SearchMode::Keyword, "Exact words")]);
        }) {
            d.search_mode = def.search_mode;
        }
        if setting(ui, "Meaning matches to show", "How closely an item has to match before it's listed. \"Show all\" ranks everything by closeness; stricter options hide weak matches. Exact word matches are always shown.", d.min_similarity != def.min_similarity, |ui| {
            let current = STRICTNESS.iter().find(|(v, _)| (*v - d.min_similarity).abs() < 1e-4).map(|(_, l)| tr(l).to_string()).unwrap_or_else(|| trf("Custom ({v})", &[("v", &format!("{:.2}", d.min_similarity))]));
            egui::ComboBox::from_id_salt("strict").selected_text(txt(&current, 14.0, None)).width(150.0).show_ui(ui, |ui| {
                for (v, l) in STRICTNESS {
                    ui.selectable_value(&mut d.min_similarity, v, txt(tr(l), 14.0, None));
                }
            });
        }) {
            d.min_similarity = def.min_similarity;
        }

        section(ui, "Meaning search model");
        let (dot, text) = self.model_status();
        row(ui, |ui| {
            ui.add_space(10.0);
            let color = if dot == Color32::GRAY {
                ui.visuals().weak_text_color()
            } else {
                dot
            };
            icons::glyph(ui, Icon::Dot, 12.0, color);
            ui.label(txt(&text, 13.0, Some(color)));
        });
        ui.add_space(4.0);
        let mut kind = d.backend.kind();
        plain_setting(ui, "Model", "Off: search by words only (plus text found in images).\nBuilt-in: a model inside this app that understands text and images, no other software needed (one-time download).\nYour server: use a model you already run, such as llama.cpp, Unsloth, Sentence Transformers, Ollama or LM Studio.", |ui| {
            segmented(ui, &mut kind, &[(BackendKind::Off, "Off"), (BackendKind::BuiltIn, "Built-in"), (BackendKind::Server, "Your server")]);
        });
        d.choose_backend(kind);
        match &mut d.backend {
            EmbedBackend::None => note(ui, "Searching by words only. Turn on a model to also find items by meaning, e.g. “trip” finds “flight to Jeddah”."),
            EmbedBackend::SigLip { repo } => self.builtin_models(ui, repo),
            EmbedBackend::Server(s) => {
                server_settings(ui, s, &mut d.server_profiles, &mut self.model_list)
            }
        }
        if d.backend != self.cfg.backend {
            ui.add_space(8.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, "Apply model").clicked() {
                    *apply_model = true;
                }
                lbl(
                    ui,
                    "Your items are indexed again in the background.",
                    12.5,
                    ui.visuals().weak_text_color(),
                );
            });
        }
    }

    fn builtin_models(&mut self, ui: &mut Ui, repo: &mut String) {
        note(ui, "Runs on this PC's processor. Finds photos and screenshots from a description, like “receipt from a restaurant”.");
        let mut start: Option<String> = None;
        for m in &SIGLIP_MODELS {
            let ready = models::siglip_ready(&self.paths.models, m.repo);
            let selected = *repo == m.repo;
            let size = format!("{:.1}", m.size_mb as f32 / 1000.0);
            choice_setting(
                ui,
                m.label,
                &trf(
                    "{repo} · {size} GB download",
                    &[("repo", &m.repo), ("size", &size)],
                ),
                selected,
                |ui| {
                    if ready {
                        chip(ui, "✔ Downloaded", GREEN);
                    } else if let Some(dl) = self.download.as_ref().filter(|x| x.repo == m.repo) {
                        let p = &dl.progress;
                        if ui.small_button(txt(tr("Cancel"), 12.0, None)).clicked() {
                            p.cancel.store(true, Ordering::Relaxed);
                        }
                        let (done, total) = (
                            p.done.load(Ordering::Relaxed),
                            p.total.load(Ordering::Relaxed).max(1),
                        );
                        ui.add(
                            egui::ProgressBar::new(done as f32 / total as f32)
                                .desired_width(150.0)
                                .text(format!("{} / {} MB", done >> 20, total >> 20)),
                        );
                        ui.ctx().request_repaint_after(Duration::from_millis(250));
                    } else if self.download.is_none()
                        && ui
                            .button(txt(
                                &trf("Download {size} GB", &[("size", &size)]),
                                14.0,
                                None,
                            ))
                            .clicked()
                    {
                        start = Some(m.repo.to_string());
                    }
                    if !selected
                        && ui
                            .add(
                                egui::Button::new(txt(tr("Use"), 14.0, None))
                                    .frame_when_inactive(false),
                            )
                            .clicked()
                    {
                        *repo = m.repo.to_string();
                    }
                },
            );
        }
        if let Some(r) = start {
            *repo = r.clone();
            let progress = Arc::new(Progress::default());
            let (p, dir, rr, c) = (
                progress.clone(),
                self.paths.models.clone(),
                r.clone(),
                ui.ctx().clone(),
            );
            let handle = std::thread::spawn(move || {
                let out = models::download_siglip(&dir, &rr, &p);
                c.request_repaint();
                out
            });
            self.download = Some(Download {
                repo: r,
                progress,
                handle,
            });
        }
    }

    fn shortcuts_page(&mut self, ui: &mut Ui, d: &mut Config, apply_keys: &mut bool) {
        let def = Config::default();
        section(ui, "Anywhere in Windows");
        let globals: [(&'static str, &str, &str, KeyField); 2] = [
            (
                "show",
                "Show or hide this window",
                "Opens the window over whatever you're doing; press again to hide it.",
                |c| &mut c.show_hotkey,
            ),
            (
                "ignore",
                "Don't record the next copy",
                "Press before copying something private: the next copy is skipped.",
                |c| &mut c.ignore_next_hotkey,
            ),
        ];
        for (id, label, help, field) in globals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let invalid = !value.trim().is_empty() && HotKey::from_str(value.trim()).is_err();
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, label, help, *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    true,
                    &mut self.record_problem,
                );
                if invalid {
                    chip(ui, "⚠ Not usable", RED);
                }
            });
            if reset {
                *value = default;
                *apply_keys = true;
            }
            match result {
                shortcut::Recorded::Started => {
                    // Off while recording, so pressing the current keys doesn't hide the window.
                    self.hotkeys = None;
                    self.hotkeys_paused = true;
                    self.recording = Some(id);
                }
                shortcut::Recorded::Changed | shortcut::Recorded::Cancelled => {
                    self.recording = None;
                    *apply_keys = true;
                }
                shortcut::Recorded::Nothing => {}
            }
        }
        if let Some(e) = &self.hotkey_error {
            note(ui, &format!("⚠ {e}"));
        }

        section(ui, "Inside the app");
        let locals: [(&'static str, &str, KeyField); 5] = [
            ("copy", "Copy the selected item and hide the window", |c| {
                &mut c.keys.copy
            }),
            ("pin", "Pin or unpin", |c| &mut c.keys.pin),
            ("delete", "Delete", |c| &mut c.keys.delete),
            ("tag", "Add a tag", |c| &mut c.keys.tag),
            ("view", "Switch between Grid, List and Detail", |c| {
                &mut c.keys.view
            }),
        ];
        let current: Vec<(&str, String)> = locals
            .iter()
            .map(|(id, _, f)| (*id, f(&mut d.clone()).clone()))
            .collect();
        for (id, label, field) in locals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let clash = !value.is_empty()
                && current
                    .iter()
                    .any(|(other, v)| *other != id && v.eq_ignore_ascii_case(value));
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, label, "", *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    false,
                    &mut self.record_problem,
                );
                if clash {
                    chip(ui, "Used twice", AMBER);
                }
            });
            if reset {
                *value = default;
            }
            match result {
                shortcut::Recorded::Started => self.recording = Some(id),
                shortcut::Recorded::Changed | shortcut::Recorded::Cancelled => {
                    self.recording = None
                }
                shortcut::Recorded::Nothing => {}
            }
        }
        for (label, keys) in [
            ("Move through the list", "Up+Down"),
            ("Move between cards in the grid", "Left+Right"),
            ("Close details, clear the search, or hide", "Escape"),
        ] {
            plain_setting(ui, label, "Built in; can't be changed.", |ui| {
                shortcut::fixed(ui, keys)
            });
        }
        note(
            ui,
            "Click a shortcut, then press the new keys. Backspace turns it off, Esc cancels.",
        );
    }

    fn data_page(&mut self, ui: &mut Ui, d: &mut Config) {
        section(ui, "Storage");
        let root = self.paths.root.clone();
        plain_setting(
            ui,
            "Data folder",
            "History, images, models and settings are stored here, on this PC only.",
            |ui| {
                if ui.button(txt(tr("Open"), 14.0, None)).clicked() {
                    reveal(&root);
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(root.display().to_string())
                            .size(12.0)
                            .color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
            },
        );
        let log = root.join("clipboard-manager.log");
        plain_setting(
            ui,
            "Log file",
            "A record of what the app did in this session. Attach it when reporting a problem.",
            |ui| {
                if ui.button(txt(tr("Show"), 14.0, None)).clicked() {
                    reveal(&log);
                }
            },
        );
        let images = self.items.iter().filter(|i| i.kind == Kind::Image).count();
        let pinned = self.items.iter().filter(|i| i.pinned).count();
        let clip = self
            .items
            .iter()
            .filter(|i| i.origin == Origin::Clipboard && !i.pinned)
            .count();
        plain_setting(ui, "Stored", "", |ui| {
            ui.label(txt(
                &trf(
                    "{n} items · {i} images · {p} pinned",
                    &[("n", &self.items.len()), ("i", &images), ("p", &pinned)],
                ),
                13.0,
                None,
            ));
        });

        section(ui, "Clean up");
        let mut clear = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            "Clear clipboard history",
            "Deletes copied items. Pinned and tagged items, and files you added, stay.",
            |ui| {
                if confirm == Some("clear") {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                &trf("Delete {n} items", &[("n", &clip)]),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        clear = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        clip > 0,
                        egui::Button::new(txt(tr("Clear…"), 14.0, Some(RED))),
                    )
                    .clicked()
                {
                    self.confirm = Some("clear");
                }
            },
        );
        if clear {
            self.confirm = None;
            self.clear_history();
        }
        let mut reset = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            "Reset all settings",
            "Puts every setting back to its default, including the search model.",
            |ui| {
                if confirm == Some("reset") {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                tr("Reset everything"),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        reset = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        *d != Config::default(),
                        egui::Button::new(txt(tr("Reset…"), 14.0, None)),
                    )
                    .clicked()
                {
                    self.confirm = Some("reset");
                }
            },
        );
        if reset {
            self.confirm = None;
            *d = Config::default();
        }
        note(ui, "Nothing you copy is sent anywhere. Search models run on this PC or on a server you choose. The update check only downloads the version file from GitHub.");
    }
}

impl App {
    fn general_page(&mut self, ui: &mut Ui, d: &mut Config) {
        general_settings(ui, d);
        self.updates_section(ui, d);
    }

    fn updates_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        section(ui, "Updates");
        let release_build = crate::update::build().is_some();
        if setting(ui, "Check for updates automatically", "Looks for a new version when the app starts and once a day. Only the version file on GitHub is fetched; nothing about you or your clipboard is sent.", d.update_check != def.update_check, |ui| {
            toggle(ui, &mut d.update_check);
        }) {
            d.update_check = def.update_check;
        }
        if setting(ui, "Download updates automatically", "On: a new version downloads in the background and installs when you restart or quit. Off: you're asked before anything is downloaded.", d.update_download != def.update_download, |ui| {
            toggle(ui, &mut d.update_download);
        }) {
            d.update_download = def.update_download;
        }
        let mut check = false;
        plain_setting(
            ui,
            "Version",
            "Every download is checked against this app's signing key before it's installed.",
            |ui| {
                if release_build
                    && ui
                        .add_enabled(
                            self.update_rx.is_none(),
                            egui::Button::new(txt(tr("Check now"), 13.0, None)),
                        )
                        .clicked()
                {
                    check = true;
                }
                let weak = ui.visuals().weak_text_color();
                let status = match &self.update {
                    UpdateState::Checking => tr("Checking…").to_string(),
                    UpdateState::UpToDate => tr("You're up to date").to_string(),
                    UpdateState::Available(r) => {
                        trf("Version {version} is available", &[("version", &r.label())])
                    }
                    UpdateState::Downloading(r, _) => {
                        trf("Downloading version {version}…", &[("version", &r.label())])
                    }
                    UpdateState::Ready(r, _) => {
                        trf("Version {version} is ready", &[("version", &r.label())])
                    }
                    UpdateState::Failed { error, .. } => {
                        trf("Update failed: {error}", &[("error", error)])
                    }
                    UpdateState::Idle if !release_build => {
                        tr("Updates are off in development builds").to_string()
                    }
                    UpdateState::Idle => String::new(),
                };
                if !status.is_empty() {
                    ui.add(egui::Label::new(txt(&status, 12.5, Some(weak))).truncate());
                }
                ui.label(txt(&crate::update::current_label(), 13.0, None));
            },
        );
        if check {
            let ctx = ui.ctx().clone();
            self.update_hidden = false;
            self.check_for_update(true, &ctx);
        }
    }
}

fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Language and time");
    // Each language is named in itself, so it can be found whatever is showing.
    if setting(
        ui,
        "Language",
        "The language of the app. Arabic also lays the window out right to left.",
        d.language != def.language,
        |ui| {
            segmented(
                ui,
                &mut d.language,
                &[
                    (Language::English, "English"),
                    (Language::Arabic, "العربية"),
                ],
            );
        },
    ) {
        d.language = def.language;
    }
    if setting(
        ui,
        "Time format",
        "How times are shown. They're stored in UTC and shown in your time zone.",
        d.time_format != def.time_format,
        |ui| {
            segmented(
                ui,
                &mut d.time_format,
                &[(TimeFormat::H24, "24-hour"), (TimeFormat::H12, "12-hour")],
            );
        },
    ) {
        d.time_format = def.time_format;
    }
    section(ui, "Startup");
    if setting(ui, "Start when you sign in", "Opens the app each time you sign in to Windows. It starts hidden: in the system tray when that's on, otherwise minimised.", d.start_at_login != def.start_at_login, |ui| {
        toggle(ui, &mut d.start_at_login);
    }) {
        d.start_at_login = def.start_at_login;
    }
    section(ui, "System tray");
    if setting(ui, "Keep running in the system tray", "Shows an icon in the notification area. Closing or minimising the window hides it there and history keeps recording. Click the icon to open; right-click it to quit.", d.tray != def.tray, |ui| {
        toggle(ui, &mut d.tray);
    }) {
        d.tray = def.tray;
    }
    note(
        ui,
        "With both on, the app starts quietly in the tray each time you sign in.",
    );
}

fn clipboard_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Recording");
    let flag = |ui: &mut Ui, label: &str, help: &str, v: &mut bool, default: bool| {
        if setting(ui, label, help, *v != default, |ui| {
            toggle(ui, v);
        }) {
            *v = default;
        }
    };
    flag(ui, "Record clipboard history", "Save everything you copy. Turn off to pause; the pause button in the main window does the same.", &mut d.monitor_clipboard, def.monitor_clipboard);
    flag(ui, "Skip passwords and API keys", "Don't save copies that look like a password, API key, token or private key. You still can paste them normally.", &mut d.skip_secrets, def.skip_secrets);
    flag(
        ui,
        "Merge duplicate copies",
        "Copying something you already have moves it to the top instead of adding it again.",
        &mut d.dedupe,
        def.dedupe,
    );
    flag(
        ui,
        "Read text in images",
        "Find screenshots and photos by the words in them. Runs on this PC.",
        &mut d.ocr_images,
        def.ocr_images,
    );
    section(ui, "After copying from the list");
    flag(
        ui,
        "Hide the window",
        "After you copy an item, the window hides so you can paste right away.",
        &mut d.minimize_after_copy,
        def.minimize_after_copy,
    );
    section(ui, "History size");
    if setting(ui, "Keep up to", "When there are more, the oldest are deleted. Pinned and tagged items, and files you added, are never deleted.", d.max_items != def.max_items, |ui| {
        let label = |n: usize| trf("{n} items", &[("n", &n.to_string().as_bytes().rchunks(3).rev().map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(","))]);
        egui::ComboBox::from_id_salt("history").selected_text(txt(&label(d.max_items), 14.0, None)).width(140.0).show_ui(ui, |ui| {
            for n in HISTORY_SIZES {
                ui.selectable_value(&mut d.max_items, n, txt(&label(n), 14.0, None));
            }
        });
    }) {
        d.max_items = def.max_items;
    }
}

fn appearance_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Look");
    if setting(
        ui,
        "Theme",
        "System follows your Windows light/dark setting.",
        d.theme != def.theme,
        |ui| {
            segmented(
                ui,
                &mut d.theme,
                &[
                    (Theme::System, "System"),
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                ],
            );
        },
    ) {
        d.theme = def.theme;
    }
    if setting(
        ui,
        "Accent colour",
        "Used for selection, buttons and highlights.",
        d.accent != def.accent,
        |ui| {
            for (i, (name, c)) in ACCENTS.iter().enumerate().rev() {
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                ui.painter().circle_filled(rect.center(), 10.0, *c);
                if d.accent == i {
                    ui.painter()
                        .circle_stroke(rect.center(), 12.0, Stroke::new(2.0, *c));
                }
                if tip(r, tr(name)).clicked() {
                    d.accent = i;
                }
            }
        },
    ) {
        d.accent = def.accent;
    }
    section(ui, "List");
    if setting(ui, "Layout", "Grid shows cards, List shows rows; both open details on demand. Detail keeps the selected item beside the list. The view button next to the search box switches too.", d.view != def.view, |ui| {
        let options: Vec<(ViewMode, &str)> = ViewMode::ALL.iter().map(|v| (*v, v.label())).collect();
        segmented(ui, &mut d.view, &options);
    }) {
        d.view = def.view;
    }
    if setting(
        ui,
        "Density",
        "Compact fits more items on screen by hiding the second line.",
        d.density != def.density,
        |ui| {
            segmented(
                ui,
                &mut d.density,
                &[
                    (Density::Comfortable, "Comfortable"),
                    (Density::Compact, "Compact"),
                ],
            );
        },
    ) {
        d.density = def.density;
    }
    if setting(ui, "Pinned on top", "Pinned items come first while you browse, whatever the order. Search results are ordered by your sort choice only.", d.pinned_first != def.pinned_first, |ui| {
        toggle(ui, &mut d.pinned_first);
    }) {
        d.pinned_first = def.pinned_first;
    }
    if setting(
        ui,
        "Group by day",
        "Show Pinned, Today, Yesterday… headers while browsing (not while searching).",
        d.group_by_date != def.group_by_date,
        |ui| {
            toggle(ui, &mut d.group_by_date);
        },
    ) {
        d.group_by_date = def.group_by_date;
    }
}

fn server_settings(
    ui: &mut Ui,
    s: &mut ServerConfig,
    profiles: &mut ServerProfiles,
    list: &mut ModelList,
) {
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        lbl(ui, "Quick setup:", 12.5, ui.visuals().weak_text_color());
        for (name, url) in [
            ("llama.cpp", "http://127.0.0.1:8080/v1"),
            ("Unsloth", "http://127.0.0.1:8888/v1"),
            ("Sentence Transformers", "http://127.0.0.1:8001/v1"),
            ("Ollama", "http://127.0.0.1:11434/v1"),
            ("LM Studio", "http://127.0.0.1:1234/v1"),
        ] {
            let current = s.base_url.trim_end_matches('/') == url;
            let b = egui::Button::new(RichText::new(name).size(12.5).color(if current {
                accent()
            } else {
                ui.visuals().text_color()
            }))
            .frame_when_inactive(current)
            .fill(if current {
                accent().gamma_multiply(0.13)
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(CornerRadius::same(12));
            if ui.add(b).on_hover_text(url).clicked() {
                profiles.switch_to(s, url);
            }
        }
    });

    list.poll();
    let editable = s.launch.is_none();
    if editable
        && list.pending.is_none()
        && list.url != ModelList::key(s)
        && !s.base_url.trim().is_empty()
    {
        list.fetch(s, ui.ctx());
    }
    plain_setting(
        ui,
        "Server address",
        "The address of the embeddings server, usually ending in /v1.",
        |ui| {
            if editable
                && icons::icon_button_ex(
                    ui,
                    Icon::Refresh,
                    "Check again",
                    Vec2::splat(28.0),
                    16.0,
                    None,
                    None,
                )
                .clicked()
            {
                list.fetch(s, ui.ctx());
            }
            ui.add_enabled(
                editable,
                egui::TextEdit::singleline(&mut s.base_url)
                    .hint_text("http://127.0.0.1:8080/v1")
                    .desired_width(300.0),
            );
        },
    );
    plain_setting(
        ui,
        "API key",
        "Only if your server asks for one. Sent as a Bearer token and stored only on this PC.",
        |ui| {
            let (eye, label) = if list.show_key {
                (Icon::EyeOff, "Hide key")
            } else {
                (Icon::Eye, "Show key")
            };
            if icons::icon_button_ex(ui, eye, label, Vec2::splat(28.0), 16.0, None, None).clicked()
            {
                list.show_key = !list.show_key;
            }
            ui.add(
                egui::TextEdit::singleline(&mut s.api_key)
                    .password(!list.show_key)
                    .hint_text("optional")
                    .desired_width(300.0),
            );
        },
    );
    let default_name = list.found.default.clone();
    let reset_model = setting(ui, "Model", "Leave empty to use the server's default embedding model. Pick from ▾ or type a name your server knows.", !s.model.trim().is_empty(), |ui| {
        let any = default_name.is_some() || !list.found.listed.is_empty();
        if editable && any {
            let b = icons::icon_button_ex(ui, Icon::Dropdown, "Models on this server", Vec2::splat(28.0), 11.0, None, None);
            egui::Popup::menu(&b).show(|ui| {
                ui.set_min_width(320.0);
                if let Some(dn) = &default_name {
                    let label = if dn.is_empty() { tr("Server default").to_string() } else { trf("Server default · {model}", &[("model", dn)]) };
                    if ui.selectable_label(s.model.is_empty(), txt(&label, 14.0, None)).clicked() {
                        s.model.clear();
                    }
                }
                if !list.found.listed.is_empty() {
                    lbl(ui, "LISTED BY THE SERVER (MAY INCLUDE CHAT MODELS)", 10.5, ui.visuals().weak_text_color());
                    for m in &list.found.listed {
                        if ui.selectable_label(s.model == *m, m).clicked() {
                            s.model = m.clone();
                        }
                    }
                }
            });
        }
        let hint = match default_name.as_deref() {
            Some(dn) if !dn.is_empty() => trf("default: {model}", &[("model", &dn)]),
            _ => tr("server default").to_string(),
        };
        ui.add_enabled(editable, egui::TextEdit::singleline(&mut s.model).hint_text(hint).desired_width(300.0));
    });
    if reset_model {
        s.model.clear();
    }
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        if list.pending.is_some() {
            ui.add(egui::Spinner::new().size(12.0));
            lbl(
                ui,
                "Checking the server…",
                12.5,
                ui.visuals().weak_text_color(),
            );
        } else if let Some(e) = &list.error {
            chip(ui, format!("⚠ {e}"), RED);
        } else if s.model.trim().is_empty() {
            match default_name.as_deref() {
                Some(dn) => {
                    let model = if dn.is_empty() {
                        tr("the default model")
                    } else {
                        dn
                    };
                    chip(
                        ui,
                        trf("✔ Connected · uses {model}", &[("model", &model)]),
                        GREEN,
                    );
                }
                None if !list.found.listed.is_empty() => {
                    chip(
                        ui,
                        "Connected, but no default embedding model: choose one",
                        AMBER,
                    );
                }
                None => {}
            }
        } else if default_name.is_some() || !list.found.listed.is_empty() {
            chip(ui, "✔ Connected", GREEN);
        }
        if !editable {
            lbl(
                ui,
                "The app starts llama-server itself; it connects after Apply.",
                12.5,
                ui.visuals().weak_text_color(),
            );
        }
    });

    let effective = if s.model.is_empty() {
        default_name.clone().unwrap_or_default()
    } else {
        s.model.clone()
    };
    let gemma = effective.to_lowercase().contains("embeddinggemma");
    let gemma_prompts = s.query_prefix == ServerConfig::GEMMA_QUERY
        && s.document_prefix == ServerConfig::GEMMA_DOCUMENT;
    if gemma && !gemma_prompts {
        row(ui, |ui| {
            ui.add_space(10.0);
            chip(
                ui,
                "💡 EmbeddingGemma searches better with its own prompts",
                AMBER,
            );
            if ui.button(txt(tr("Use them"), 14.0, None)).clicked() {
                s.query_prefix = ServerConfig::GEMMA_QUERY.into();
                s.document_prefix = ServerConfig::GEMMA_DOCUMENT.into();
            }
        });
    }

    ui.add_space(6.0);
    egui::CollapsingHeader::new(txt(tr("Advanced"), 13.5, None)).id_salt("advanced").show(ui, |ui| {
        if setting(ui, "Query prompt", "Text put before what you search for. Some models need it (EmbeddingGemma: “task: search result | query: ”).", !s.query_prefix.is_empty(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.query_prefix).hint_text("none").desired_width(300.0));
        }) {
            s.query_prefix.clear();
        }
        if setting(ui, "Item prompt", "Text put before each item when it's indexed (EmbeddingGemma: “title: none | text: ”).", !s.document_prefix.is_empty(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.document_prefix).hint_text("none").desired_width(300.0));
        }) {
            s.document_prefix.clear();
        }
        if setting(ui, "Vector size", "Smaller vectors use less memory and search faster, with slightly lower quality. Works with Matryoshka models such as EmbeddingGemma and UltraVBERT.", s.truncate_dims != 0, |ui| {
            let current = VECTOR_SIZES.iter().find(|(v, _)| *v == s.truncate_dims).map(|(_, l)| l.to_string()).unwrap_or_else(|| format!("{} dims", s.truncate_dims));
            egui::ComboBox::from_id_salt("dims").selected_text(current).width(170.0).show_ui(ui, |ui| {
                for (v, l) in VECTOR_SIZES {
                    ui.selectable_value(&mut s.truncate_dims, v, l);
                }
            });
        }) {
            s.truncate_dims = 0;
        }
        if setting(ui, "Search images with this model", "Send images to the server too. Only for models that understand images (CLIP, SigLIP, BiUltraVBERT…). Otherwise images are found by the text in them.", s.images, |ui| {
            toggle(ui, &mut s.images);
        }) {
            s.images = false;
        }
        let mut launch = s.launch.is_some();
        if setting(ui, "Start llama-server for me", "The app starts llama-server with your .gguf file in the background and stops it when you quit.", launch, |ui| {
            toggle(ui, &mut launch);
        }) {
            launch = false;
        }
        if launch && s.launch.is_none() {
            s.launch = Some(LaunchConfig {
                llama_server: PathBuf::from(if cfg!(windows) { "llama-server.exe" } else { "llama-server" }),
                gguf: PathBuf::new(),
                mmproj: None,
                port: 8765,
                extra_args: "--pooling mean -c 2048 -ub 2048".into(),
            });
        } else if !launch {
            s.launch = None;
        }
        if let Some(l) = &mut s.launch {
            plain_setting(ui, "llama-server program", "llama-server.exe from a llama.cpp release.", |ui| path_field(ui, &mut l.llama_server, &["exe"]));
            plain_setting(ui, "Model file (.gguf)", "", |ui| path_field(ui, &mut l.gguf, &["gguf"]));
            plain_setting(ui, "Image projector (.gguf)", "Optional mmproj file, for models that embed images.", |ui| {
                let mut p = l.mmproj.clone().unwrap_or_default();
                path_field(ui, &mut p, &["gguf"]);
                l.mmproj = if p.as_os_str().is_empty() { None } else { Some(p) };
            });
            plain_setting(ui, "Port", "", |ui| {
                ui.add(egui::DragValue::new(&mut l.port).range(1024..=65535));
            });
            plain_setting(ui, "Extra arguments", "Passed to llama-server as-is.", |ui| {
                ui.add(egui::TextEdit::singleline(&mut l.extra_args).desired_width(300.0));
            });
        }
    });
}
