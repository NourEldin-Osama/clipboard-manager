//! Settings as a full page with sections. Changes apply as you make them,
//! except a server's settings, which wait for Save in its editor, and the
//! global shortcuts.

use eframe::egui::{Frame, Margin, Pos2, Rect, Ui};

use super::icons::{self, Icon};
use super::shortcut;
use super::view::{
    accent, accent_name, button_fill, card_frame, chip, choice_track, chosen_face, cut_line,
    divider, far, far_layout, flip, lbl, line_color, note_fill, paint_progress, paint_text,
    pill_job, primary_button, radius, reveal, row, row_layout, row_of, set_choice_name, set_name,
    size, slow_spinner, space, start_anchor, text_shade, tip, title_lbl, txt, warning_fill, Reacts,
    ACCENTS, AMBER, GREEN, PROGRESS_LINE, RED,
};
use super::*;
use crate::config::{
    CardHeight, DarkPanels, Density, Language, ReaderModel, TextReader, TimeFormat,
};

mod about;
mod appearance;
mod clipboard;
mod general;
mod privacy;
mod search;
mod servers;
mod shortcuts;
mod widgets;

// The helpers of every page, for this file and the pages.
use appearance::*;
use servers::*;
use widgets::*;

pub(super) use servers::{model_label, use_found, PendingSend, ServerEditor};

/// How wide a settings page is at most.
const COLUMN_WIDTH: f32 = 800.0;

/// How far a settings page starts after the page list's rows.
const COLUMN_GAP: f32 = 32.0;

/// How wide the page list is, and the room it keeps beside its rows.
const PAGE_LIST_WIDTH: f32 = 220.0;
const PAGE_LIST_PADDING: i8 = 10;

/// The room at a page's far side, where its scroll bar is.
const PAGE_FAR_MARGIN: i8 = 20;

/// The pages, each about one question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SettingsPage {
    General,
    Appearance,
    /// What is kept, and for how long.
    Clipboard,
    /// What is never kept, and where the data goes.
    Privacy,
    Search,
    /// The saved servers and the ones found on this PC (D68).
    Servers,
    Shortcuts,
    About,
}

impl SettingsPage {
    const ALL: [SettingsPage; 8] = [
        Self::General,
        Self::Appearance,
        Self::Clipboard,
        Self::Privacy,
        Self::Search,
        Self::Servers,
        Self::Shortcuts,
        Self::About,
    ];

    fn label(self) -> (Icon, String) {
        match self {
            Self::General => (Icon::General, t!("settings-page-general")),
            Self::Appearance => (Icon::Palette, t!("settings-page-appearance")),
            Self::Clipboard => (Icon::Clipboard, t!("settings-page-clipboard")),
            Self::Privacy => (Icon::Lock, t!("settings-page-privacy")),
            Self::Search => (Icon::Search, t!("settings-page-search")),
            Self::Servers => (Icon::Server, t!("settings-page-servers")),
            Self::Shortcuts => (Icon::Keyboard, t!("settings-page-shortcuts")),
            Self::About => (Icon::Info, t!("settings-page-about")),
        }
    }
}

impl App {
    pub(super) fn open_settings(&mut self, page: SettingsPage) {
        self.stash_server_editor();
        self.server_lists = Default::default();
        self.draft = Some(self.cfg.clone());
        self.settings_page = page;
        self.recording = None;
        self.confirm = None;
        self.pending_send = None;
    }

    /// Leaving settings applies everything. A server being edited and not
    /// saved stays as it was, and a server outside this PC that the user
    /// hasn't agreed to gets nothing.
    pub(super) fn close_settings(&mut self, ctx: &egui::Context) {
        self.stash_server_editor();
        self.pending_send = None;
        if let Some(d) = self.draft.take() {
            self.apply_settings(d, ctx, true);
        }
        self.recording = None;
        if self.hotkeys_paused {
            ctx.request_repaint();
        }
        self.confirm = None;
    }

