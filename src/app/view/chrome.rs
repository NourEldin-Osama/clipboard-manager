//! The status bar.

use super::*;

/// A model's name as a server reports it, short enough for the status bar:
/// llama-server reports its model file's whole path, which shows as the
/// file's name. Names such as `org/model` stay as they are.
fn short_model_name(name: &str) -> &str {
    let is_path = name.contains('\\') || name.starts_with('/') || name.ends_with(".gguf");
    if is_path {
        name.rsplit(['\\', '/']).next().unwrap_or(name)
    } else {
        name
    }
}

impl App {
    /// The search model in use: its name and its server's, as menus show
    /// them, and its name on the server, which may be long.
    fn search_model_names(&self) -> Option<(String, String, String)> {
        let at = self.cfg.search_model?;
        let (server, model) = self.cfg.model(at)?;
        let learned = self
            .model_list
            .found
            .default
            .clone()
            .filter(|name| !name.is_empty());
        let id = match model.model.trim() {
            "" => learned
                .clone()
                .unwrap_or_else(|| t!("settings-server-default")),
            id => id.to_string(),
        };
        let name = match (model.name.trim(), &learned) {
            ("", Some(learned)) if model.model.trim().is_empty() => {
                short_model_name(learned).to_string()
            }
            _ => super::settings::model_label(model),
        };
        Some((name, server.label().to_string(), id))
    }

    pub(crate) fn model_status(&self) -> (Color32, String) {
        let on = self.cfg.search_engine == SearchEngine::Server;
        if on && self.search_retry.is_some() {
            return (RED, t!("status-model-failing"));
        }
        match (&self.backend, self.search_model_names()) {
            (BackendState::Loading, _) => (AMBER, t!("status-model-loading")),
            // The model's name is in the tip: the words say what it does.
            (BackendState::Ready { .. }, Some(_)) if on => (GREEN, t!("status-model-on")),
            (BackendState::Off, _) if !on => (Color32::GRAY, t!("status-model-off")),
            // On with no model picked yet: nothing is wrong, there is none.
            (BackendState::Off, None) => (Color32::GRAY, t!("status-model-none")),
            (BackendState::Off, _) => (RED, t!("status-model-unavailable")),
            (BackendState::Ready { .. }, _) => (Color32::GRAY, t!("status-model-words-only")),
        }
    }

    /// The state of search by meaning in words, for its part of Settings,
    /// where its model shows below.
    pub(crate) fn meaning_state(&self) -> (Color32, String) {
        let on = self.cfg.search_engine == SearchEngine::Server;
        if on && self.search_retry.is_some() {
            return (RED, t!("status-model-failing"));
        }
        let picked = self.cfg.search_model.and_then(|at| self.cfg.model(at));
        match (&self.backend, picked) {
            _ if !on => (Color32::GRAY, t!("settings-meaning-off")),
            (BackendState::Loading, _) => (AMBER, t!("status-model-loading")),
            (_, None) => (Color32::GRAY, t!("settings-meaning-none")),
            (BackendState::Ready { .. }, Some(_)) => (GREEN, t!("settings-meaning-ready")),
            (BackendState::Off, Some(_)) => (RED, t!("settings-meaning-unavailable")),
        }
    }

    /// The status bar's hint: the search model's full name, when there is one.
    fn model_tip(&self) -> String {
        if self.search_retry.is_some() {
            return t!("status-model-failing-tip");
        }
        match self.search_model_names() {
            Some((_, server, id)) if matches!(self.backend, BackendState::Ready { .. }) => {
                t!("status-model-tip-named", model = id, server = server)
            }
            _ => t!("status-model-tip"),
        }
    }

