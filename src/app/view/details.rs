//! The details pane and the tag editor.

use super::*;

/// Shows a file or folder in the system's file manager.
pub(crate) fn reveal(p: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let started = if p.is_dir() {
        std::process::Command::new("explorer").arg(p).spawn()
    } else {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", p.display()))
            .spawn()
    };
    #[cfg(target_os = "macos")]
    let started = std::process::Command::new("open").arg("-R").arg(p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let started = {
        let dir = if p.is_dir() {
            p
        } else {
            p.parent().unwrap_or(p)
        };
        std::process::Command::new("xdg-open").arg(dir).spawn()
    };
    started.map(|_| ())
}

impl App {
    /// Tag pills with remove buttons, then an input for adding more.
    pub(super) fn tag_editor(&mut self, ui: &mut Ui, it: &Item) {
        let mut remove: Option<String> = None;
        let mut add: Option<String> = None;
        let mut filter: Option<String> = None;
        let mut close_input = false;
        row_wrapped(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for t in &it.tags {
                let c = tag_color(t);
                Frame::new()
                    .fill(c.gamma_multiply(if ui.visuals().dark_mode { 0.25 } else { 0.13 }))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin {
                        left: 10,
                        right: 4,
                        top: 2,
                        bottom: 2,
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if tip(
                            ui.add(
                                egui::Label::new(txt(&format!("# {t}"), 12.5, Some(c)))
                                    .sense(Sense::click()),
                            ),
                            &t!("details-only-this-tag"),
                        )
                        .clicked()
                        {
                            filter = Some(t.clone());
                        }
                        if icon_button_ex(
                            ui,
                            Icon::Close,
                            &t!("details-remove-tag"),
                            Vec2::splat(18.0),
                            8.0,
                            None,
                            Some(c),
                        )
                        .clicked()
                        {
                            remove = Some(t.clone());
                        }
                    });
            }
            let editing = self.tag_input.as_ref().is_some_and(|(id, _)| *id == it.id);
            if editing {
                let known = self.library.all_tags();
                let id = egui::Id::unique(("tag-input", it.id));
                // Read Enter while the field still has focus: egui reports the
                // lost focus a frame later, when the key is no longer "pressed".
                let enter =
                    ui.memory(|m| m.has_focus(id)) && ui.input(|i| i.key_pressed(Key::Enter));
                let (_, text) = self.tag_input.as_mut().unwrap();
                let r = ui.add(
                    egui::TextEdit::singleline(text)
                        .id(id)
                        .hint_text(txt(&t!("details-tag-hint"), 13.0, None))
                        .desired_width(150.0)
                        .font(FontId::proportional(13.0)),
                );
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
                let mut suggestions = known
                    .iter()
                    .filter(|(k, _)| {
                        !it.has_tag(k) && (typed.is_empty() || k.to_lowercase().starts_with(&typed))
                    })
                    .take(5)
                    .peekable();
                if suggestions.peek().is_some() {
                    for (k, _) in suggestions {
                        if ui
                            .add(
                                egui::Button::new(txt(&format!("+ {k}"), 12.0, Some(tag_color(k))))
                                    .frame_when_inactive(false),
                            )
                            .clicked()
                        {
                            add = Some(k.clone());
                        }
                    }
                }
            } else {
                let job = pill_job(
                    &if it.tags.is_empty() {
                        t!("details-add-tag")
                    } else {
                        t!("details-tag")
                    },
                    12.5,
                    ui.visuals().weak_text_color(),
                    None,
                );
                if tip(
                    icons::pill(ui, Icon::Add, job, accent(), false),
                    &hint(&t!("details-add-tag-tip"), &self.cfg.keys.tag),
                )
                .clicked()
                {
                    self.tag_input = Some((it.id, String::new()));
                    self.tag_focus = true;
                }
            }
        });
        if close_input {
            self.tag_input = None;
        }
        if let Some(t) = remove {
            self.run(Command::RemoveTag(it.id, t));
        }
        if let Some(t) = add {
            self.run(Command::AddTags(it.id, t));
        }
        if let Some(t) = filter {
            self.tag_filter = Some(t);
            self.dirty = true;
        }
    }

    /// The selected item in full. `closeable` when it is a side pane in Grid or
    /// List view, which gets a back button.
    pub(super) fn details(&mut self, ui: &mut Ui, closeable: bool) {
        let ctx = ui.ctx().clone();
        let Some(it) = self.selected.and_then(|id| self.library.get(id)).cloned() else {
            self.empty_details(ui);
            return;
        };
        // Header: what it is at the reading start, what you can do at the far end.
        row(ui, |ui| {
            if closeable
                && icons::icon_button_ex(
                    ui,
                    Icon::Back,
                    &t!("details-close"),
                    Vec2::splat(30.0),
                    20.0,
                    None,
                    None,
                )
                .clicked()
            {
                self.details_open = false;
            }
            icons::chip(
                ui,
                icons::kind(it.kind),
                &it.kind.label(),
                kind_color(it.kind),
            );
            if it.pinned {
                icons::chip(ui, Icon::PinFilled, &t!("details-pinned"), accent());
            }
            if it.origin == Origin::Imported {
                icons::chip(ui, Icon::Folder, &t!("details-from-files"), Color32::GRAY);
            }
            if it.formatted {
                icons::chip(ui, Icon::Text, &t!("details-formatted"), Color32::GRAY);
            }
            far(ui, |ui| {
                let keys = self.cfg.keys.clone();
                if icons::icon_button(ui, Icon::Delete, &hint(&t!("details-delete"), &keys.delete))
                    .clicked()
                {
                    self.run(Command::Delete(it.id));
                }
                let (pin, tip) = if it.pinned {
                    (Icon::PinFilled, hint(&t!("details-unpin"), &keys.pin))
                } else {
                    (Icon::Pin, hint(&t!("details-pin-keep"), &keys.pin))
                };
                let pin_color = it.pinned.then(accent);
                if icon_button_ex(ui, pin, &tip, Vec2::splat(34.0), 16.0, None, pin_color).clicked()
                {
                    self.run(Command::Pin(it.id));
                }
                if it.kind == Kind::Image
                    && icons::icon_button(ui, Icon::Save, &t!("details-save-image")).clicked()
                {
                    if let (Some(src), Some(dst)) = (
                        &it.image,
                        rfd::FileDialog::new()
                            .set_file_name(format!("clip-{}.png", it.id))
                            .save_file(),
                    ) {
                        let images = &self.library.store().images;
                        if let Err(e) = images.open(it.id, src).and_then(|i| Ok(i.save(&dst)?)) {
                            self.set_error(log_error!("error-saving-image", error = e.to_string()));
                        }
                    }
                    self.after_dialog();
                }
                if let Some(p) = &it.source {
                    if icons::icon_button(ui, Icon::Folder, &t!("details-show-in-folder")).clicked()
                    {
                        if let Err(e) = reveal(p) {
                            self.set_error(log_error!(
                                "error-opening-folder",
                                error = e.to_string()
                            ));
                        }
                    }
                }
                if it.kind == Kind::Link
                    && icons::icon_button(ui, Icon::Browser, &t!("details-open-in-browser"))
                        .clicked()
                {
                    ctx.open_url(egui::OpenUrl::new_tab(it.text.trim()));
                }
                let copy_tip = t!(
                    "details-copy-tip",
                    action = hint(&t!("details-copy"), &keys.copy)
                );
                if icon_button_ex(
                    ui,
                    Icon::Copy,
                    &copy_tip,
                    Vec2::new(44.0, 34.0),
                    16.0,
                    Some(accent()),
                    Some(Color32::WHITE),
                )
                .clicked()
                {
                    self.run(Command::Copy(it.id));
                }
            });
        });
        ui.add_space(2.0);
        let meta = self.meta_line(&it);
        let weak = ui.visuals().weak_text_color();
        row(ui, |ui| ui.label(txt(&meta, 12.5, Some(weak))));
        if let Some(app) = &it.app {
            let ignored = self.cfg.rules.iter().any(|r| r.skips_app(app));
            let mut ignore = false;
            row(ui, |ui| {
                ui.label(txt(
                    &t!("details-copied-from", app = app.as_str()),
                    12.5,
                    Some(weak),
                ));
                if !ignored {
                    let link = t!("details-ignore-app", app = app.as_str());
                    ignore = ui.link(txt(&link, 12.5, None)).clicked();
                }
            });
            if ignore {
                self.ignore_app(app);
            }
        }
        ui.add_space(6.0);
        self.tag_editor(ui, &it);
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match it.kind {
                Kind::Image => self.image_details(ui, &it),
                Kind::Files => {
                    for line in it.text.lines() {
                        let p = PathBuf::from(line);
                        row(ui, |ui| {
                            icons::glyph(
                                ui,
                                if p.is_dir() { Icon::Folder } else { Icon::File },
                                15.0,
                                ui.visuals().text_color(),
                            );
                            let name = p
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_else(|| line.to_string());
                            if tip(ui.link(txt(&name, 14.5, None)), line).clicked() {
                                if let Err(e) = reveal(&p) {
                                    self.set_error(log_error!(
                                        "error-opening-folder",
                                        error = e.to_string()
                                    ));
                                }
                            }
                            let parent = p
                                .parent()
                                .map(|x| x.display().to_string())
                                .unwrap_or_default();
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
                        Frame::new()
                            .fill(code_bg)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(d.shown.as_str()).monospace().size(13.0),
                                    )
                                    .selectable(true),
                                );
                            });
                    } else if bidi::has_rtl(&d.shown) {
                        // Arabic: wrapped in reading order (not selectable; copy takes all of it).
                        let shown = d.shown.clone();
                        let c = ui.visuals().text_color();
                        bidi::paragraph(ui, &shown, FontId::proportional(15.0), c);
                    } else {
                        ui.add(
                            egui::Label::new(RichText::new(d.shown.as_str()).size(15.0))
                                .selectable(true)
                                .wrap(),
                        );
                    }
                }
            });
    }

    pub(super) fn meta_line(&mut self, it: &Item) -> String {
        let mut parts = vec![when(it.created)];
        match it.kind {
            Kind::Text => {
                let d = self.detail_text(it);
                let (c, w, code) = (d.chars, d.words, d.monospace);
                if code {
                    parts.push(t!("details-code-lines", count = it.text.lines().count()));
                }
                parts.push(t!("details-characters", count = c));
                parts.push(t!("details-words", count = w));
            }
            Kind::Image => {
                parts.push(t!(
                    "details-image-size",
                    width = it.width,
                    height = it.height
                ));
                match it.ocr.as_deref() {
                    Some(o) if !o.trim().is_empty() => parts.push(t!("details-has-text")),
                    Some(_) => parts.push(t!("details-no-text")),
                    None if self.cfg.ocr_images => parts.push(t!("details-reading-text")),
                    None => {}
                }
            }
            Kind::Link => parts.push(domain(&it.text).to_string()),
            Kind::Files => parts.push(t!("details-files-count", count = it.text.lines().count())),
        }
        if let Some(src) = &it.source {
            parts.push(src.display().to_string());
        }
        parts.join("  ·  ")
    }

    pub(super) fn image_details(&mut self, ui: &mut Ui, it: &Item) {
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
                lbl(
                    ui,
                    &t!("details-image-missing"),
                    14.0,
                    ui.visuals().weak_text_color(),
                );
            }
        }
        if let Some(t) = it.ocr.as_deref().filter(|t| !t.trim().is_empty()) {
            ui.add_space(16.0);
            row(ui, |ui| {
                lbl(
                    ui,
                    &t!("details-text-in-image"),
                    11.5,
                    ui.visuals().weak_text_color(),
                );
                if icon_button_ex(
                    ui,
                    Icon::Copy,
                    &t!("details-copy-text"),
                    Vec2::splat(26.0),
                    13.0,
                    None,
                    None,
                )
                .clicked()
                {
                    self.run(Command::CopyText(t.to_string()));
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

    pub(super) fn empty_details(&self, ui: &mut Ui) {
        ui.add_space(80.0);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            lbl(ui, &t!("details-select-an-item"), 14.0, weak);
            ui.add_space(18.0);
            let k = &self.cfg.keys;
            let rows = [
                ("Up+Down", t!("details-key-move")),
                (k.copy.as_str(), t!("details-key-copy")),
                (k.pin.as_str(), t!("details-key-pin")),
                (k.tag.as_str(), t!("details-key-tag")),
                (k.delete.as_str(), t!("details-key-delete")),
                ("Escape", t!("details-key-clear")),
            ];
            for (keys, what) in rows.into_iter().filter(|(k, _)| !k.trim().is_empty()) {
                row(ui, |ui| {
                    // Keys hug a shared centre line from one side, actions from the other.
                    let half = ui.available_width() / 2.0;
                    let (first, _) =
                        ui.allocate_exact_size(Vec2::new(half - 8.0, 28.0), Sense::hover());
                    let mut caps =
                        ui.new_child(egui::UiBuilder::new().max_rect(first).layout(far_layout()));
                    // A right-to-left child lays the caps out backwards; undo that.
                    let keys = if i18n::rtl() {
                        keys.to_string()
                    } else {
                        keys.split('+').rev().collect::<Vec<_>>().join("+")
                    };
                    shortcut::keycaps(&mut caps, &keys, false);
                    ui.add_space(8.0);
                    lbl(ui, &what, 12.5, weak);
                });
            }
        });
    }
}
