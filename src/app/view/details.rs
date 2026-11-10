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

/// What the actions, the facts and the tags below the content take, about:
/// the content scrolls above them.
const FACTS_HEIGHT: f32 = 130.0;

/// Between the facts of the details.
const FACTS_GAP: &str = "  ·  ";

/// How big the details show a text: short ones larger, so they read first.
fn content_size(text: &str) -> f32 {
    let (chars, lines) = (text.chars().count(), text.lines().count());
    if chars <= 60 && lines <= 2 {
        size::TITLE
    } else if chars <= 160 && lines <= 4 {
        size::LARGE
    } else {
        size::BODY
    }
}

/// An item's text to read: Arabic wrapped in reading order (not
/// selectable; copy takes all of it) on the side its first letter reads
/// from; other text in lines of about 70 characters on the left, as
/// left-to-right text reads, in Arabic too.
fn reading_text(ui: &mut Ui, text: &str) {
    let text_size = content_size(text);
    // The pane's main content, in the strongest text color.
    let color = ui.visuals().strong_text_color();
    if bidi::has_rtl(text) {
        bidi::content(ui, text, FontId::proportional(text_size), color);
        return;
    }
    let room = ui.available_rect_before_wrap();
    let width = room.width().min(text_size * READING_WIDTH);
    let lines = Rect::from_min_size(room.min, Vec2::new(width, room.height()));
    let builder = egui::UiBuilder::new()
        .max_rect(lines)
        .layout(Layout::top_down(Align::Min));
    ui.scope_builder(builder, |ui| {
        ui.add(
            egui::Label::new(
                RichText::new(text)
                    .size(text_size)
                    .color(color)
                    .line_height(Some(text_size * 1.6)),
            )
            .selectable(true)
            .wrap(),
        );
    });
}

/// How tall the actions under the content are.
const BUTTON_HEIGHT: f32 = 34.0;

/// The details' main button: the main way to put the item back, with its
/// key, and an arrow to the other ways when there are any. A long program
/// name is cut short when the header has no room for it.
fn main_button(ui: &mut Ui, ways: &[CopyWay]) -> Option<Paste> {
    let (main, others) = ways.split_first()?;
    let height = BUTTON_HEIGHT;
    let arrow_w = if others.is_empty() { 0.0 } else { 30.0 };
    // The key nearly white, so it reads on the accent; still a little
    // quieter than the button's word.
    let key_color = theme::on_button(225);
    let keys = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            main.keys.clone(),
            FontId::proportional(size::SMALL),
            key_color,
        )
    });
    let keys_w = if main.keys.trim().is_empty() {
        0.0
    } else {
        keys.size().x + 10.0
    };
    let room = ui.available_width() - 8.0 - 28.0 - keys_w - arrow_w;
    let font = FontId::proportional(size::BODY);
    let label = one_line(ui, &main.text, &[], font, Color32::WHITE, room.max(60.0));
    let main_w = 14.0 + label.size().x + keys_w + 14.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(main_w + arrow_w, height), Sense::hover());
    // Laid out left to right, then mirrored for Arabic.
    let part = |x: f32, w: f32| {
        flip(
            rect,
            Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, height)),
        )
    };
    let main_rect = part(rect.left(), main_w);
    let arrow_rect = part(rect.left() + main_w, arrow_w);
    // Each part rounds only its outer corners.
    let (start, end) = (
        CornerRadius {
            nw: 8,
            sw: 8,
            ne: 0,
            se: 0,
        },
        CornerRadius {
            nw: 0,
            sw: 0,
            ne: 8,
            se: 8,
        },
    );
    let (main_round, arrow_round) = match (others.is_empty(), i18n::rtl()) {
        (true, _) => (CornerRadius::same(radius::MEDIUM), CornerRadius::ZERO),
        (false, false) => (start, end),
        (false, true) => (end, start),
    };
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(radius::MEDIUM),
        button_fill(accent()),
    );
    let resp = ui.interact(
        main_rect,
        egui::Id::unique("details-main-button"),
        Sense::click(),
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &main.text));
    let lit = |resp: &egui::Response, round: CornerRadius, at: Rect| {
        if resp.hovered() {
            let pressed = resp.is_pointer_button_down_on();
            let shade = theme::lit(pressed);
            ui.painter().rect_filled(at, round, shade);
        }
    };
    lit(&resp, main_round, main_rect);
    let at = |x: f32, size: Vec2| {
        flip(
            rect,
            Rect::from_min_size(Pos2::new(x, rect.center().y - size.y / 2.0), size),
        )
        .min
    };
    let label_size = label.size();
    ui.painter()
        .galley(at(rect.left() + 14.0, label_size), label, Color32::WHITE);
    if keys_w > 0.0 {
        let x = rect.left() + 14.0 + label_size.x + 10.0;
        ui.painter().galley(at(x, keys.size()), keys, key_color);
    }
    let mut chosen = tip(resp, &hint(&main.text, &main.keys))
        .clicked()
        .then_some(main.paste);
    if !others.is_empty() {
        let more = t!("details-more-ways");
        let arrow = ui.interact(
            arrow_rect,
            egui::Id::unique("details-more-ways"),
            Sense::click(),
        );
        arrow.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &more));
        lit(&arrow, arrow_round, arrow_rect);
        // A thin line between the two parts.
        let edge = if i18n::rtl() {
            arrow_rect.right()
        } else {
            arrow_rect.left()
        };
        ui.painter().rect_filled(
            Rect::from_center_size(
                Pos2::new(edge, rect.center().y),
                Vec2::new(1.0, height - 14.0),
            ),
            0.0,
            theme::on_button(70),
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(arrow_rect.center(), Vec2::splat(14.0)),
            Icon::Dropdown,
            Color32::WHITE,
        );
        let arrow = tip(arrow, &more);
        egui::Popup::menu(&arrow).show(|ui| {
            let width = menu_width(
                ui,
                others.iter().map(|w| (w.text.as_str(), w.keys.as_str())),
            );
            ui.set_width(width);
            for way in others {
                if menu_entry(ui, &way.text, &way.keys, None, None).clicked() {
                    chosen = Some(way.paste);
                }
            }
        });
    }
    chosen
}

