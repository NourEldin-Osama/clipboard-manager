//! The list view and the rows it paints.

use super::*;

#[derive(Debug, PartialEq)]
pub(super) enum Line {
    Header(Group, usize),
    Row(usize),
}

/// One row per group, with a header above each run of equal groups that
/// counts its rows. Runs once per frame, so it stays linear.
pub(super) fn with_headers(labels: impl Iterator<Item = Group>) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut header_at = None;
    let mut last = None;
    for (r, label) in labels.enumerate() {
        if last != Some(label) {
            header_at = Some(lines.len());
            lines.push(Line::Header(label, 0));
            last = Some(label);
        }
        if let Some(Line::Header(_, n)) = header_at.map(|i| &mut lines[i]) {
            *n += 1;
        }
        lines.push(Line::Row(r));
    }
    lines
}

/// The item's thumbnail texture, queuing a decode the first time it's seen.
/// `seen` notes the frame it was drawn in (see `thumbs_to_drop`).
pub(super) fn thumb_for(
    it: &Item,
    thumbs: &HashMap<u64, Option<TextureHandle>>,
    loader: &mut ImageLoader,
    seen: (&mut HashMap<u64, u64>, u64),
) -> Option<TextureHandle> {
    if it.kind == Kind::Image {
        let (seen, frame) = seen;
        seen.insert(it.id, frame);
    }
    match (it.kind, thumbs.get(&it.id), &it.image) {
        (Kind::Image, Some(t), _) => t.clone(),
        (Kind::Image, None, Some(p)) => {
            loader.request(it.id, p, false);
            None
        }
        _ => None,
    }
}

/// The thumbnails to drop so that `keep` stay: those drawn longest ago
/// first. The ones on screen were drawn last, so they stay.
pub(super) fn thumbs_to_drop(
    seen: &HashMap<u64, u64>,
    ids: impl Iterator<Item = u64>,
    keep: usize,
) -> Vec<u64> {
    let mut by_age: Vec<(u64, u64)> = ids
        .map(|id| (seen.get(&id).copied().unwrap_or(0), id))
        .collect();
    let extra = by_age.len().saturating_sub(keep);
    by_age.sort_unstable();
    by_age.into_iter().take(extra).map(|(_, id)| id).collect()
}