    pub(super) fn apply_settings(&mut self, mut d: Config, ctx: &egui::Context, prune: bool) {
        if d == self.cfg && !prune {
            return;
        }
        let backend_changed = d.search_backend() != self.cfg.search_backend();
        let keys_changed = d.popup_hotkey != self.cfg.popup_hotkey
            || d.window_hotkey != self.cfg.window_hotkey
            || d.ignore_next_hotkey != self.cfg.ignore_next_hotkey;
        let reader = d.text_reader();
        if reader != self.cfg.text_reader() {
            self.worker.send(Job::SetTextReader(reader.clone()));
            // Images not read yet get read now; images already read keep
            // their text.
            if reader != TextReader::Off {
                for id in self.library.images_without_ocr() {
                    self.worker.send_backfill(Job::Ocr(id));
                }
            }
        }
        let tray_changed = d.tray != self.cfg.tray;
        let login_changed = d.start_at_login != self.cfg.start_at_login;
        let look_changed = d.accent != self.cfg.accent
            || d.theme != self.cfg.theme
            || d.dark_panels != self.cfg.dark_panels;
        let language_changed = d.language != self.cfg.language;
        i18n::set(d.language, d.time_format);
        let search_changed =
            d.search_mode != self.cfg.search_mode || d.min_similarity != self.cfg.min_similarity;
        if d.rules != self.cfg.rules {
            self.rules = crate::rules::Rules::new(&d.rules);
        }
        // Kept by the app itself, not by the page.
        d.history_warned = self.cfg.history_warned;
        self.cfg = d;
        self.flags
            .enabled
            .store(self.cfg.monitor_clipboard, Ordering::SeqCst);
        self.flags
            .skip_secrets
            .store(self.cfg.skip_secrets, Ordering::SeqCst);
        if look_changed {
            set_accent(self.cfg.accent);
            style(ctx, self.cfg.dark_panels);
            apply_theme(ctx, self.cfg.theme);
        }
        self.save_config();
        if backend_changed {
            self.set_backend(self.cfg.search_backend());
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
                self.set_error(log_error!(
                    "error-start-at-sign-in",
                    error = format!("{e:#}")
                ));
                self.cfg.start_at_login = !self.cfg.start_at_login;
                self.save_config();
            }
        }
        if search_changed {
            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
        }
        if prune {
            self.prune(true);
        }
        self.dirty = true;
    }

    pub(super) fn settings_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let Some(mut d) = self.draft.take() else {
            return;
        };
        let before = d.clone();
        let mut close = false;
        let mut apply_keys = false;

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
                    ui.add_space(space::SMALL);
                    let strong = ui.visuals().strong_text_color();
                    title_lbl(ui, &t!("settings-title"), size::TITLE, strong);
                    // A server's settings wait for its Save.
                    if self.server_editor.is_none() {
                        far(ui, |ui| {
                            let weak = ui.visuals().weak_text_color();
                            lbl(ui, &t!("settings-saved-automatically"), size::SMALL, weak);
                            icons::glyph(ui, Icon::Check, 13.0, weak);
                        });
                    }
                });
            });

        page_list_panel(panel).show(ui, |ui| {
            for page in SettingsPage::ALL {
                let (icon, name) = page.label();
                let sel = self.settings_page == page;
                let (rect, r) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
                r.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, sel, &name));
                let v = ui.visuals();
                let color = if sel {
                    text_shade(accent(), v.dark_mode)
                } else if r.hovered() {
                    v.strong_text_color()
                } else {
                    v.text_color()
                };
                if sel {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(radius::MEDIUM),
                        accent().gamma_multiply(0.13),
                    );
                } else if r.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(radius::MEDIUM),
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
                    Rect::from_min_size(Pos2::new(rect.left() + 42.0, rect.center().y), Vec2::ZERO),
                )
                .min;
                paint_text(
                    ui,
                    ui.painter(),
                    at,
                    start_anchor(egui::Align::Center),
                    &name,
                    FontId::proportional(size::BODY),
                    color,
                );
                if r.clicked() {
                    self.settings_page = page;
                    self.confirm = None;
                    self.recording = None;
                    self.stash_server_editor();
                }
            }
        });

        page_column(ui, self.settings_page, |ui| match self.settings_page {
            SettingsPage::General => self.general_page(ui, &mut d),
            SettingsPage::Appearance => appearance_page(ui, &mut d),
            SettingsPage::Clipboard => self.clipboard_page(ui, &mut d),
            SettingsPage::Privacy => self.privacy_page(ui, &mut d),
            SettingsPage::Search => self.search_page(ui, &mut d),
            SettingsPage::Servers => self.servers_page(ui, &mut d),
            SettingsPage::Shortcuts => self.shortcuts_page(ui, &mut d, &mut apply_keys),
            SettingsPage::About => self.about_page(ui, &mut d),
        });

        // A server outside this PC gets nothing before the user agrees.
        self.hold_unagreed(&mut d, before);
        if close {
            // Back from a server's editor goes to the page, not out of Settings.
            if !self.stash_server_editor() {
                self.draft = Some(d);
                self.close_settings(&ctx);
                return;
            }
        }
        // Everything except the global shortcuts applies immediately.
        let mut live = d.clone();
        live.popup_hotkey = self.cfg.popup_hotkey.clone();
        live.window_hotkey = self.cfg.window_hotkey.clone();
        live.ignore_next_hotkey = self.cfg.ignore_next_hotkey.clone();
        if live != self.cfg {
            self.apply_settings(live, &ctx, false);
            // A setting the system refused (start at sign-in) snaps back.
            d.start_at_login = self.cfg.start_at_login;
        }
        if apply_keys {
            let mut c = self.cfg.clone();
            c.popup_hotkey = d.popup_hotkey.clone();
            c.window_hotkey = d.window_hotkey.clone();
            c.ignore_next_hotkey = d.ignore_next_hotkey.clone();
            self.apply_settings(c, &ctx, false);
        }
        // App::logic turns the global shortcuts back on in the next frame.
        if self.hotkeys_paused && self.recording.is_none() {
            ctx.request_repaint();
        }
        self.draft = Some(d);
    }
}

