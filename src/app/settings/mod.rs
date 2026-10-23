//! Settings as a full page with sections. Changes apply as you make them; only
//! the search model waits for "Apply", because a half-typed address would
//! otherwise reconnect on every keystroke.

use eframe::egui::{Frame, Margin, Pos2, Rect, Ui};

use super::icons::{self, Icon};
use super::shortcut;
use super::view::{
    accent, accent_name, chip, far, flip, lbl, paint_text, primary_button, reveal, row,
    start_anchor, tip, txt, ACCENTS, AMBER, GREEN, RED,
};
use super::*;
use crate::config::{BackendKind, Density, Language, ServerProfiles, TimeFormat};

mod appearance;
mod clipboard;
mod data;
mod general;
mod search;
mod server;
mod shortcuts;
mod widgets;

// The helpers of every page, for this file and the pages.
use appearance::*;
use clipboard::*;
use server::*;
use widgets::*;

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

    fn label(self) -> (Icon, String) {
        match self {
            Self::General => (Icon::General, t!("settings-page-general")),
            Self::Search => (Icon::Search, t!("settings-page-search")),
            Self::Clipboard => (Icon::Clipboard, t!("settings-page-clipboard")),
            Self::Shortcuts => (Icon::Keyboard, t!("settings-page-shortcuts")),
            Self::Appearance => (Icon::Palette, t!("settings-page-appearance")),
            Self::Data => (Icon::Lock, t!("settings-page-data")),
        }
    }
}

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
            for id in self.library.images_without_ocr() {
                self.worker.send_backfill(Job::Ocr(id));
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t!("app-title")));
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
        let mut discard_model = false;
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
                        &t!("settings-back"),
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
                    lbl(ui, &t!("settings-title"), 20.0, strong);
                    far(ui, |ui| {
                        if model_pending {
                            if primary_button(ui, &t!("settings-apply-model")).clicked() {
                                apply_model = true;
                            }
                            if ui
                                .button(txt(&t!("settings-discard-changes"), 13.0, None))
                                .clicked()
                            {
                                discard_model = true;
                            }
                            chip(ui, t!("settings-model-not-applied"), AMBER);
                        } else {
                            let weak = ui.visuals().weak_text_color();
                            lbl(ui, &t!("settings-saved-automatically"), 12.5, weak);
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
                        &name,
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
        if discard_model {
            d.backend = self.cfg.backend.clone();
        }
        // App::logic turns the global shortcuts back on in the next frame.
        if self.hotkeys_paused && self.recording.is_none() {
            ctx.request_repaint();
        }
        self.draft = Some(d);
    }
}