/// `hot` while the pointer is over the row: its hover buttons then take the
/// right edge, so the time and badges are left out.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_row(
    ui: &Ui,
    rect: Rect,
    hot: bool,
    it: &Item,
    hit: &Hit,
    thumb: Option<TextureHandle>,
    selected: bool,
    compact: bool,
    terms: &[String],
    max_sem: f32,
) {
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let a = accent();
    // Laid out left to right, then mirrored for Arabic.
    let m = |r: Rect| flip(rect, r);
    let rtl = i18n::rtl();
    if selected {
        p.rect_filled(
            rect,
            CornerRadius::same(8),
            a.gamma_multiply(if v.dark_mode { 0.22 } else { 0.12 }),
        );
        p.rect_filled(
            m(Rect::from_min_size(
                rect.min + Vec2::new(0.0, 8.0),
                Vec2::new(3.0, rect.height() - 16.0),
            )),
            CornerRadius::same(2),
            a,
        );
    } else if hot {
        p.rect_filled(
            rect,
            CornerRadius::same(8),
            v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6),
        );
    }

    let av = if compact { 26.0 } else { 38.0 };
    let av_rect = m(Rect::from_min_size(
        Pos2::new(rect.left() + 12.0, rect.center().y - av / 2.0),
        Vec2::splat(av),
    ));
    match thumb {
        Some(t) => {
            let s = t.size_vec2();
            let k = (av / s.x).max(av / s.y);
            // Fill the square, cropping the longer side.
            let uv_w = (av / (s.x * k)).min(1.0);
            let uv_h = (av / (s.y * k)).min(1.0);
            let uv = Rect::from_min_max(
                Pos2::new((1.0 - uv_w) / 2.0, (1.0 - uv_h) / 2.0),
                Pos2::new((1.0 + uv_w) / 2.0, (1.0 + uv_h) / 2.0),
            );
            egui::Image::new((t.id(), Vec2::splat(av)))
                .uv(uv)
                .corner_radius(6.0)
                .paint_at(ui, av_rect);
        }
        None => {
            let c = kind_color(it.kind);
            p.rect_filled(
                av_rect,
                CornerRadius::same(8),
                c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }),
            );
            let code = it.kind == Kind::Text
                && looks_like_code(&it.text.chars().take(600).collect::<String>());
            if code {
                p.text(
                    av_rect.center(),
                    Align2::CENTER_CENTER,
                    "</>",
                    FontId::monospace(av * 0.3),
                    c,
                );
            } else {
                icons::paint(
                    &p,
                    Rect::from_center_size(av_rect.center(), Vec2::splat(av * 0.46)),
                    icons::kind(it.kind),
                    c,
                );
            }
        }
    }

    let x = rect.left() + 12.0 + av + 12.0;
    let right_w = if hot { 134.0 } else { 64.0 };
    let text_w = (rect.right() - x - right_w - 8.0).max(40.0);
    // The text column, mirrored with the rest; lines start at its reading edge.
    let column = m(Rect::from_min_size(
        Pos2::new(x, rect.top()),
        Vec2::new(text_w, rect.height()),
    ));
    let title_color = v.strong_text_color();
    let weak = v.weak_text_color();
    let sub = if compact { String::new() } else { subtitle(it) };
    let title = row_title(it);
    let title = if title.is_empty() {
        t!("item-empty")
    } else {
        title
    };

    let tg = one_line(
        ui,
        &title,
        terms,
        FontId::proportional(14.0),
        title_color,
        text_w,
    );
    // One line sits on the row's midline, beside the icon; two lines share it.
    let title_y = if sub.is_empty() {
        rect.center().y - tg.size().y / 2.0
    } else {
        rect.top() + 10.0
    };
    let line_x = |w: f32| {
        if rtl {
            column.right() - w
        } else {
            column.left()
        }
    };
    p.galley(Pos2::new(line_x(tg.size().x), title_y), tg, title_color);
    if !sub.is_empty() {
        let sg = one_line(ui, &sub, terms, FontId::proportional(12.5), weak, text_w);
        p.galley(Pos2::new(line_x(sg.size().x), rect.top() + 32.0), sg, weak);
    }

    if hot {
        return;
    }
    // Far edge: time, then pinned star and relevance.
    let one_line_row = compact || sub.is_empty();
    let time_y = if one_line_row {
        rect.center().y
    } else {
        rect.top() + 18.0
    };
    let time_at = m(Rect::from_min_size(
        Pos2::new(rect.right() - 12.0, time_y),
        Vec2::ZERO,
    ))
    .min;
    let time_anchor = if rtl {
        Align2::LEFT_CENTER
    } else {
        Align2::RIGHT_CENTER
    };
    paint_text(
        ui,
        &p,
        time_at,
        time_anchor,
        &row_time(it.created),
        FontId::proportional(11.5),
        weak,
    );
    let mut ix = rect.right() - 12.0;
    // On one line the badges sit before the time instead of under it.
    let iy = if one_line_row {
        rect.center().y
    } else {
        rect.top() + 40.0
    };
    if one_line_row {
        ix -= 56.0;
    }
    if it.pinned {
        icons::paint(
            &p,
            m(Rect::from_center_size(
                Pos2::new(ix - 6.5, iy),
                Vec2::splat(13.0),
            )),
            Icon::PinFilled,
            a,
        );
        ix -= 18.0;
    }
    if let Some(s) = hit.relevance.filter(|_| hit.semantic.is_some()) {
        // Three bars: how close this match is to the best one.
        let ratio = if max_sem > 0.0 { s / max_sem } else { 0.0 };
        let level = if ratio > 0.93 {
            3
        } else if ratio > 0.8 {
            2
        } else {
            1
        };
        for b in 0..3 {
            let h = 4.0 + b as f32 * 3.0;
            let r = m(Rect::from_min_size(
                Pos2::new(ix - 18.0 + b as f32 * 6.0, iy + 6.0 - h),
                Vec2::new(4.0, h),
            ));
            p.rect_filled(
                r,
                CornerRadius::same(1),
                if b < level {
                    a
                } else {
                    weak.gamma_multiply(0.35)
                },
            );
        }
    }
}