impl App {
    /// Opens the tag field of an item in the details pane.
    pub(super) fn start_tagging(&mut self, id: u64) {
        self.selected = Some(id);
        self.tag_input = Some((id, String::new()));
        self.tag_focus = true;
        self.details_open = true;
    }

    /// Asks where to save an image item, and saves it there as PNG.
    pub(super) fn save_image_as(&mut self, it: &Item) {
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

    /// Shows a file or folder in the system's file manager.
    pub(super) fn show_in_folder(&mut self, place: &std::path::Path) {
        if let Err(e) = reveal(place) {
            self.set_error(log_error!("error-opening-folder", error = e.to_string()));
        }
    }

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
                let shade = text_shade(c, ui.visuals().dark_mode);
                Frame::new()
                    .fill(c.gamma_multiply(if ui.visuals().dark_mode { 0.3 } else { 0.13 }))
                    .corner_radius(CornerRadius::same(radius::LARGE))
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
                                egui::Label::new(txt(&format!("# {t}"), size::SMALL, Some(shade)))
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
                            Some(shade),
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
                        .hint_text(txt(&t!("details-tag-hint"), size::CONTROL, None))
                        .desired_width(150.0)
                        .font(FontId::proportional(size::CONTROL)),
                );
                set_name(&r, &t!("details-tag-hint"));
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
                                egui::Button::new(txt(
                                    &format!("+ {k}"),
                                    size::SMALL,
                                    Some(text_shade(tag_color(k), ui.visuals().dark_mode)),
                                ))
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
                    size::SMALL,
                    ui.visuals().weak_text_color(),
                    None,
                );
                // A thin outline, so the pill's edge lines up with the actions
                // above it.
                let pill = icons::pill(ui, Icon::Add, job, accent(), false);
                ui.painter().rect_stroke(
                    pill.rect,
                    CornerRadius::same(radius::ROUND),
                    Stroke::new(1.0, line_color(ui.visuals().dark_mode)),
                    egui::StrokeKind::Inside,
                );
                if tip(pill, &hint(&t!("details-add-tag-tip"), &self.cfg.keys.tag)).clicked() {
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
        // Back, while the details cover the list. The copied content comes
        // first, then what you can do with it, one line of facts and the tags.
        if closeable {
            row(ui, |ui| {
                if icons::icon_button_ex(
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
            });
        }
        ui.add_space(space::SMALL);
        // Long content scrolls above the facts and tags, which stay in view.
        let room = (ui.available_height() - FACTS_HEIGHT).max(FACTS_HEIGHT);
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(room)
            .show(ui, |ui| match it.kind {
                Kind::Image => self.image_details(ui, &it),
                Kind::Files => {
                    // Checked once per item: the pane draws every frame,
                    // and a path on an unreachable drive can take seconds.
                    if self.folders_of.as_ref().is_none_or(|(id, _)| *id != it.id) {
                        let folders = it
                            .text
                            .lines()
                            .map(|l| std::path::Path::new(l).is_dir())
                            .collect();
                        self.folders_of = Some((it.id, folders));
                    }
                    let folders = self
                        .folders_of
                        .as_ref()
                        .map(|(_, f)| f.clone())
                        .unwrap_or_default();
                    for (index, line) in it.text.lines().enumerate() {
                        let p = PathBuf::from(line);
                        let is_folder = folders.get(index).copied().unwrap_or(false);
                        row(ui, |ui| {
                            icons::glyph(
                                ui,
                                if is_folder { Icon::Folder } else { Icon::File },
                                15.0,
                                ui.visuals().text_color(),
                            );
                            let name = p
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_else(|| line.to_string());
                            if tip(ui.link(txt(&name, size::BODY, None)), line).clicked() {
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
                            ui.label(txt(
                                &parent,
                                size::SMALL,
                                Some(ui.visuals().weak_text_color()),
                            ));
                        });
                    }
                }
                Kind::Link => {
                    ui.hyperlink_to(
                        RichText::new(crate::net::readable_url(it.text.trim())).size(size::LARGE),
                        it.text.trim(),
                    );
                }
                Kind::Text => {
                    if let Some(color) = color_in(&it.text) {
                        let line = t!("item-color", value = color_other_way(&it.text, color));
                        row(ui, |ui| {
                            let (swatch, _) =
                                ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                            let edge = ui.visuals().weak_text_color().gamma_multiply(0.3);
                            ui.painter().rect_filled(
                                swatch,
                                CornerRadius::same(radius::MEDIUM),
                                color,
                            );
                            ui.painter().rect_stroke(
                                swatch,
                                CornerRadius::same(radius::MEDIUM),
                                Stroke::new(1.0, edge),
                                egui::StrokeKind::Inside,
                            );
                            lbl(ui, &line, size::BODY, ui.visuals().text_color());
                        });
                        ui.add_space(space::SMALL);
                    }
                    let d = self.detail_text(&it);
                    if d.monospace {
                        let code_bg = ui.visuals().panel_fill;
                        Frame::new()
                            .fill(code_bg)
                            .corner_radius(CornerRadius::same(radius::MEDIUM))
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(d.shown.as_str())
                                            .monospace()
                                            .size(size::CONTROL),
                                    )
                                    .selectable(true),
                                );
                            });
                    } else {
                        // The text itself, the pane's main content, lined up with
                        // the actions under it; a box around it read as a field.
                        Frame::new()
                            .inner_margin(Margin::symmetric(0, 4))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                reading_text(ui, &d.shown);
                            });
                    }
                }
            });
        ui.add_space(space::MEDIUM);
        // The actions right under the content, the main one first and Delete
        // at the far end, apart from the rest.
        row(ui, |ui| {
            let keys = self.cfg.keys.clone();
            let ways = self.ways_to_copy(&it);
            if let Some(paste) = main_button(ui, &ways) {
                self.run(Command::Copy(it.id, paste));
            }
            // A pinned item's pin button says what a click does, with the
            // filled pin of the pinned state.
            let pin = if it.pinned {
                let shade = text_shade(accent(), ui.visuals().dark_mode);
                let job = pill_job(&t!("details-unpin"), size::CONTROL, shade, None);
                let r = icons::tinted_button(ui, Icon::PinFilled, job, accent(), BUTTON_HEIGHT);
                tip(r, &hint(&t!("details-unpin"), &keys.pin))
            } else {
                let tip = hint(&t!("details-pin-keep"), &keys.pin);
                let size = Vec2::splat(BUTTON_HEIGHT);
                icon_button_ex(ui, Icon::Pin, &tip, size, 16.0, None, None)
            };
            if pin.clicked() {
                self.run(Command::Pin(it.id));
            }
            if it.kind == Kind::Image
                && icons::icon_button(ui, Icon::Save, &t!("details-save-image")).clicked()
            {
                self.save_image_as(&it);
            }
            if let Some(p) = place_of(&it) {
                if icons::icon_button(ui, Icon::Folder, &t!("details-show-in-folder")).clicked() {
                    self.show_in_folder(p);
                }
            }
            if it.kind == Kind::Link
                && icons::icon_button(ui, Icon::Browser, &t!("details-open-in-browser")).clicked()
            {
                ctx.open_url(egui::OpenUrl::new_tab(it.text.trim()));
            }
            far(ui, |ui| {
                if icons::icon_button(ui, Icon::Delete, &hint(&t!("details-delete"), &keys.delete))
                    .clicked()
                {
                    self.run(Command::Delete(it.id));
                }
            });
        });
        ui.add_space(space::SMALL);
        // One line of facts: when and where it came from first, then how
        // long it is.
        let mut parts = self.meta_parts(&it);
        let mut facts = vec![parts.remove(0)];
        if let Some(app) = &it.app {
            // The program's own name once it is known, else its file name.
            let program = it.program.as_deref().and_then(|p| self.programs.get(p));
            let name = crate::app::programs::shown_name(app, program);
            facts.push(t!("details-copied-from", app = name.as_str()));
        }
        facts.extend(parts);
        if it.origin == Origin::Imported {
            facts.push(t!("details-from-files"));
        }
        if it.formatted {
            facts.push(t!("details-formatted"));
        }
        let weak = ui.visuals().weak_text_color();
        let read_failed = it.ocr.is_none() && self.ocr_failed.contains(&it.id);
        let read_again = row_wrapped(ui, |ui| {
            ui.label(txt(&facts.join(FACTS_GAP), size::SMALL, Some(weak)));
            read_failed
                && ui
                    .link(txt(&t!("details-read-again"), size::SMALL, None))
                    .clicked()
        });
        if read_again {
            self.run(Command::ReadAgain(it.id));
        }
        ui.add_space(space::SMALL);
        self.tag_editor(ui, &it);
    }

    /// The facts about an item, its time first.
    pub(super) fn meta_parts(&mut self, it: &Item) -> Vec<String> {
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
                    None if self.ocr_failed.contains(&it.id) => {
                        parts.push(t!("details-read-failed"))
                    }
                    None if self.cfg.reads_images() => parts.push(t!("details-reading-text")),
                    None => {}
                }
            }
            Kind::Link => parts.push(domain(&it.text).to_string()),
            Kind::Files => parts.push(t!("details-files-count", count = it.text.lines().count())),
        }
        if let Some(src) = &it.source {
            parts.push(src.display().to_string());
        }
        parts
    }

    pub(super) fn image_details(&mut self, ui: &mut Ui, it: &Item) {
        match self.preview(it) {
            Preview::Ready(tex) => {
                let size = tex.size_vec2();
                let max_h = (ui.available_height() - 140.0).max(220.0);
                let scale = (ui.available_width() / size.x).min(max_h / size.y).min(1.0);
                let tint = theme::picture_tint(ui.visuals().dark_mode);
                ui.add(
                    egui::Image::new((tex.id(), size * scale))
                        .tint(tint)
                        .corner_radius(8.0),
                );
            }
            Preview::Loading => {
                ui.add_space(space::HUGE);
                slow_spinner(ui, 22.0, None);
            }
            Preview::Missing => {
                lbl(
                    ui,
                    &t!("details-image-missing"),
                    size::BODY,
                    ui.visuals().weak_text_color(),
                );
            }
        }
        if let Some(t) = it.ocr.as_deref().filter(|t| !t.trim().is_empty()) {
            ui.add_space(space::LARGE);
            row(ui, |ui| {
                lbl(
                    ui,
                    &t!("details-text-in-image"),
                    size::SMALL,
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
                bidi::content(ui, t, FontId::proportional(size::BODY), c);
            } else {
                // On the left, as left-to-right text reads, in Arabic too.
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    ui.add(egui::Label::new(RichText::new(t).size(size::BODY)).selectable(true));
                });
            }
        }
    }

    pub(super) fn empty_details(&self, ui: &mut Ui) {
        ui.add_space(space::EMPTY_PAGE);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            lbl(ui, &t!("details-select-an-item"), size::BODY, weak);
            ui.add_space(space::LARGE);
            let k = &self.cfg.keys;
            let quick = t!("quick-paste-keys");
            let rows = [
                ("Up+Down", t!("details-key-move")),
                (k.paste.as_str(), t!("details-key-paste")),
                (k.paste_plain.as_str(), t!("details-key-paste-plain")),
                (k.copy_only.as_str(), t!("details-key-copy-only")),
                (quick.as_str(), t!("details-key-quick")),
                ("Shift+F10", t!("details-key-menu")),
                ("F1", t!("details-key-all")),
                (k.pin.as_str(), t!("details-key-pin")),
                (k.tag.as_str(), t!("details-key-tag")),
                (k.delete.as_str(), t!("details-key-delete")),
                ("Escape", t!("details-key-clear")),
            ];
            for (keys, what) in rows.into_iter().filter(|(k, _)| !k.trim().is_empty()) {
                row(ui, |ui| {
                    // Keys hug a shared centre line from one side, actions from the other.
                    let half = ui.available_width() / 2.0;
                    // Only the first frame after starting can be this narrow.
                    let width = (half - 8.0).max(0.0);
                    let (first, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
                    let mut caps =
                        ui.new_child(egui::UiBuilder::new().max_rect(first).layout(far_layout()));
                    // A right-to-left child lays the caps out backwards; undo that.
                    let keys = if i18n::rtl() {
                        keys.to_string()
                    } else {
                        keys.split('+').rev().collect::<Vec<_>>().join("+")
                    };
                    shortcut::keycaps(&mut caps, &keys, false);
                    ui.add_space(space::SMALL);
                    lbl(ui, &what, size::SMALL, weak);
                });
            }
        });
    }
}