/// The page list's panel, on the reading-start side.
fn page_list_panel(fill: Color32) -> egui::Panel {
    let panel = if i18n::rtl() {
        egui::Panel::right("settings_nav")
    } else {
        egui::Panel::left("settings_nav")
    };
    panel
        .resizable(false)
        .exact_size(PAGE_LIST_WIDTH)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .inner_margin(Margin::symmetric(PAGE_LIST_PADDING, 8))
                .fill(fill),
        )
}

/// The page beside the page list, on the window's gray as the page list
/// is, so its cards stand on it as the main window's do: a column that
/// starts `COLUMN_GAP` after the page list's rows, narrow enough that each
/// control stays near its name, with room at its far side for the cards'
/// shadow. Each page keeps its own scroll position.
fn page_column(ui: &mut Ui, page: SettingsPage, add: impl FnOnce(&mut Ui)) {
    let (left, right) = if i18n::rtl() {
        (PAGE_FAR_MARGIN, 0)
    } else {
        (0, PAGE_FAR_MARGIN)
    };
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .inner_margin(Margin {
                    left,
                    right,
                    top: 8,
                    bottom: 16,
                })
                .fill(ui.visuals().panel_fill),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("settings", page))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // The page list's rows end its padding before this
                    // panel's edge.
                    let area = ui.available_rect_before_wrap();
                    let start = COLUMN_GAP - f32::from(PAGE_LIST_PADDING);
                    let width = (area.width() - start - space::SMALL).clamp(0.0, COLUMN_WIDTH);
                    let column = flip(
                        area,
                        Rect::from_min_size(
                            Pos2::new(area.left() + start, area.top()),
                            Vec2::new(width, area.height()),
                        ),
                    );
                    ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                        // The pages set their own gaps between titles and cards.
                        ui.spacing_mut().item_spacing.y = 0.0;
                        add(ui);
                    });
                    ui.add_space(space::HUGE);
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::arabic_run;

    /// Where the page list's rows and the page's column are in a window
    /// `width` wide.
    fn rows_and_column(width: f32) -> (Rect, Rect) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 700.0))),
            ..Default::default()
        };
        let (mut rows, mut column) = (Rect::NOTHING, Rect::NOTHING);
        let mut output = ctx.run_ui(input, |ui| {
            let fill = ui.visuals().panel_fill;
            // The rows take the page list's whole width.
            page_list_panel(fill).show(ui, |ui| rows = ui.max_rect());
            page_column(ui, SettingsPage::General, |ui| column = ui.max_rect());
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        (rows, column)
    }

    /// The gap from the page list's rows to the column, and the room left
    /// at the window's far side.
    fn gap_and_far_room(width: f32, rows: Rect, column: Rect) -> (f32, f32) {
        if i18n::rtl() {
            (rows.left() - column.right(), column.left())
        } else {
            (column.left() - rows.right(), width - column.right())
        }
    }

    fn check_column_beside_page_list() {
        for width in [1280.0, 1920.0] {
            let (rows, column) = rows_and_column(width);
            let (gap, _) = gap_and_far_room(width, rows, column);
            assert_eq!(gap, COLUMN_GAP, "{width}: {rows:?} {column:?}");
            assert_eq!(column.width(), COLUMN_WIDTH, "{width}");
        }
        // Narrower than the page list and a whole column: the column takes
        // the room there is, short of the scroll bar's and the shadow's.
        for width in [560.0, 800.0, 1000.0] {
            let (rows, column) = rows_and_column(width);
            let (gap, far) = gap_and_far_room(width, rows, column);
            assert_eq!(gap, COLUMN_GAP, "{width}");
            assert_eq!(far, f32::from(PAGE_FAR_MARGIN) + space::SMALL, "{width}");
            assert!(column.width() < COLUMN_WIDTH, "{width}: {column:?}");
        }
    }

    #[test]
    fn the_column_starts_beside_the_page_list() {
        check_column_beside_page_list();
    }

    #[test]
    fn the_column_starts_beside_the_page_list_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_column_starts_beside_the_page_list_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_column_starts_beside_the_page_list_right_to_left"]
    fn the_column_starts_beside_the_page_list_in_arabic() {
        if arabic_run() {
            check_column_beside_page_list();
        }
    }
}
