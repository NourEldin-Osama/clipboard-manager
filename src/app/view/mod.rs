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
use crate::config::Density;

mod chrome;
mod details;
mod grid;
mod items;
mod keyboard;
mod list;
mod theme;
mod time_format;
mod top_bar;
mod widgets;

// The helpers of every part, for this file and its parts.
use items::*;
use keyboard::*;
use list::*;
use theme::*;
use time_format::*;
use widgets::*;

// The ones the rest of the app uses.
pub(crate) use details::reveal;
pub(crate) use items::looks_like_code;
pub(crate) use theme::{
    accent, add_fonts, apply_theme, set_accent, style, ACCENTS, AMBER, GREEN, RED,
};
pub(crate) use widgets::{
    chip, far, flip, lbl, paint_text, primary_button, row, row_layout, row_wrapped, start_anchor,
    tip, txt,
};

impl App {
    /// Everything below the top bar: list, grid or list + details, per the view mode.
    fn main_area(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        // Panels swap sides right to left: the list starts on the reading side.
        let (start, end) = if i18n::rtl() {
            (egui::Panel::right("list"), egui::Panel::left("details"))
        } else {
            (egui::Panel::left("list"), egui::Panel::right("details"))
        };
        if self.cfg.view == ViewMode::Detail {
            start
                .resizable(true)
                .show_separator_line(false)
                .default_size(ui.available_width() * 0.42)
                .size_range(300.0..=900.0)
                .frame(
                    Frame::new()
                        .inner_margin(Margin {
                            left: 8,
                            right: 8,
                            top: 4,
                            bottom: 4,
                        })
                        .fill(fill),
                )
                .show(ui, |ui| self.list(ui));
            let detail_fill = ui.visuals().extreme_bg_color;
            egui::CentralPanel::default()
                .frame(
                    Frame::new()
                        .inner_margin(Margin {
                            left: 24,
                            right: 20,
                            top: 14,
                            bottom: 14,
                        })
                        .fill(detail_fill),
                )
                .show(ui, |ui| self.details(ui, false));
            return;
        }
        if self.details_open && self.selected.is_some() {
            let detail_fill = ui.visuals().extreme_bg_color;
            end.resizable(true)
                .show_separator_line(false)
                .default_size(ui.available_width() * 0.45)
                .size_range(320.0..=1100.0)
                .frame(
                    Frame::new()
                        .inner_margin(Margin {
                            left: 20,
                            right: 20,
                            top: 14,
                            bottom: 14,
                        })
                        .fill(detail_fill),
                )
                .show(ui, |ui| self.details(ui, true));
        }
        let margin = if self.cfg.view == ViewMode::Grid {
            Margin {
                left: 16,
                right: 12,
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

    /// Runs what the list or grid asked for while it was drawn.
    fn apply_actions(&mut self, actions: Vec<Command>) {
        for command in actions {
            self.run(command);
        }
        if self.thumbs.len() > MAX_THUMBS {
            for id in self.thumbs.drain().map(|(id, _)| id).collect::<Vec<_>>() {
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.finish_delete();
        self.install_on_exit();
        crate::embed::server::stop_launched_server();
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // However a shortcut recording ended (new keys, the same keys, Esc, another
        // settings page, or leaving Settings), the global shortcuts come back.
        // New keys are already registered by apply_settings, which clears the flag.
        if self.hotkeys_paused && self.recording.is_none() {
            self.register_hotkeys(ctx);
        }
        // A delete becomes final when its undo time is over, also while hidden.
        if let Some(pending) = &self.pending_delete {
            let left = UNDO_FOR.saturating_sub(pending.since.elapsed());
            if left.is_zero() {
                self.finish_delete();
            } else {
                ctx.request_repaint_after(left);
            }
        }
        self.window_logic(ctx);
        self.handle_events(ctx);
        self.focused.store(
            ctx.input(|i| i.viewport().focused.unwrap_or(false)),
            Ordering::SeqCst,
        );
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
            self.update_bar(ui);
            self.status_bar(ui);
            self.main_area(ui);
        }
        self.toasts(&ctx);
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("drop"),
            ));
            p.rect_filled(r, 0.0, Color32::from_black_alpha(160));
            let g = ctx.fonts_mut(|f| {
                f.layout_job(bidi::job(
                    tr("Drop to add to your searchable library"),
                    FontId::proportional(22.0),
                    Color32::WHITE,
                ))
            });
            let row = Rect::from_center_size(
                r.center(),
                Vec2::new(26.0 + 6.0 + g.size().x, g.size().y.max(26.0)),
            );
            icons::paint_icon_text(&p, row, Icon::Folder, 26.0, g, Color32::WHITE);
        }
    }
}
