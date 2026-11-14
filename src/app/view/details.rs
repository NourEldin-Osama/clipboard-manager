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

/// How much of the window's width the details pane takes when it first
/// shows: most clips are short, so the list beside it gets the most room.
const DETAILS_SHARE: f32 = 0.42;

/// The details pane's first width stays in this range: wide enough for its
/// actions row, narrow enough to leave the list most of a wide window.
const DETAILS_FIRST_WIDTH: std::ops::RangeInclusive<f32> = 400.0..=560.0;

/// What the list beside the details keeps at least, also in a window too
/// narrow for both.
const LIST_LEAST_WIDTH: f32 = 320.0;

/// The details pane on the reading-end side of `ui`, beside the list or
/// the grid, with `add` in it. One panel in every view, so it keeps one
/// width; the user can drag its edge.
pub(super) fn details_panel<R>(
    ui: &mut Ui,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let panel = if i18n::rtl() {
        egui::Panel::left("details-pane")
    } else {
        egui::Panel::right("details-pane")
    };
    let (least, most) = (*DETAILS_FIRST_WIDTH.start(), *DETAILS_FIRST_WIDTH.end());
    let window_width = ui.available_width();
    let room = (window_width - LIST_LEAST_WIDTH).max(0.0);
    let first = (window_width * DETAILS_SHARE).clamp(least, most).min(room);
    let fill = ui.visuals().extreme_bg_color;
    panel
        .resizable(true)
        .show_separator_line(false)
        .size_range(least.min(room)..=room)
        .default_size(first)
        .frame(
            Frame::new()
                .inner_margin(Margin {
                    left: 20,
                    right: 20,
                    top: 14,
                    bottom: 14,
                })
                .fill(fill),
        )
        .show(ui, |ui| {
            // egui sizes a side panel by what it holds: the pane keeps its
            // width whatever it shows.
            ui.set_min_width(ui.available_width());
            add(ui)
        })
}

/// The least room the content keeps, in a very short window.
const LEAST_CONTENT_HEIGHT: f32 = 48.0;

/// The room for the content's scroll bar in the pane's margin, beside the
/// content: as wide as the bar under the pointer, and a little more.
const SCROLL_BAR_ROOM: f32 = 12.0;

/// The pane's actions row at its top, in one place from item to item;
/// under it the content in a scroll area as tall as the content, up to the
/// room the rest leaves; and right under that what `under` adds (the facts
/// and the tags), so they stay in sight however long the content is. Only
/// the content scrolls. The height of `under` is the last frame's; when it
/// changes, egui lays the frame out again.
fn actions_over_content<T>(
    ui: &mut Ui,
    state: &mut T,
    actions: impl FnOnce(&mut Ui, &mut T),
    content: impl FnOnce(&mut Ui, &mut T),
    under: impl FnOnce(&mut Ui, &mut T),
) {
    actions(ui, state);
    // A gap of 12 under the row, with the one egui leaves after it.
    ui.add_space(space::MEDIUM - ui.spacing().item_spacing.y);
    // What goes under the content, with the gaps around it.
    let height_id = ui.scope_id().with("details-under-height");
    let under_height = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(height_id))
        .unwrap_or_default();
    let room = (ui.available_height() - under_height).max(LEAST_CONTENT_HEIGHT);
    // The scroll bar floats in the pane's margin, beside the content, so it
    // never covers the text: Arabic lines start at the content's edge.
    let width = ui.available_width();
    let at = ui.cursor().min;
    let area = Rect::from_min_size(at, Vec2::new(width + SCROLL_BAR_ROOM, room));
    let mut scroll = ui.new_child(egui::UiBuilder::new().max_rect(area).layout(*ui.layout()));
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .content_margin(Margin {
            left: 0,
            right: SCROLL_BAR_ROOM as i8,
            top: 0,
            bottom: 0,
        })
        .show(&mut scroll, |ui| content(ui, state));
    let shown = Rect::from_min_size(at, Vec2::new(width, scroll.min_rect().height().min(room)));
    ui.advance_cursor_after_rect(shown);
    ui.add_space(space::SMALL);
    ui.scope(|ui| under(ui, state));
    let used = ui.cursor().top() - shown.bottom();
    if (used - under_height).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(height_id, used));
        ui.ctx()
            .request_discard("the details' facts or tags changed height");
    }
}

