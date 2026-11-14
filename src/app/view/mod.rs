//! Main window: search bar, filter chips, grouped list, details pane, status bar.
//!
//! Structure comes from alignment, background tone and type weight rather than
//! borders: the list and details panes differ by fill, rows by tint, groups by
//! small headers on the same left edge as the content.

use std::sync::atomic::AtomicUsize;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Align, Align2, Frame, Layout, Margin, Pos2, Rect, Ui};

use super::bidi;
use super::icons::{self, icon_button_at, icon_button_ex, view_icon, Icon};
use super::shortcut;
use super::*;
use crate::config::{CardHeight, Density};

mod checklist;
mod chrome;
mod details;
mod grid;
mod items;
mod key_hints;
mod keyboard;
mod keys_sheet;
mod list;
mod menu;
mod messages;
mod note;
mod popup;
mod theme;
mod time_format;
mod top_bar;
mod whats_new;
mod widgets;

// The helpers of every part, for this file and its parts.
use items::*;
use key_hints::*;
use keyboard::*;
use list::*;
use theme::*;
use time_format::*;
use widgets::*;

// The ones the rest of the app uses.
pub(crate) use details::reveal;
pub(crate) use grid::{CardHeights, CARD_CHIP_PADDING, CHIP_SIDE, GRID_GAP};
pub(crate) use items::looks_like_code;
pub(crate) use messages::{Question, UpdateParts};
pub(crate) use theme::{
    accent, accent_name, add_fonts, apply_theme, button_fill, card_frame, choice_track,
    chosen_face, chosen_pill, key_cap, line_color, lit, note_fill, radius, semibold, set_accent,
    size, space, style, text_shade, warning_fill, ACCENTS, AMBER, GREEN, READING_WIDTH, RED,
};
pub(crate) use widgets::{
    chip, cut_line, divider, far, far_layout, flip, lbl, paint_progress, paint_text, part_corners,
    pill_job, primary_button, row, row_layout, row_of, row_wrapped, section_title, set_choice_name,
    set_name, slow_spinner, start_anchor, tip, title_lbl, txt, Reacts, PROGRESS_LINE,
};

