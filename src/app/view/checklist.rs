//! The getting-started checklist for new installs (D53): four steps that
//! tick themselves as they are done. It shows in the empty list, then above
//! the rows, until every step is done or it is closed; then it is gone for
//! good.

use super::*;
use crate::config::SearchEngine;

impl App {
    /// The steps, each done or not, with its text.
    fn checklist_steps(&self) -> [(bool, String); 4] {
        let items = self.library.items();
        // The popup's key, else the full window's; without either, the
        // window is opened some other way.
        let keys = [&self.cfg.popup_hotkey, &self.cfg.window_hotkey]
            .into_iter()
            .find(|k| !k.trim().is_empty());
        let paste = match keys {
            Some(keys) => t!("checklist-paste", keys = keys.as_str()),
            None => t!("checklist-paste-no-key"),
        };
        [
            (!items.is_empty(), t!("checklist-copy")),
            (self.cfg.checklist.pasted, paste),
            (items.iter().any(|it| it.pinned), t!("checklist-pin")),
            (
                self.cfg.search_engine != SearchEngine::Off,
                t!("checklist-meaning"),
            ),
        ]
    }

    /// Closes the checklist for good.
    fn close_checklist(&mut self) {
        self.cfg.checklist.closed = true;
        self.save_config();
    }

    /// The checklist as a card as wide as `ui`, while it is open.
    pub(super) fn checklist(&mut self, ui: &mut Ui) {
        if self.cfg.checklist.closed {
            return;
        }
        let steps = self.checklist_steps();
        if steps.iter().all(|(done, _)| *done) {
            self.close_checklist();
            return;
        }
        let v = ui.visuals().clone();
        let (mut close, mut set_up, mut turn_on) = (false, false, false);
        // A model server on this PC with a search model turns it on in one
        // click; without one, Set up opens Settings.
        let found = if steps[3].0 {
            None
        } else {
            self.local_servers.look(ui.ctx());
            self.local_servers.found.iter().find_map(|server| {
                let model = server
                    .models
                    .iter()
                    .find(|m| crate::catalog::is_search_model(m))?;
                let saved =
                    self.cfg.servers.iter().any(|s| {
                        crate::net::host(&s.base_url) == crate::net::host(&server.base_url)
                    });
                (!saved).then(|| (server.clone(), model.clone()))
            })
        };
        Frame::new()
            .fill(v.extreme_bg_color)
            .corner_radius(CornerRadius::same(radius::LARGE))
            .inner_margin(Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                row(ui, |ui| {
                    ui.label(title_txt(&t!("checklist-title"), size::BODY, None));
                });
                ui.add_space(space::TINY);
                for (i, (done, text)) in steps.iter().enumerate() {
                    row(ui, |ui| {
                        step_mark(ui, *done);
                        ui.add_space(space::TINY);
                        let color = if *done {
                            v.weak_text_color()
                        } else {
                            v.text_color()
                        };
                        match (&found, i == 3 && !*done) {
                            (Some((server, _)), true) => {
                                let text = t!(
                                    "checklist-meaning-found",
                                    server = bidi::isolate(server.kind.name())
                                );
                                lbl(ui, &text, size::CONTROL, color);
                                let link = txt(&t!("checklist-turn-on"), size::CONTROL, None);
                                turn_on = ui.link(link).clicked();
                            }
                            // Search by meaning is set up in Settings.
                            (None, true) => {
                                lbl(ui, text, size::CONTROL, color);
                                let link = txt(&t!("checklist-set-up"), size::CONTROL, None);
                                set_up = ui.link(link).clicked();
                            }
                            _ => {
                                lbl(ui, text, size::CONTROL, color);
                            }
                        }
                    });
                }
                row(ui, |ui| {
                    far(ui, |ui| {
                        let link = txt(&t!("checklist-close"), size::SMALL, None);
                        close = ui.link(link).clicked();
                    });
                });
            });
        if set_up {
            self.open_settings(SettingsPage::Search);
        }
        if let Some((server, model)) = found.filter(|_| turn_on) {
            let mut cfg = self.cfg.clone();
            crate::app::settings::use_found(&mut cfg, &server, &model);
            self.apply_settings(cfg, ui.ctx(), false);
        }
        if close {
            self.close_checklist();
        }
    }

    /// The checklist above the rows or cards, `offset` from the left and
    /// `width` wide, as they are, while it is open and nothing is searched.
    pub(super) fn checklist_above(&mut self, ui: &mut Ui, offset: f32, width: f32) {
        if self.cfg.checklist.closed || !self.query.trim().is_empty() {
            return;
        }
        ui.horizontal(|ui| {
            // No gap before the card, but the usual gaps inside it.
            let gap = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(offset);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.set_width(width);
                self.checklist(ui);
            });
        });
        ui.add_space(space::SMALL);
    }
}

/// A step's mark: a check in the accent color when done, an empty ring when
/// not.
fn step_mark(ui: &mut Ui, done: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
    let p = ui.painter();
    if done {
        p.circle_filled(rect.center(), 9.0, button_fill(accent()));
        let check = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
        icons::paint(p, check, Icon::Check, Color32::WHITE);
    } else {
        let ring = ui.visuals().weak_text_color().gamma_multiply(0.7);
        p.circle_stroke(rect.center(), 8.0, Stroke::new(1.5, ring));
    }
}