/// The row of the pane's actions: Back first when `back`, while the
/// details cover the list, then what `add` adds. It is as tall as the
/// buttons from the start, so Back sits on their line. Back's response,
/// when it shows.
fn actions_row(ui: &mut Ui, back: bool, add: impl FnOnce(&mut Ui)) -> Option<egui::Response> {
    row_of(ui, BUTTON_HEIGHT, |ui| {
        let back = back.then(|| {
            let size = Vec2::splat(30.0);
            icon_button_ex(ui, Icon::Back, &t!("details-close"), size, 20.0, None, None)
        });
        add(ui);
        back
    })
}

/// Between two facts of the details, and on a list row's second line.
pub(super) const FACTS_DOT: &str = "·";

/// Facts one after another in reading order, a dot between each two. Each
/// fact is a label of its own, in its own direction, so in Arabic a name
/// and the number of the next fact don't run together. In a wrapped row a
/// fact that doesn't fit starts the next line; a dot never starts one.
fn facts_line(ui: &mut Ui, facts: &[String], color: Color32) {
    let font = FontId::proportional(size::SMALL);
    let dot_width = ui
        .fonts_mut(|f| f.layout_no_wrap(FACTS_DOT.into(), font, color))
        .size()
        .x;
    for (n, fact) in facts.iter().enumerate() {
        if n > 0 && ui.available_size_before_wrap().x >= dot_width {
            lbl(ui, FACTS_DOT, size::SMALL, color);
        }
        lbl(ui, fact, size::SMALL, color);
    }
}

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

/// How tall the actions at the pane's top are.
const BUTTON_HEIGHT: f32 = 34.0;

/// The main button's word is cut no shorter than this.
const LEAST_LABEL: f32 = 60.0;