    pub(super) fn status_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        let bar = egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin::symmetric(16, 6))
                    .fill(fill),
            )
            .show(ui, |ui| {
                let group = self.group();
                if !group.is_empty() {
                    self.group_bar(ui, &group);
                    return;
                }
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
                    let count = txt(&count, size::SMALL, Some(weak)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::FontSelection::Default,
                    );
                    let (dot, text) = self.model_status();
                    let color = if dot == Color32::GRAY {
                        weak
                    } else {
                        text_shade(dot, ui.visuals().dark_mode)
                    };
                    let job = pill_job(&text, size::SMALL, color, None);
                    // The dot has the state's color, as the words do.
                    let r = tip(
                        icons::state_pill(ui, Icon::Dot, job, color),
                        &self.model_tip(),
                    );
                    if r.clicked() {
                        if self.search_retry.is_some() {
                            self.run(Command::TrySearchAgain);
                        } else {
                            self.open_settings(SettingsPage::Search);
                        }
                    }
                    if self.queue > 0 {
                        let total = self.queue_total.max(self.queue);
                        let done = total - self.queue;
                        slow_spinner(ui, 11.0, Some(weak));
                        ui.label(txt(
                            &t!("status-indexing", done = done, total = total),
                            size::SMALL,
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
                            size::SMALL,
                            Some(weak),
                        ));
                    }
                    if !self.status.is_empty() {
                        ui.label(txt(&self.status, size::SMALL, Some(weak)));
                    }
                    if !self.reader_status.is_empty() {
                        slow_spinner(ui, 11.0, Some(weak));
                        ui.label(txt(&self.reader_status, size::SMALL, Some(weak)));
                    }
                    if !self.flags.enabled.load(Ordering::SeqCst) {
                        icons::chip(ui, Icon::Pause, &t!("status-recording-paused"), AMBER);
                    }
                    if self.recording_stopped {
                        let text = t!("status-recording-stopped");
                        let r = icons::chip(ui, Icon::Warning, &text, RED).interact(Sense::click());
                        set_name(&r, &text);
                        if tip(r, &t!("status-recording-stopped-tip")).clicked() {
                            self.run(Command::StartRecordingAgain);
                        }
                    }
                    // A global shortcut that doesn't work must be visible here
                    // too, not only in Settings.
                    if let Some(keys) = self.hotkey_conflicts.first().cloned() {
                        let amber = text_shade(AMBER, ui.visuals().dark_mode);
                        icons::glyph(ui, Icon::Warning, 13.0, amber);
                        let text = t!("status-shortcut-taken", keys = keys);
                        let r = ui.add(
                            egui::Label::new(txt(&text, size::SMALL, Some(amber)))
                                .sense(Sense::click()),
                        );
                        if tip(r, &t!("status-open-shortcut-settings")).clicked() {
                            self.open_settings(SettingsPage::Shortcuts);
                        }
                    }
                    if let Some((e, _)) = &self.error {
                        let red = text_shade(RED, ui.visuals().dark_mode);
                        icons::glyph(ui, Icon::Warning, 13.0, red);
                        let room = ui.available_width()
                            - count.size().x
                            - 2.0 * ui.spacing().item_spacing.x;
                        let r = cut_line(ui, e, room.max(0.0), size::SMALL, red, Sense::click());
                        if tip(r, &t!("status-dismiss-error")).clicked() {
                            self.error = None;
                        }
                    }
                    far(ui, |ui| ui.label(count));
                });
            });
        // A thin line between the items and the bar: the items go on under it.
        let rect = bar.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            Stroke::new(1.0, line_color(ui.visuals().dark_mode)),
        );
    }

    /// While several items are selected, the status bar holds what can be
    /// done to them all, each with its key.
    fn group_bar(&mut self, ui: &mut Ui, group: &[u64]) {
        let k = self.cfg.keys.clone();
        let v = ui.visuals().clone();
        let all_pinned = group
            .iter()
            .all(|&id| self.library.get(id).is_some_and(|it| it.pinned));
        let (mut command, mut tags, mut clear) = (None, None, false);
        let count = t!("group-selected", count = group.len());
        let count = selected_count(ui, &count, v.strong_text_color());
        // As tall as the pills from the start, so the count sits on their line.
        row_of(ui, icons::pill_height(count.size().y), |ui| {
            ui.label(count);
            ui.add_space(space::MEDIUM);
            let button = |ui: &mut Ui, icon: Icon, text: &str, keys: &str, color: Color32| {
                let job = pill_job(text, size::SMALL, color, None);
                tip(icons::pill(ui, icon, job, color, false), &hint(text, keys)).clicked()
            };
            let plain = v.text_color();
            let pin = if all_pinned {
                t!("group-unpin")
            } else {
                t!("group-pin")
            };
            if button(ui, Icon::Pin, &pin, &k.pin, plain) {
                command = Some(Command::PinGroup(group.to_vec()));
            }
            if button(ui, Icon::Tag, &t!("group-tag"), &k.tag, plain) {
                self.group_tag = Some(String::new());
            }
            let copy = t!("group-copy");
            if button(ui, Icon::Copy, &copy, &k.copy_only, plain) {
                command = Some(Command::CopyTogether(group.to_vec(), Paste::No));
            }
            let red = text_shade(RED, v.dark_mode);
            if button(ui, Icon::Delete, &t!("group-delete"), &k.delete, red) {
                command = Some(Command::DeleteGroup(group.to_vec()));
            }
            if let Some(text) = self.group_tag.as_mut() {
                // Open until Enter adds the tags, or Esc or another
                // selection closes it, so it keeps the keyboard.
                let id = egui::Id::unique("group-tag");
                let enter =
                    ui.memory(|m| m.has_focus(id)) && ui.input(|i| i.key_pressed(Key::Enter));
                let hint_text = t!("details-tag-hint");
                let r = ui.add(
                    egui::TextEdit::singleline(text)
                        .id(id)
                        .hint_text(txt(&hint_text, size::SMALL, None))
                        .desired_width(150.0)
                        .font(FontId::proportional(size::SMALL)),
                );
                set_name(&r, &hint_text);
                r.request_focus();
                if enter && !text.trim().is_empty() {
                    tags = Some(text.clone());
                }
            }
            far(ui, |ui| {
                let link = txt(&t!("group-clear"), size::SMALL, None);
                clear = tip(ui.link(link), &hint(&t!("group-clear"), "Escape")).clicked();
            });
        });
        if let Some(command) = command {
            self.run(command);
        }
        if let Some(tags) = tags {
            self.tag_group(&tags);
        }
        if clear {
            self.clear_marks();
        }
    }
}