/// One line of item text cut to `width`, query terms highlighted; Arabic in
/// reading order.
pub(super) fn one_line(
    ui: &Ui,
    text: &str,
    terms: &[String],
    font: FontId,
    color: Color32,
    width: f32,
) -> Arc<egui::Galley> {
    if bidi::has_rtl(text) {
        let first = text.lines().next().unwrap_or("");
        // In the Arabic UI a line with Arabic in it reads right to left, even when
        // it starts with a Latin tag or word.
        let base = if i18n::rtl() { Some(true) } else { None };
        bidi::lines(
            ui,
            first,
            terms,
            font,
            color,
            accent().gamma_multiply(0.28),
            width,
            1,
            base,
        )
        .into_iter()
        .next()
        .map(|(g, _)| g)
        .unwrap_or_else(|| {
            ui.fonts_mut(|f| f.layout_no_wrap(String::new(), FontId::proportional(12.0), color))
        })
    } else {
        ui.fonts_mut(|f| f.layout_job(highlighted(text, terms, font, color, width, 1)))
    }
}

impl App {
    /// The list of items; `in_popup` when it is the quick popup's.
    pub(super) fn list(&mut self, ui: &mut Ui, in_popup: bool) {
        if self.results.is_empty() {
            // Until the first results come, show nothing rather than "no matches".
            if !self.searching {
                self.empty_list(ui);
            }
            return;
        }
        let compact = self.cfg.density == Density::Compact;
        let row_h = if compact { 40.0 } else { 58.0 };
        let header_h = 30.0;
        let searching = !self.query.trim().is_empty();
        // The details pane is always beside the list in Detail view, so rows
        // there don't need an "open" button; the popup has no details pane.
        let can_open = self.cfg.view != ViewMode::Detail && !in_popup;

        // Rows, with date/pinned headers when browsing.
        let mut lines: Vec<Line> = Vec::with_capacity(self.results.len() + 8);
        // Day headers only make sense in date order.
        if !searching && self.cfg.group_by_date && self.sort() == SortOrder::Recent {
            let pinned_first = self.cfg.pinned_first;
            lines = with_headers(self.results.iter().map(|h| {
                self.library
                    .get(h.id)
                    .map_or(Group::Older, |it| Group::of(it, pinned_first))
            }));
        } else {
            lines.extend((0..self.results.len()).map(Line::Row));
        }
        let mut ys = Vec::with_capacity(lines.len());
        let mut y = 0.0;
        for l in &lines {
            ys.push(y);
            y += match l {
                Line::Header(..) => header_h,
                Line::Row(_) => row_h + 2.0,
            };
        }
        let total = y;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if in_popup {
            // Its own scroll place, apart from the full window's list.
            sa = sa.id_salt("popup-list");
        }
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
            if let Some(i) = lines.iter().position(
                |l| matches!(l, Line::Row(r) if Some(self.results[*r].id) == self.selected),
            ) {
                let (off, view_h) = if in_popup {
                    self.popup_list_view
                } else {
                    self.list_view
                };
                let top = ys[i]
                    - if i > 0 && matches!(lines[i - 1], Line::Header(..)) {
                        header_h
                    } else {
                        0.0
                    };
                if top < off {
                    sa = sa.vertical_scroll_offset(top);
                } else if ys[i] + row_h > off + view_h {
                    sa = sa.vertical_scroll_offset(ys[i] + row_h - view_h);
                }
            }
        }