/// The details' main button: the main way to put the item back, with its
/// key, and an arrow to the other ways when there are any. It leaves
/// `reserved` of the row's width to the buttons after it: a long program
/// name is cut short when the row has no room for it.
fn main_button(ui: &mut Ui, ways: &[CopyWay], reserved: f32) -> Option<Paste> {
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
    let gap = ui.spacing().item_spacing.x;
    let room = ui.available_width() - reserved - gap - 8.0 - 28.0 - arrow_w;
    // In a row short of room the key goes first; its tip still names it.
    let keys_w = if main.keys.trim().is_empty() || room - keys.size().x - 10.0 < LEAST_LABEL {
        0.0
    } else {
        keys.size().x + 10.0
    };
    let font = FontId::proportional(size::BODY);
    let room = (room - keys_w).max(LEAST_LABEL);
    let label = one_line(ui, &main.text, &[], font, Color32::WHITE, room);
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

    /// Opens an image item in Windows' Photos app: a PNG copy is written in
    /// the background, then Photos opens it.
    #[cfg(windows)]
    pub(super) fn open_in_photos(&mut self, it: &Item) {
        let Some(stored) = it.image.clone() else {
            return;
        };
        let (images, id) = (self.library.store().images.clone(), it.id);
        self.background.run(move || {
            let copy = crate::app::photos::write_copy(&images, id, &stored);
            Done::CopiedForPhotos(copy.map_err(|e| format!("{e:#}")))
        });
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
                let shade = tag_shade(c, ui.visuals().dark_mode);
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
                                    Some(tag_shade(tag_color(k), ui.visuals().dark_mode)),
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
                // A thin outline, so the pill reads as a button under the
                // facts.
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
        let Some(it) = self.selected.and_then(|id| self.library.get(id)).cloned() else {
            self.empty_details(ui);
            return;
        };
        // At the top what you can do with it, after Back while the details
        // cover the list; under that the copied content, and right under
        // it one line of facts and the tags.
        actions_over_content(
            ui,
            self,
            |ui, app| app.details_actions(ui, &it, closeable),
            |ui, app| app.details_content(ui, &it),
            |ui, app| app.facts_and_tags(ui, &it),
        );
    }

    /// The copied content, as its kind shows it.
    fn details_content(&mut self, ui: &mut Ui, it: &Item) {
        match it.kind {
            Kind::Image => self.image_details(ui, it),
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
                    let value = bidi::isolate(&color_other_way(&it.text, color));
                    let line = t!("item-color", value = value);
                    row(ui, |ui| {
                        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                        let edge = ui.visuals().weak_text_color().gamma_multiply(0.3);
                        ui.painter()
                            .rect_filled(swatch, CornerRadius::same(radius::MEDIUM), color);
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
                let d = self.detail_text(it);
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
                    // the facts under it; a box around it read as a field.
                    Frame::new()
                        .inner_margin(Margin::symmetric(0, 4))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            reading_text(ui, &d.shown);
                        });
                }
            }
        }
    }

    /// The actions at the pane's top, in one group: Back first when
    /// `closeable`, then the main one, and Delete last.
    fn details_actions(&mut self, ui: &mut Ui, it: &Item, closeable: bool) {
        let back = actions_row(ui, closeable, |ui| {
            let keys = self.cfg.keys.clone();
            let ways = self.ways_to_copy(it);
            // The main button leaves the buttons after it the room they
            // took in the frame before.
            let rest_id = ui.scope_id().with("details-actions");
            let reserved = ui
                .ctx()
                .data(|d| d.get_temp::<f32>(rest_id))
                .unwrap_or_default();
            if let Some(paste) = main_button(ui, &ways, reserved) {
                self.run(Command::Copy(it.id, paste));
            }
            let rest = ui.scope(|ui| {
                // A pinned item's pin button says what a click does, with
                // the filled pin of the pinned state.
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
                if it.kind == Kind::Image {
                    if icons::icon_button(ui, Icon::Save, &t!("details-save-image")).clicked() {
                        self.save_image_as(it);
                    }
                    let full_screen = hint(&t!("details-view-full-screen"), "F11");
                    if icons::icon_button(ui, Icon::FullScreen, &full_screen).clicked() {
                        self.view_full_screen(it.id);
                    }
                    self.photos_button(ui, it);
                }
                if let Some(p) = place_of(it) {
                    if icons::icon_button(ui, Icon::Folder, &t!("details-show-in-folder")).clicked()
                    {
                        self.show_in_folder(p);
                    }
                }
                if it.kind == Kind::Link
                    && icons::icon_button(ui, Icon::Browser, &t!("details-open-in-browser"))
                        .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(it.text.trim()));
                }
                let delete = hint(&t!("details-delete"), &keys.delete);
                if icons::icon_button(ui, Icon::Delete, &delete).clicked() {
                    self.run(Command::Delete(it.id));
                }
            });
            let rest = rest.response.rect.width();
            if (rest - reserved).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(rest_id, rest));
                ui.ctx()
                    .request_discard("the buttons after Copy changed width");
            }
        });
        if back.is_some_and(|back| back.clicked()) {
            self.details_open = false;
        }
    }

    /// Under the content: one line of facts, then the tags.
    fn facts_and_tags(&mut self, ui: &mut Ui, it: &Item) {
        // When and where it came from first, then how long it is.
        let mut parts = self.meta_parts(it);
        let mut facts = vec![parts.remove(0)];
        if let Some(app) = &it.app {
            // The program's own name once it is known, else its file name.
            let program = it.program.as_deref().and_then(|p| self.programs.get(p));
            let name = crate::app::programs::shown_name(app, program);
            facts.push(t!("details-copied-from", app = bidi::isolate(&name)));
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
            facts_line(ui, &facts, weak);
            read_failed
                && ui
                    .link(txt(&t!("details-read-again"), size::SMALL, None))
                    .clicked()
        });
        if read_again {
            self.run(Command::ReadAgain(it.id));
        }
        ui.add_space(space::SMALL);
        self.tag_editor(ui, it);
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
                // Large, so in full, in the dark theme too.
                let tint = theme::picture_tint(ui.visuals().dark_mode, 1.0);
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

    /// Open in Photos among the pane's actions, with the Photos app's own
    /// icon when Windows has it.
    #[cfg(windows)]
    fn photos_button(&mut self, ui: &mut Ui, it: &Item) {
        let tip = t!("details-open-in-photos");
        let button = match crate::app::photos::logo(ui.ctx()) {
            Some(logo) => icons::picture_button(ui, logo.id(), &tip),
            None => icons::icon_button(ui, Icon::Image, &tip),
        };
        if button.clicked() {
            self.open_in_photos(it);
        }
    }

    /// Only Windows has the Photos app.
    #[cfg(not(windows))]
    fn photos_button(&mut self, _ui: &mut Ui, _it: &Item) {}

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
                key_hint_row(ui, keys, &what, weak);
            }
        });
    }
}