/// The count of selected items in SemiBold, set like the pills' labels so it
/// sits on their line: a plain label's letters sit a point lower in its box.
fn selected_count(ui: &Ui, text: &str, color: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = pill_job(text, size::SMALL, color, None);
    for section in &mut job.sections {
        section.format.font_id = semibold(size::SMALL);
    }
    ui.fonts_mut(|f| f.layout_job(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where each text that `add` paints has its baseline, in points from
    /// the top, at `scale` pixels per point.
    fn baselines(scale: f32, add: impl Fn(&mut Ui)) -> Vec<f32> {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(scale);
        add_fonts(&ctx);
        // The bundled fonts apply from the second frame.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| add(ui));
        output.textures_delta.clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| add(ui));
        output.textures_delta.clear();
        assert_eq!(ctx.pixels_per_point(), scale);
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    let row = text.galley.rows.first()?;
                    let glyph = row.row.glyphs.first()?;
                    Some(text.pos.y + row.pos.y + glyph.pos.y)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_selected_count_sits_on_the_line_of_the_buttons() {
        for scale in [1.0, 1.25] {
            let lines = baselines(scale, |ui| {
                let color = ui.visuals().text_color();
                let count = selected_count(ui, "2 selected", color);
                row_of(ui, icons::pill_height(count.size().y), |ui| {
                    ui.label(count);
                    let job = pill_job("Pin", size::SMALL, color, None);
                    icons::pill(ui, Icon::Pin, job, color, false);
                });
            });
            assert_eq!(lines.len(), 2, "two texts at {scale} pixels per point");
            assert_eq!(lines[0], lines[1], "at {scale} pixels per point");
        }
    }

    #[test]
    fn a_model_file_shows_by_its_name() {
        assert_eq!(
            short_model_name(r"C:\models\embeddinggemma-2-UD-Q4_K_XL.gguf"),
            "embeddinggemma-2-UD-Q4_K_XL.gguf"
        );
        assert_eq!(short_model_name("/models/nomic.gguf"), "nomic.gguf");
        assert_eq!(
            short_model_name("openai/text-embedding-3-small"),
            "openai/text-embedding-3-small"
        );
        assert_eq!(short_model_name("bge-small-en-v1.5"), "bge-small-en-v1.5");
    }
}