        let max_sem = self
            .results
            .iter()
            .filter_map(|h| h.relevance)
            .fold(f32::MIN, f32::max);
        let terms: Vec<String> = if searching {
            self.query
                .to_lowercase()
                .split_whitespace()
                .map(String::from)
                .collect()
        } else {
            vec![]
        };
        let mut actions = Vec::new();
        let frame = ui.ctx().cumulative_pass_nr();
        let (library, results, thumbs, loader, selected, seen) = (
            &self.library,
            &self.results,
            &self.thumbs,
            &mut self.loader,
            self.selected,
            &mut self.thumb_seen,
        );
        let out = sa.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            for (i, l) in lines.iter().enumerate() {
                let h = if matches!(l, Line::Header(..)) {
                    header_h
                } else {
                    row_h
                };
                if ys[i] + h < viewport.min.y || ys[i] > viewport.max.y {
                    continue;
                }
                let rect =
                    Rect::from_min_size(Pos2::new(origin.x, origin.y + ys[i]), Vec2::new(width, h));
                match l {
                    Line::Header(label, n) => {
                        let p = ui.painter();
                        let weak = ui.visuals().weak_text_color();
                        let (label_x, count_x) = if i18n::rtl() {
                            (rect.right() - 12.0, rect.left() + 12.0)
                        } else {
                            (rect.left() + 12.0, rect.right() - 12.0)
                        };
                        paint_text(
                            ui,
                            p,
                            Pos2::new(label_x, rect.bottom() - 7.0),
                            start_anchor(egui::Align::Max),
                            &label.text().to_uppercase(),
                            FontId::proportional(11.5),
                            weak,
                        );
                        let end = if i18n::rtl() {
                            Align2::LEFT_BOTTOM
                        } else {
                            Align2::RIGHT_BOTTOM
                        };
                        p.text(
                            Pos2::new(count_x, rect.bottom() - 7.0),
                            end,
                            n.to_string(),
                            FontId::proportional(11.5),
                            weak.gamma_multiply(0.7),
                        );
                    }
                    Line::Row(r) => {
                        let hit = &results[*r];
                        let Some(it) = library.get(hit.id) else {
                            continue;
                        };
                        let thumb = thumb_for(it, thumbs, loader, (&mut *seen, frame));
                        let resp =
                            ui.interact(rect, egui::Id::unique(("row", it.id)), Sense::click());
                        let hot = ui.rect_contains_pointer(rect);
                        paint_row(
                            ui,
                            rect,
                            hot,
                            it,
                            hit,
                            thumb,
                            Some(it.id) == selected,
                            compact,
                            &terms,
                            max_sem,
                        );
                        if resp.clicked() {
                            actions.push(Command::Select(it.id));
                        }
                        if resp.double_clicked() {
                            actions.push(Command::Copy(it.id, Paste::Yes));
                        }
                        if hot {
                            // Hover actions replace the time at the right edge.
                            let pin = if it.pinned {
                                (Icon::PinFilled, t!("item-unpin"))
                            } else {
                                (Icon::Pin, t!("item-pin"))
                            };
                            let mut buttons = vec![
                                (Icon::Delete, t!("item-delete"), Command::Delete(it.id)),
                                (pin.0, pin.1, Command::Pin(it.id)),
                                (Icon::Copy, t!("item-copy"), Command::Copy(it.id, Paste::No)),
                            ];
                            if can_open {
                                buttons.push((
                                    Icon::Open,
                                    t!("item-open-details"),
                                    Command::Open(it.id),
                                ));
                            }
                            let size = 28.0;
                            let mut x = rect.right() - 8.0 - buttons.len() as f32 * (size + 2.0);
                            for (icon, tip, action) in buttons {
                                let b = flip(
                                    rect,
                                    Rect::from_min_size(
                                        Pos2::new(x, rect.center().y - size / 2.0),
                                        Vec2::splat(size),
                                    ),
                                );
                                if icon_button_at(
                                    ui,
                                    b,
                                    egui::Id::unique(("row-action", it.id, &tip)),
                                    icon,
                                    &tip,
                                )
                                .clicked()
                                {
                                    actions.push(action);
                                }
                                x += size + 2.0;
                            }
                        }
                    }
                }
            }
        });
        let view = (out.state.clamped_offset().y, out.inner_rect.height());
        if in_popup {
            self.popup_list_view = view;
        } else {
            self.list_view = view;
        }
        self.apply_actions(actions);
    }

    pub(super) fn empty_list(&mut self, ui: &mut Ui) {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            let strong = ui.visuals().strong_text_color();
            if self.library.items().is_empty() {
                icons::glyph(ui, Icon::Clipboard, 40.0, weak);
                ui.add_space(8.0);
                lbl(ui, &t!("list-empty-title"), 16.0, strong);
                ui.add_space(4.0);
                let help = t!("list-empty-help", key = self.cfg.popup_hotkey.as_str());
                ui.label(txt(&help, 14.0, Some(weak)));
            } else if !self.query.trim().is_empty() {
                icons::glyph(ui, Icon::Search, 36.0, weak);
                ui.add_space(8.0);
                let none = t!("list-no-matches", query = self.query.trim());
                ui.label(txt(&none, 15.0, Some(strong)));
                ui.add_space(4.0);
                match (&self.backend, self.cfg.search_mode) {
                    (BackendState::Off, _) => {
                        lbl(ui, &t!("list-meaning-finds-more"), 14.0, weak);
                        if ui
                            .link(txt(&t!("list-set-up-meaning"), 14.0, None))
                            .clicked()
                        {
                            self.open_settings(SettingsPage::Search);
                        }
                    }
                    (_, SearchMode::Keyword) => {
                        if ui.link(txt(&t!("list-try-smart"), 14.0, None)).clicked() {
                            self.cfg.search_mode = SearchMode::Hybrid;
                            self.save_config();
                            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                            self.dirty = true;
                        }
                    }
                    _ => {
                        lbl(ui, &t!("list-try-other-words"), 14.0, weak);
                    }
                }
                if self.filter != Filter::All
                    && ui
                        .link(txt(&search_all_instead(self.filter), 14.0, None))
                        .clicked()
                {
                    self.filter = Filter::All;
                    self.dirty = true;
                }
            } else {
                ui.label(txt(&none_yet(self.filter), 14.0, Some(weak)));
            }
        });
    }
}

