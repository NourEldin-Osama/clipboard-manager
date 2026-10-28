//! The update bar, the status bar and toasts.

use super::*;

/// A button on the update bar.
enum UpdateAction {
    Later,
    Skip,
    Download,
    Cancel,
    Restart,
    Retry,
}

impl App {
    /// A strip under the search bar while an update is found, downloading,
    /// ready, or failed. "Later" hides it until something changes.
    pub(super) fn update_bar(&mut self, ui: &mut Ui) {
        let show = !self.update_hidden
            && matches!(
                self.update,
                UpdateState::Available(_)
                    | UpdateState::Downloading(..)
                    | UpdateState::Ready(..)
                    | UpdateState::Failed { .. }
            );
        if !show {
            return;
        }
        let ctx = ui.ctx().clone();
        // Opaque: a panel's frame is all that's painted there, so a see-through
        // tint would show the window's black backdrop.
        let fill = ui
            .visuals()
            .panel_fill
            .lerp_to_gamma(accent(), if ui.visuals().dark_mode { 0.18 } else { 0.10 });
        let mut action: Option<UpdateAction> = None;
        egui::Panel::top("update")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 16,
                        right: 12,
                        top: 6,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    let text_color = ui.visuals().strong_text_color();
                    let notes = match &self.update {
                        UpdateState::Available(r)
                        | UpdateState::Downloading(r, _)
                        | UpdateState::Ready(r, _) => Some(r.notes_url.clone()),
                        UpdateState::Failed {
                            release: Some(r), ..
                        } => Some(r.notes_url.clone()),
                        _ => None,
                    };
                    // The buttons go first, at the far end, so the message can
                    // be cut to the room they leave.
                    far(ui, |ui| {
                        if icon_button_ex(
                            ui,
                            Icon::Close,
                            &t!("update-later"),
                            Vec2::splat(26.0),
                            12.0,
                            None,
                            None,
                        )
                        .clicked()
                        {
                            action = Some(UpdateAction::Later);
                        }
                        match &self.update {
                            UpdateState::Available(_) => {
                                if ui.button(txt(&t!("update-skip"), 13.0, None)).clicked() {
                                    action = Some(UpdateAction::Skip);
                                }
                                if primary_button(ui, &t!("update-download")).clicked() {
                                    action = Some(UpdateAction::Download);
                                }
                            }
                            UpdateState::Downloading(..) => {
                                if ui.button(txt(&t!("common-cancel"), 13.0, None)).clicked() {
                                    action = Some(UpdateAction::Cancel);
                                }
                            }
                            UpdateState::Ready(..) => {
                                if primary_button(ui, &t!("update-restart")).clicked() {
                                    action = Some(UpdateAction::Restart);
                                }
                            }
                            UpdateState::Failed {
                                release: Some(_), ..
                            } if ui
                                .button(txt(&t!("update-try-again"), 13.0, None))
                                .clicked() =>
                            {
                                action = Some(UpdateAction::Retry);
                            }
                            _ => {}
                        }
                        if let Some(url) = notes {
                            if ui.link(txt(&t!("update-whats-new"), 13.0, None)).clicked() {
                                ctx.open_url(egui::OpenUrl::new_tab(url));
                            }
                        }
                        ui.with_layout(row_layout(), |ui| match &self.update {
                            UpdateState::Available(r) => {
                                ui.label(txt(
                                    &t!("update-available", version = r.label()),
                                    13.5,
                                    Some(text_color),
                                ));
                            }
                            UpdateState::Downloading(r, p) => {
                                let (done, total) = (
                                    p.done.load(Ordering::Relaxed),
                                    p.total.load(Ordering::Relaxed).max(1),
                                );
                                if done >= total {
                                    ui.label(txt(
                                        &t!("update-verifying", version = r.label()),
                                        13.5,
                                        Some(text_color),
                                    ));
                                    slow_spinner(ui, 14.0, None);
                                } else {
                                    ui.label(txt(
                                        &t!("update-downloading", version = r.label()),
                                        13.5,
                                        Some(text_color),
                                    ));
                                    ui.add(
                                        egui::ProgressBar::new(done as f32 / total as f32)
                                            .desired_width(160.0)
                                            .text(txt(
                                                &t!(
                                                    "update-progress",
                                                    done = done >> 20,
                                                    total = total >> 20
                                                ),
                                                13.0,
                                                None,
                                            )),
                                    );
                                }
                            }
                            UpdateState::Ready(r, _) => {
                                ui.label(txt(
                                    &t!("update-ready-restart", version = r.label()),
                                    13.5,
                                    Some(text_color),
                                ));
                            }
                            UpdateState::Failed { error, .. } => {
                                icons::glyph(ui, Icon::Warning, 14.0, RED);
                                let text = t!("update-failed", error = error.as_str());
                                let width = ui.available_width();
                                cut_line(ui, &text, width, 13.0, RED, Sense::hover());
                            }
                            _ => {}
                        });
                    });
                });
            });
        match action {
            Some(UpdateAction::Later) => self.update_hidden = true,
            Some(UpdateAction::Skip) => {
                if let UpdateState::Available(r) = &self.update {
                    self.cfg.skipped_update = r.key();
                    self.save_config();
                }
                self.update = UpdateState::Idle;
            }
            Some(UpdateAction::Download | UpdateAction::Retry) => {
                let r = match &self.update {
                    UpdateState::Available(r) => Some(r.clone()),
                    UpdateState::Failed {
                        release: Some(r), ..
                    } => Some(r.clone()),
                    _ => None,
                };
                if let Some(r) = r {
                    self.download_update(r, &ctx);
                }
            }
            Some(UpdateAction::Cancel) => {
                if let UpdateState::Downloading(_, p) = &self.update {
                    p.cancel.store(true, Ordering::Relaxed);
                }
            }
            Some(UpdateAction::Restart) => self.restart_to_update(&ctx),
            None => {}
        }
    }

    pub(crate) fn model_status(&self) -> (Color32, String) {
        match (&self.backend, &self.cfg.backend) {
            (BackendState::Loading, _) => (AMBER, t!("status-model-loading")),
            (BackendState::Ready { .. }, EmbedBackend::SigLip { repo }) => (
                GREEN,
                t!(
                    "status-model-builtin",
                    model = repo.rsplit('/').next().unwrap_or(repo)
                ),
            ),
            (BackendState::Ready { .. }, EmbedBackend::Server(s)) => {
                let name = if !s.model.trim().is_empty() {
                    s.model.trim().to_string()
                } else {
                    self.model_list
                        .found
                        .default
                        .clone()
                        .filter(|d| !d.is_empty())
                        .unwrap_or_else(|| {
                            t!("status-model-server-default", server = domain(&s.base_url))
                        })
                };
                (GREEN, name)
            }
            (BackendState::Off, EmbedBackend::None) => (Color32::GRAY, t!("status-model-off")),
            (BackendState::Off, _) => (RED, t!("status-model-unavailable")),
            (BackendState::Ready { .. }, EmbedBackend::None) => {
                (Color32::GRAY, t!("status-model-words-only"))
            }
        }
    }

    pub(super) fn status_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin::symmetric(16, 6))
                    .fill(fill),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    let weak = ui.visuals().weak_text_color();
                    // The item count goes at the far end. It is laid out first,
                    // so a long error before it can be cut to the room left.
                    let pinned = self.library.items().iter().filter(|i| i.pinned).count();
                    let n = self.library.items().len();
                    let count = if pinned > 0 {
                        t!("status-items-pinned", count = n, pinned = pinned)
                    } else {
                        t!("status-items", count = n)
                    };
                    let count = txt(&count, 12.0, Some(weak)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::FontSelection::Default,
                    );
                    let (dot, text) = self.model_status();
                    let color = if dot == Color32::GRAY { weak } else { dot };
                    let job = pill_job(&text, 12.0, color, None);
                    let r = tip(
                        icons::pill(ui, Icon::Dot, job, color, false),
                        &t!("status-model-tip"),
                    );
                    if r.clicked() {
                        self.open_settings(SettingsPage::Search);
                    }
                    if self.queue > 0 {
                        let total = self.queue_total.max(self.queue);
                        let done = total - self.queue;
                        slow_spinner(ui, 11.0, Some(weak));
                        ui.label(txt(
                            &t!("status-indexing", done = done, total = total),
                            12.0,
                            Some(weak),
                        ));
                        ui.add(
                            egui::ProgressBar::new(done as f32 / total as f32)
                                .desired_width(90.0)
                                .desired_height(6.0),
                        );
                    }
                    if self.imports_running > 0 {
                        slow_spinner(ui, 11.0, Some(weak));
                        ui.label(txt(
                            &t!("status-adding-files", count = self.imported),
                            12.0,
                            Some(weak),
                        ));
                    }
                    if !self.status.is_empty() {
                        ui.label(txt(&self.status, 12.0, Some(weak)));
                    }
                    if !self.flags.enabled.load(Ordering::SeqCst) {
                        icons::chip(ui, Icon::Pause, &t!("status-recording-paused"), AMBER);
                    }
                    // A global shortcut that doesn't work must be visible here
                    // too, not only in Settings.
                    if let Some(keys) = self.hotkey_conflicts.first().cloned() {
                        icons::glyph(ui, Icon::Warning, 13.0, AMBER);
                        let text = t!("status-shortcut-taken", keys = keys);
                        let r = ui.add(
                            egui::Label::new(txt(&text, 12.0, Some(AMBER))).sense(Sense::click()),
                        );
                        if tip(r, &t!("status-open-shortcut-settings")).clicked() {
                            self.open_settings(SettingsPage::Shortcuts);
                        }
                    }
                    if let Some((e, _)) = &self.error {
                        icons::glyph(ui, Icon::Warning, 13.0, RED);
                        let room = ui.available_width()
                            - count.size().x
                            - 2.0 * ui.spacing().item_spacing.x;
                        let r = cut_line(ui, e, room.max(0.0), 12.0, RED, Sense::click());
                        if tip(r, &t!("status-dismiss-error")).clicked() {
                            self.error = None;
                        }
                    }
                    far(ui, |ui| ui.label(count));
                });
            });
    }

    pub(super) fn toasts(&mut self, ctx: &egui::Context) {
        let mut clicked = false;
        if let Some(toast) = &self.toast {
            if toast.shown.elapsed() > toast.how_long {
                self.toast = None;
                return;
            }
            let dark = ctx.theme() == egui::Theme::Dark;
            let fg = if dark {
                Color32::from_gray(20)
            } else {
                Color32::WHITE
            };
            egui::Area::new(egui::Id::unique("toast"))
                .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -48.0))
                .show(ctx, |ui| {
                    Frame::new()
                        .fill(if dark {
                            Color32::from_gray(235)
                        } else {
                            Color32::from_gray(40)
                        })
                        .corner_radius(CornerRadius::same(18))
                        .inner_margin(Margin::symmetric(16, 8))
                        .show(ui, |ui| {
                            row(ui, |ui| {
                                ui.label(txt(toast.text.as_str(), 14.0, Some(fg)));
                                if let Some((label, _)) = &toast.action {
                                    let button = egui::Button::new(txt(label, 13.0, Some(fg)))
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0, fg))
                                        .corner_radius(CornerRadius::same(12));
                                    clicked = ui.add(button).clicked();
                                }
                            });
                        });
                });
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if clicked {
            if let Some((_, command)) = self.toast.take().and_then(|t| t.action) {
                self.run(command);
            }
        }
        if let Some((_, at)) = &self.error {
            if at.elapsed() > Duration::from_secs(20) {
                self.error = None;
            }
        }
    }
}