/// One key of the empty pane: its caps and what it does hug a shared
/// centre line, the caps from the reading start's side.
pub(super) fn key_hint_row(ui: &mut Ui, keys: &str, what: &str, color: Color32) {
    row(ui, |ui| {
        let half = ui.available_width() / 2.0;
        // Only the first frame after starting can be this narrow.
        let width = (half - 8.0).max(0.0);
        let (first, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
        let mut caps = ui.new_child(egui::UiBuilder::new().max_rect(first).layout(far_layout()));
        shortcut::keycaps(&mut caps, keys, false);
        ui.add_space(space::SMALL);
        lbl(ui, what, size::SMALL, color);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window `size` big, for one frame at a time.
    fn input(size: Vec2) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        }
    }

    /// The details pane as it first shows in a window `width` wide, and
    /// the room left beside it for the list.
    fn first_pane(width: f32) -> (Rect, Rect) {
        let ctx = egui::Context::default();
        let (mut pane, mut rest) = (Rect::NOTHING, Rect::NOTHING);
        let mut output = ctx.run_ui(input(Vec2::new(width, 700.0)), |ui| {
            pane = details_panel(ui, |ui| ui.label("Details")).response.rect;
            rest = ui.available_rect_before_wrap();
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        (pane, rest)
    }

    #[test]
    fn the_details_pane_first_takes_400_to_560_points() {
        for width in (720..=2000).step_by(10).map(|w| w as f32) {
            let (pane, rest) = first_pane(width);
            assert!(
                DETAILS_FIRST_WIDTH.contains(&pane.width()),
                "{width}: {pane:?}"
            );
            // At the window's reading end, the list beside it.
            assert_eq!(pane.right(), width, "{width}");
            assert_eq!(rest.right(), pane.left(), "{width}");
        }
        // A window too narrow for both still leaves the list its room.
        let (pane, rest) = first_pane(560.0);
        assert_eq!(rest.width(), LIST_LEAST_WIDTH, "{pane:?}");
    }

    /// Where the pane's parts are: the pane, the room inside its margin,
    /// Back when it shows, the main button (a stand-in as tall), where the
    /// content starts and where its last line ends (below the pane when it
    /// doesn't fit), the facts and the tags.
    struct Parts {
        pane: Rect,
        inside: Rect,
        back: Option<Rect>,
        main: Rect,
        content_top: f32,
        content_end: f32,
        facts: Rect,
        tags: Rect,
    }

    /// The pane in a window of the app's first size, with `lines` lines of
    /// content and Back when `back`. With `to_end`, the content is scrolled
    /// to its end.
    fn parts_with(lines: usize, back: bool, to_end: bool) -> Parts {
        let ctx = egui::Context::default();
        let mut parts = Parts {
            pane: Rect::NOTHING,
            inside: Rect::NOTHING,
            back: None,
            main: Rect::NOTHING,
            content_top: 0.0,
            content_end: 0.0,
            facts: Rect::NOTHING,
            tags: Rect::NOTHING,
        };
        let window = Vec2::from(crate::app::WINDOW_SIZE);
        // The height under the content is the last frame's, and a scroll
        // moves the content in the frame after it: the first frames find
        // them.
        for _ in 0..4 {
            let mut output = ctx.run_ui(input(window), |ui| {
                let pane = details_panel(ui, |ui| {
                    parts.inside = ui.max_rect();
                    actions_over_content(
                        ui,
                        &mut parts,
                        |ui, parts| {
                            let size = Vec2::new(120.0, BUTTON_HEIGHT);
                            let shown = actions_row(ui, back, |ui| {
                                parts.main = ui.add_sized(size, egui::Button::new("Copy")).rect;
                            });
                            parts.back = shown.map(|back| back.rect);
                        },
                        |ui, parts| {
                            parts.content_top = ui.cursor().top();
                            for line in 0..lines {
                                ui.label(format!("Line {line}"));
                            }
                            parts.content_end = ui.min_rect().bottom();
                            if to_end {
                                let at_once = egui::style::ScrollAnimation::none();
                                ui.scroll_to_cursor_animation(Some(Align::BOTTOM), at_once);
                            }
                        },
                        |ui, parts| {
                            parts.facts = ui.label("Today, 10:30").rect;
                            ui.add_space(space::SMALL);
                            parts.tags = ui.button("Add a tag").rect;
                        },
                    );
                });
                parts.pane = pane.response.rect;
            });
            output.textures_delta.clear();
        }
        parts
    }

    /// Long content keeps the pane's width, and its scroll bar floats
    /// beside it, in the pane's margin.
    #[test]
    fn the_scroll_bar_floats_beside_the_content() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (mut content, mut pane) = (Rect::NOTHING, Rect::NOTHING);
        let mut bar = None;
        for _ in 0..3 {
            let mut output = ctx.run_ui(input(Vec2::new(480.0, 600.0)), |ui| {
                // Inside the pane's margin, as its frame sets it.
                pane = ui.max_rect().shrink(20.0);
                ui.scope_builder(egui::UiBuilder::new().max_rect(pane), |ui| {
                    actions_over_content(
                        ui,
                        &mut (),
                        |ui, _| {
                            let _ = ui.button("Copy");
                        },
                        |ui, _| {
                            content = ui.max_rect();
                            for line in 0..300 {
                                ui.label(format!("Line {line}"));
                            }
                        },
                        |ui, _| {
                            ui.label("Today, 10:30");
                        },
                    );
                });
            });
            output.textures_delta.clear();
            let update = output.platform_output.accesskit_update.expect("a tree");
            bar = update
                .nodes
                .iter()
                .find(|(_, node)| node.role() == egui::accesskit::Role::ScrollBar)
                .and_then(|(_, node)| node.bounds());
        }
        let bar = bar.expect("a scroll bar");
        assert_eq!(content.x_range(), pane.x_range());
        assert!(
            bar.x0 as f32 >= content.right(),
            "{bar:?} beside {content:?}"
        );
        assert!(
            bar.x1 as f32 <= pane.right() + 20.0,
            "{bar:?} in the margin"
        );
    }

    /// The facts line in a wrapped row in a window `width` wide, as drawn:
    /// each line of each of its labels, with its text and its place, in
    /// reading order: line by line, each from its reading start.
    fn facts_drawn(ctx: &egui::Context, facts: &[String], width: f32) -> Vec<(String, Rect)> {
        let mut output = ctx.run_ui(input(Vec2::new(width, 300.0)), |ui| {
            row_wrapped(ui, |ui| facts_line(ui, facts, Color32::GRAY));
        });
        output.textures_delta.clear();
        let mut drawn = Vec::new();
        for clipped in &output.shapes {
            let egui::Shape::Text(text) = &clipped.shape else {
                continue;
            };
            // A label that goes on in the next line can leave its first
            // line empty.
            for row in &text.galley.rows {
                if !row.row.glyphs.is_empty() {
                    let rect = row.rect_without_leading_space();
                    drawn.push((row.row.text(), rect.translate(text.pos.to_vec2())));
                }
            }
        }
        let rtl = i18n::rtl();
        drawn.sort_by(|(_, a), (_, b)| {
            if (a.center().y - b.center().y).abs() > 1.0 {
                a.center().y.total_cmp(&b.center().y)
            } else if rtl {
                b.center().x.total_cmp(&a.center().x)
            } else {
                a.center().x.total_cmp(&b.center().x)
            }
        });
        drawn
    }

    /// The facts in the window's language: the time, an Arabic fact with a
    /// Latin name, and two with numbers. In reading order, from the left
    /// in English and from the right in Arabic: on one line, each fact
    /// whole with a dot between each two, so the name and the number never
    /// stand side by side; wrapped onto more lines, still in order, and no
    /// line starts with a dot.
    fn check_facts_in_reading_order() {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let facts = [
            t!("time-today-at", time = "5:31"),
            t!("details-copied-from", app = "Microsoft Teams"),
            t!("details-characters", count = 44),
            t!("details-words", count = 8),
        ];
        let wide = facts_drawn(&ctx, &facts, 800.0);
        let texts: Vec<&str> = wide.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(texts.len(), 7, "{texts:?}");
        for (at, mark) in ["5:31", "Microsoft Teams", "44", "8"].iter().enumerate() {
            assert!(texts[2 * at].contains(mark), "{mark} in {texts:?}");
        }
        for at in [1, 3, 5] {
            assert_eq!(texts[at], FACTS_DOT, "{texts:?}");
        }
        let top = wide[0].1.center().y;
        assert!(wide.iter().all(|(_, r)| (r.center().y - top).abs() <= 1.0));

        let mut wrapped_at = 0;
        for width in (120..=400).step_by(4).map(|w| w as f32) {
            let drawn = facts_drawn(&ctx, &facts, width);
            let line_starts: Vec<&str> = drawn
                .iter()
                .enumerate()
                .filter(|(at, (_, r))| {
                    *at == 0 || (r.center().y - drawn[at - 1].1.center().y).abs() > 1.0
                })
                .map(|(_, (text, _))| text.as_str())
                .collect();
            if line_starts.len() > 1 {
                wrapped_at += 1;
            }
            assert!(!line_starts.contains(&FACTS_DOT), "{width}: {drawn:?}");
            let read: String = drawn.iter().map(|(text, _)| text.as_str()).collect();
            let mut from = 0;
            for mark in ["5:31", "Microsoft", "44", "8"] {
                let at = read[from..].find(mark);
                let at = at.unwrap_or_else(|| panic!("{width}: {mark} in {read}"));
                from += at + mark.len();
            }
        }
        assert!(wrapped_at > 0, "the facts wrap in a narrow pane");
    }

    #[test]
    fn the_facts_read_in_order() {
        check_facts_in_reading_order();
    }

    #[test]
    fn the_facts_read_in_order_right_to_left() {
        crate::i18n::passes_in_arabic(module_path!(), "the_facts_read_in_order_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by the_facts_read_in_order_right_to_left"]
    fn the_facts_read_in_order_in_arabic() {
        if crate::i18n::arabic_run() {
            check_facts_in_reading_order();
        }
    }

    /// The actions row starts the pane, in its first 20 points: Back at
    /// the reading start when it shows, then the main button on its line.
    /// The content starts 12 points under the row, and the facts and tags
    /// come right under it. Long content scrolls under the row: the row,
    /// the facts and the tags keep their places.
    fn check_the_actions_lead_and_stay() {
        let rtl = i18n::rtl();
        // The gap between two parts of a pane, as between the facts and
        // the tags.
        let between = space::SMALL + egui::Style::default().spacing.item_spacing.y;
        for back in [false, true] {
            let short = parts_with(1, back, false);
            let long = parts_with(300, back, false);
            let scrolled = parts_with(300, back, true);
            let (pane, inside, main) = (short.pane, short.inside, short.main);
            let first = short.back.unwrap_or(main);
            assert_eq!(short.back.is_some(), back);
            for part in short.back.iter().chain([&main]) {
                let top = part.top() - pane.top();
                assert!((0.0..=20.0).contains(&top), "{back}: {part:?} in {pane:?}");
            }
            // The row from the reading start, Back first.
            let start = if rtl {
                inside.right() - first.right()
            } else {
                first.left() - inside.left()
            };
            assert!(start.abs() <= 0.5, "{back}: {first:?} in {inside:?}");
            if let Some(back) = short.back {
                let after = if rtl {
                    main.right() < back.left()
                } else {
                    main.left() > back.right()
                };
                assert!(after, "{main:?} after {back:?}");
                assert!((main.center().y - back.center().y).abs() <= 0.5);
            }
            let gap = short.content_top - main.bottom();
            assert!(
                (gap - space::MEDIUM).abs() <= 0.5,
                "{back}: the content starts 12 under the row: {gap}"
            );
            let gap = short.facts.top() - short.content_end;
            assert!(
                (gap - between).abs() <= 0.5,
                "{back}: the facts start right under the content: {gap}"
            );
            assert!(short.tags.top() > short.facts.bottom());
            // Long content runs past the pane's end and scrolls; its facts
            // and tags stay in the pane, under it.
            assert!(
                long.content_end > pane.bottom(),
                "{back}: the content is long"
            );
            assert!(long.facts.top() > main.bottom() && long.tags.bottom() <= pane.bottom());
            assert!(
                scrolled.content_top < long.content_top - 1000.0
                    && scrolled.content_end <= scrolled.facts.top(),
                "{back}: the content scrolls to its end: {} to {}",
                scrolled.content_top,
                scrolled.content_end
            );
            for parts in [&long, &scrolled] {
                assert_eq!(parts.main, main, "{back}: the main button stays");
                assert_eq!(parts.back, short.back, "{back}: Back stays");
            }
            assert_eq!((scrolled.facts, scrolled.tags), (long.facts, long.tags));
        }
    }

    #[test]
    fn the_actions_lead_the_pane_and_stay() {
        check_the_actions_lead_and_stay();
    }

    #[test]
    fn the_actions_lead_the_pane_and_stay_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_actions_lead_the_pane_and_stay_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_actions_lead_the_pane_and_stay_right_to_left"]
    fn the_actions_lead_the_pane_and_stay_in_arabic() {
        if crate::i18n::arabic_run() {
            check_the_actions_lead_and_stay();
        }
    }
}