/// The link that leaves a filter which has no matches.
fn search_all_instead(filter: Filter) -> String {
    match filter {
        Filter::Text => t!("list-search-all-not-text"),
        Filter::Links => t!("list-search-all-not-links"),
        Filter::Images => t!("list-search-all-not-images"),
        Filter::Files => t!("list-search-all-not-files"),
        Filter::Pinned | Filter::All => t!("list-search-all-not-pinned"),
    }
}

/// What an empty filter says.
fn none_yet(filter: Filter) -> String {
    match filter {
        Filter::All => t!("list-none-yet"),
        Filter::Text => t!("list-none-yet-text"),
        Filter::Links => t!("list-none-yet-links"),
        Filter::Images => t!("list-none-yet-images"),
        Filter::Files => t!("list-none-yet-files"),
        Filter::Pinned => t!("list-none-yet-pinned"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumbnails_drawn_longest_ago_are_dropped_first() {
        // Items 1 to 5, drawn in frames 10 to 50; 6 never drawn.
        let seen: HashMap<u64, u64> = (1..=5).map(|id| (id, id * 10)).collect();
        let mut dropped = thumbs_to_drop(&seen, 1..=6, 3);
        dropped.sort_unstable();
        assert_eq!(dropped, [1, 2, 6]);
        assert!(
            thumbs_to_drop(&seen, 1..=3, 3).is_empty(),
            "nothing over the limit"
        );
    }

    #[test]
    fn headers_count_their_rows() {
        let (today, yesterday) = (Group::Today, Group::Yesterday);
        let lines = with_headers([today, today, yesterday, today].into_iter());
        assert_eq!(
            lines,
            vec![
                Line::Header(today, 2),
                Line::Row(0),
                Line::Row(1),
                Line::Header(yesterday, 1),
                Line::Row(2),
                Line::Header(today, 1),
                Line::Row(3),
            ]
        );
    }

    #[test]
    fn one_big_group_stays_linear() {
        let lines = with_headers(std::iter::repeat_n(Group::Today, 100_000));
        assert_eq!(lines[0], Line::Header(Group::Today, 100_000));
        assert_eq!(lines.len(), 100_001);
    }
}