impl App {
    /// Everything below the top bar: list, grid or list + details, per the view mode.
    fn main_area(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        // The details at the reading end: always in Detail view, and while
        // open over the grid or the list, with a Back button there. The list
        // or the grid takes the rest.
        if self.pane_shown() {
            let closeable = self.cfg.view != ViewMode::Detail;
            details::details_panel(ui, |ui| self.details(ui, closeable));
        }
        // The cards end 16 points from the window's edge, after the 12 the
        // grid keeps for its scroll bar; rows add their own 8 to the 8 here.
        let margin = if self.cfg.view == ViewMode::Grid {
            Margin {
                left: 16,
                right: 4,
                top: 8,
                bottom: 4,
            }
        } else {
            Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            }
        };
        egui::CentralPanel::default()
            .frame(Frame::new().inner_margin(margin).fill(fill))
            .show(ui, |ui| {
                if self.cfg.view == ViewMode::Grid {
                    self.grid(ui)
                } else {
                    self.list(ui, false)
                }
            });
    }

    pub(super) fn set_view(&mut self, v: ViewMode) {
        self.cfg.view = v;
        self.details_open = false;
        self.scroll_to_selected = true;
        self.save_config();
    }

    /// The details pane shows beside the list or the grid: always in
    /// Detail, and after Open details in the other two.
    fn pane_shown(&self) -> bool {
        self.cfg.view == ViewMode::Detail || (self.details_open && self.selected.is_some())
    }

    /// Shows `view` with the details pane `open` beside it or not, as the
    /// layout controls ask. Only the view is saved: the pane beside the grid
    /// or the list is the one Open details opens.
    fn show_layout(&mut self, (view, open): (ViewMode, bool)) {
        if view != self.cfg.view {
            self.set_view(view);
        }
        if view == ViewMode::Detail {
            return;
        }
        if !open {
            self.details_open = false;
            return;
        }
        // With none selected, the pane shows the first item shown.
        if self.selected.is_none() {
            self.selected = self.results.first().map(|hit| hit.id);
            self.scroll_to_selected = true;
        }
        self.details_open = self.selected.is_some();
    }

    /// A file dialog took the keyboard focus away: the search box gets it
    /// back, so typing searches again.
    pub(super) fn after_dialog(&self) {
        self.focus_search.store(true, Ordering::SeqCst);
    }

    /// Runs what the list or grid asked for while it was drawn.
    fn apply_actions(&mut self, actions: Vec<Command>) {
        for command in actions {
            self.run(command);
        }
        if self.thumbs.len() > MAX_THUMBS {
            for id in thumbs_to_drop(&self.thumb_seen, self.thumbs.keys().copied(), THUMBS_KEPT) {
                self.thumbs.remove(&id);
                self.thumb_seen.remove(&id);
                self.loader.forget(id, false);
            }
        }
    }
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let line_points = ctx.options(|o| o.input_options.line_scroll_speed);
        // The keys held as the frame starts; this frame's changes are events.
        let modifiers = ctx.input(|i| i.modifiers);
        let events = std::mem::take(&mut raw_input.events);
        raw_input.events = repair_input(events, modifiers, line_points);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        keep_window_state(storage, self.was_minimized, &mut self.window_state);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.finish_delete();
        self.install_on_exit();
        crate::llama::stop_all();
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // However a shortcut recording ended (new keys, the same keys, Esc, another
        // settings page, or leaving Settings), the global shortcuts come back.
        // New keys are already registered by apply_settings, which clears the flag.
        if self.hotkeys_paused && self.recording.is_none() {
            self.register_hotkeys(ctx);
        }
        // The messages' time runs while a window shows them (D55). A delete
        // becomes final when its message is gone, so its Undo waits while
        // the window is hidden.
        let main_shown = !self.win.hidden.load(Ordering::SeqCst)
            && ctx.input(|i| i.viewport().minimized != Some(true));
        let shown = main_shown || self.win.popup.load(Ordering::SeqCst);
        self.tick_toasts(ctx, shown);
        if self.pending_delete.is_some() && !self.toasts.iter().any(Toast::undoes) {
            self.finish_delete();
        }
        // A search server that stopped answering is asked again now and then,
        // also while hidden.
        if let Some(retry) = &mut self.search_retry {
            let now = Instant::now();
            if now >= retry.at {
                retry.again(now);
                self.worker.send(Job::Probe);
            }
            ctx.request_repaint_after(retry.at.saturating_duration_since(now));
        }
        // Deleting by age runs at start and then every hour, also while hidden.
        if self.cfg.delete_after_days > 0
            && self.next_age_prune.is_none_or(|at| Instant::now() >= at)
        {
            self.next_age_prune = Some(Instant::now() + AGE_PRUNE_EVERY);
            self.prune(true);
            ctx.request_repaint_after(AGE_PRUNE_EVERY);
        }
        self.window_logic(ctx);
        self.popup_logic(ctx);
        self.note_logic(ctx);
        self.handle_events(ctx);
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if self.focused.swap(focused, Ordering::SeqCst) && !focused {
            self.left_for_another_program();
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if !dropped.is_empty() {
            self.run(Command::Import(dropped));
        }
        if self.query_changed.is_some() {
            ctx.request_repaint_after(QUERY_DEBOUNCE);
        }
        if self.research_pending {
            ctx.request_repaint_after(RESEARCH_EVERY);
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        bidi::forget_drawn();
        self.keyboard_focus = keyboard_moved_focus(&ctx, self.keyboard_focus);
        self.keyboard(&ctx, false);
        self.refresh_results();
        // Before the main window, so the popup's search field takes the
        // keyboard when it opens.
        self.popup_window(&ctx);
        self.note_window(&ctx);
        // Hidden or minimised, the main window draws nothing; the popup may
        // be open meanwhile.
        if self.win.hidden.load(Ordering::SeqCst)
            || ctx.input(|i| i.viewport().minimized == Some(true))
        {
            return;
        }
        // The window draws, so a show has worked.
        self.show_check = None;
        if self.draft.is_some() {
            self.settings_ui(ui);
        } else {
            self.top_bar(ui);
            self.status_bar(ui);
            self.main_area(ui);
        }
        self.toasts(&ctx, false);
        self.keys_sheet(&ctx, false);
        self.whats_new(&ctx);
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("drop"),
            ));
            p.rect_filled(r, 0.0, theme::drop_shade());
            let g = ctx.fonts_mut(|f| {
                f.layout_job(bidi::job(
                    &t!("list-drop-files"),
                    semibold(size::TITLE),
                    Color32::WHITE,
                ))
            });
            let row = Rect::from_center_size(
                r.center(),
                Vec2::new(26.0 + 6.0 + g.size().x, g.size().y.max(26.0)),
            );
            icons::paint_icon_text(&p, row, Icon::Folder, 26.0, g, Color32::WHITE);
        }
        if self.keyboard_focus {
            focus_outline(&ctx);
        }
        bidi::read_in_reading_order(&ctx);
    }
}
