//! The list view and the rows it paints.

use super::*;

/// Room between a row's edge and what it shows. With the list panel's
/// margin, the content lines up 16 points from the window's edge, as the
/// search field, the cards and the status bar do.
const ROW_PADDING: f32 = 8.0;

/// In a wide window, List view keeps its rows at most this wide, centred,
/// so the time and the pin stay near the text.
const LIST_MAX_WIDTH: f32 = 900.0;

/// Height of the link between the strong and the weaker matches.
pub(super) const WEAKER_LINE: f32 = 48.0;

/// The link between the strong matches and the `count` weaker ones,
/// across `rect`: it shows them below it, or hides them while `open`.
/// Returns whether it was clicked.
pub(super) fn weaker_line(ui: &Ui, rect: Rect, count: usize, open: bool) -> bool {
    let text = if open {
        t!("list-hide-weaker", count = count)
    } else {
        t!("list-show-weaker", count = count)
    };
    let resp = ui.interact(rect, egui::Id::unique("weaker-link"), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &text));
    let color = ui.visuals().hyperlink_color;
    let font = FontId::proportional(size::BODY);
    let text_w = ui
        .fonts_mut(|f| f.layout_no_wrap(text.clone(), font.clone(), color))
        .size()
        .x;
    let (icon, gap) = (16.0, 6.0);
    // As wide as the results, so it reads as their continuation, but quiet:
    // a link with its arrow at their start, a background only under the
    // pointer.
    let row = rect.shrink2(Vec2::new(0.0, 6.0));
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(
            row,
            CornerRadius::same(radius::MEDIUM),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let content = flip(
        row,
        Rect::from_min_size(
            Pos2::new(row.left() + 16.0, row.center().y - 12.0),
            Vec2::new(icon + gap + text_w, 24.0),
        ),
    );
    let icon_rect = Rect::from_min_size(
        Pos2::new(content.left(), content.center().y - icon / 2.0),
        Vec2::splat(icon),
    );
    // The arrow points down to what it shows, and up while open.
    let icon_rect = flip(content, icon_rect);
    if open {
        icons::paint_turned(p, icon_rect, Icon::Dropdown, color);
    } else {
        icons::paint(p, icon_rect, Icon::Dropdown, color);
    }
    let start = Pos2::new(content.left() + icon + gap, content.center().y);
    let start = flip(content, Rect::from_min_size(start, Vec2::ZERO)).min;
    paint_text(
        ui,
        p,
        start,
        start_anchor(egui::Align::Center),
        &text,
        font,
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Where the link between the strong and the weaker matches goes, as the
/// list and the grid draw it.
#[derive(Clone, Copy)]
pub(super) struct WeakerLinkPlace {
    /// While the link is open: the index in the results of the first
    /// weaker match, which follows the link. None while they are folded.
    pub(super) first_weak: Option<usize>,
    /// How many weaker matches there are.
    pub(super) count: usize,
}

impl WeakerLinkPlace {
    /// Draws the link across `rect`; when clicked, what it asks for.
    pub(super) fn clicked(self, ui: &Ui, rect: Rect) -> Option<Command> {
        let open = self.first_weak.is_some();
        weaker_line(ui, rect, self.count, open).then_some(if open {
            Command::HideWeaker
        } else {
            Command::ShowWeaker
        })
    }
}

impl App {
    /// The link to the weaker matches, when there are any. Weaker matches
    /// with no strong one before them are all there is, without a link.
    pub(super) fn weaker_link_place(&self) -> Option<WeakerLinkPlace> {
        let first_weak = self
            .results
            .iter()
            .position(|h| h.weak)
            .filter(|&first| first > 0);
        match first_weak {
            Some(first) => Some(WeakerLinkPlace {
                first_weak,
                count: self.results.len() - first,
            }),
            None if !self.weaker.is_empty() => Some(WeakerLinkPlace {
                first_weak: None,
                count: self.weaker.len(),
            }),
            None => None,
        }
    }
}

/// How close a match's meaning is to the best match's, in three steps: one
/// bar below the strong matches, two from there, three near the best.
/// `None` without a meaning score.
pub(super) fn relevance_level(hit: &Hit) -> Option<usize> {
    let closeness = hit.closeness?;
    Some(if closeness >= 0.75 {
        3
    } else if closeness >= crate::search::CLOSE_TO_BEST {
        2
    } else {
        1
    })
}

/// Three bars ending at `end_x` and standing on `bottom_y`, the first
/// `level` of them lit. Placed left to right, then mirrored in `within`.
pub(super) fn paint_relevance(
    p: &egui::Painter,
    within: Rect,
    end_x: f32,
    bottom_y: f32,
    level: usize,
    lit: Color32,
    unlit: Color32,
) {
    for b in 0..3 {
        let h = 4.0 + b as f32 * 3.0;
        let r = flip(
            within,
            Rect::from_min_size(
                Pos2::new(end_x - 18.0 + b as f32 * 6.0, bottom_y - h),
                Vec2::new(4.0, h),
            ),
        );
        let color = if b < level {
            lit
        } else {
            unlit.gamma_multiply(0.35)
        };
        p.rect_filled(r, CornerRadius::same(radius::SMALL), color);
    }
}

/// The day header that stays at the top of the list while its rows scroll
/// under it, and where it goes: at `top`, or above it while the next day's
/// header pushes it out. None while the top shows a header in its place.
fn pinned_header(lines: &[Line], ys: &[f32], top: f32, header_h: f32) -> Option<(usize, f32)> {
    let is_header = |i: &usize| matches!(lines[*i], Line::Header(..));
    let above = (0..lines.len()).filter(|&i| ys[i] < top).rfind(is_header)?;
    let next = (above + 1..lines.len()).find(is_header);
    Some((above, next.map_or(top, |i| top.min(ys[i] - header_h))))
}

/// A day's header in the list, as small section titles show it, on the
/// side reading starts, and how many rows it has right after.
pub(super) fn paint_header(ui: &Ui, rect: Rect, label: Group, count: usize) {
    let weak = ui.visuals().weak_text_color();
    let rtl = i18n::rtl();
    let label_x = if rtl {
        rect.right() - ROW_PADDING
    } else {
        rect.left() + ROW_PADDING
    };
    let title = section_title(&label.text().to_uppercase(), weak).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        FontId::proportional(size::SMALL),
    );
    let y = rect.bottom() - 7.0;
    let title_w = title.size().x;
    let at = start_anchor(egui::Align::Max).anchor_size(Pos2::new(label_x, y), title.size());
    ui.painter().galley(at.min, title, weak);
    // The count right after its label, so the two read together.
    let (count_at, anchor) = if rtl {
        (Pos2::new(label_x - title_w - 6.0, y), Align2::RIGHT_BOTTOM)
    } else {
        (Pos2::new(label_x + title_w + 6.0, y), Align2::LEFT_BOTTOM)
    };
    ui.painter().text(
        count_at,
        anchor,
        count.to_string(),
        FontId::proportional(size::SMALL),
        weak,
    );
}

#[derive(Debug, PartialEq)]
pub(super) enum Line {
    Header(Group, usize),
    Row(usize),
    /// The link between the strong and the weaker matches.
    WeakerLink,
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
/// right edge, so the time and badges are left out. `numbered` rows leave
/// room at the start for the numbers of quick paste, and show `number`.
/// `dated` rows sit under a day header, which gives their day; rows
/// `under_pinned` sit under the Pinned header, which says they are pinned.
/// With a `time_column` width shared by the rows, each row keeps the pin's
/// slot before it, so times, pins and relevance bars line up down the list.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_row(
    ui: &Ui,
    rect: Rect,
    hot: bool,
    it: &Item,
    hit: &Hit,
    thumb: Option<TextureHandle>,
    program_icon: Option<&TextureHandle>,
    selected: bool,
    compact: bool,
    terms: &[String],
    numbered: bool,
    number: Option<usize>,
    dated: bool,
    under_pinned: bool,
    time_column: Option<f32>,
) {
    let show_pin = it.pinned && !under_pinned;
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let a = accent();
    // Laid out left to right, then mirrored for Arabic.
    let m = |r: Rect| flip(rect, r);
    let rtl = i18n::rtl();
    let fill = if selected {
        Some(a.gamma_multiply(if v.dark_mode { 0.16 } else { 0.12 }))
    } else if hot {
        Some(v.widgets.hovered.weak_bg_fill.gamma_multiply(0.6))
    } else {
        None
    };
    if let Some(fill) = fill {
        p.rect_filled(rect, CornerRadius::same(radius::MEDIUM), fill);
    }
    if selected {
        p.rect_filled(
            m(Rect::from_min_size(
                rect.min + Vec2::new(0.0, 8.0),
                Vec2::new(3.0, rect.height() - 16.0),
            )),
            CornerRadius::same(radius::SMALL),
            a,
        );
    }

    let gutter = if numbered { NUMBER_CAP + 6.0 } else { 0.0 };
    if let Some(n) = number {
        // The number of quick paste as a key, the one Ctrl+number presses.
        let color = v.text_color();
        let label = shortcut::keycap_label(ui, &n.to_string(), color);
        let at = m(Rect::from_center_size(
            Pos2::new(rect.left() + ROW_PADDING + gutter / 2.0, rect.center().y),
            Vec2::splat(NUMBER_CAP),
        ));
        shortcut::paint_keycap(&p, at, label, color, v.dark_mode);
    }
    let av = if compact { 26.0 } else { 38.0 };
    let av_rect = m(Rect::from_min_size(
        Pos2::new(
            rect.left() + ROW_PADDING + gutter,
            rect.center().y - av / 2.0,
        ),
        Vec2::splat(av),
    ));
    // A text that is a colour or a path shows that before its program.
    let color = (it.kind == Kind::Text)
        .then(|| color_in(&it.text))
        .flatten();
    let path = (it.kind == Kind::Text).then(|| path_in(&it.text)).flatten();
    // The kind's symbol in its readable shade, so it doesn't look faded: on a
    // tile of the kind's color, or on its own in a one-line row.
    let symbol = |c: Color32, icon: Icon| {
        if !compact {
            p.rect_filled(
                av_rect,
                CornerRadius::same(radius::MEDIUM),
                c.gamma_multiply(if v.dark_mode { 0.3 } else { 0.13 }),
            );
        }
        let side = if compact { 16.0 } else { av * 0.46 };
        icons::paint(
            &p,
            Rect::from_center_size(av_rect.center(), Vec2::splat(side)),
            icon,
            text_shade(c, v.dark_mode),
        );
    };
    // On one line a picture or a swatch takes about the room of the kind's
    // symbol, so the column stays even.
    let picture = if compact {
        av_rect.shrink(4.0)
    } else {
        av_rect
    };
    match thumb {
        Some(t) => {
            let side = picture.width();
            let s = t.size_vec2();
            let k = (side / s.x).max(side / s.y);
            // Fill the square, cropping the longer side.
            let uv_w = (side / (s.x * k)).min(1.0);
            let uv_h = (side / (s.y * k)).min(1.0);
            let uv = Rect::from_min_max(
                Pos2::new((1.0 - uv_w) / 2.0, (1.0 - uv_h) / 2.0),
                Pos2::new((1.0 + uv_w) / 2.0, (1.0 + uv_h) / 2.0),
            );
            egui::Image::new((t.id(), Vec2::splat(side)))
                .uv(uv)
                .tint(theme::picture_tint(v.dark_mode))
                .corner_radius(if compact { 4.0 } else { 6.0 })
                .paint_at(ui, picture);
        }
        None if color.is_some() => {
            let swatch = if compact {
                picture
            } else {
                av_rect.shrink(3.0)
            };
            p.rect_filled(
                swatch,
                CornerRadius::same(radius::MEDIUM),
                color.expect("checked above"),
            );
            p.rect_stroke(
                swatch,
                CornerRadius::same(radius::MEDIUM),
                Stroke::new(1.0, v.weak_text_color().gamma_multiply(0.3)),
                egui::StrokeKind::Inside,
            );
        }
        None if path.is_some() => {
            let icon = match path.and_then(|p| p.extension()) {
                Some(_) => Icon::File,
                None => Icon::Folder,
            };
            symbol(kind_color(Kind::Files), icon);
        }
        None => symbol(kind_color(it.kind), item_icon(it)),
    }
    // The program it came from as a small badge on the symbol's lower corner,
    // ringed in the row's color, the plain program icon when it has none of
    // its own; one line has no room for it.
    if !compact {
        let badge = m(Rect::from_center_size(
            Pos2::new(
                rect.left() + ROW_PADDING + gutter + av - 4.0,
                rect.center().y + av / 2.0 - 4.0,
            ),
            Vec2::splat(PROGRAM_BADGE),
        ));
        let under = fill.map_or(v.panel_fill, |fill| v.panel_fill.blend(fill));
        p.rect_filled(badge.expand(2.0), CornerRadius::same(radius::SMALL), under);
        match program_icon {
            Some(icon) => {
                egui::Image::new((icon.id(), Vec2::splat(PROGRAM_BADGE))).paint_at(ui, badge);
            }
            None => icons::paint(&p, badge, Icon::AppGeneric, v.weak_text_color()),
        }
    }

    let x = rect.left() + ROW_PADDING + gutter + av + 12.0;
    let sub = if compact { String::new() } else { subtitle(it) };
    // The pin and the relevance bars sit before the time on the first
    // line, so the title ends before them.
    let bars = if relevance_level(hit).is_some() {
        22.0
    } else {
        0.0
    };
    let pin_slot = show_pin || (time_column.is_some() && bars > 0.0);
    let pin = if pin_slot { PIN_SLOT } else { 0.0 };
    let badges = pin + bars;
    // The time's column: as wide as a clock time, wider for a long weekday
    // or a date with its year, or the width the rows share.
    let time = row_time(it.created, dated);
    let time_w = time_column.unwrap_or_else(|| time_width(ui, &time).max(TIME_COLUMN));
    let right_w = if hot { 150.0 } else { time_w + 8.0 };
    let text_w = (rect.right() - x - right_w - 8.0).max(40.0);
    // While hovered, the buttons take the far end instead.
    let title_w = if hot {
        text_w
    } else {
        (text_w - badges).max(40.0)
    };
    // The text column, mirrored with the rest; lines start at its reading edge.
    let column = m(Rect::from_min_size(
        Pos2::new(x, rect.top()),
        Vec2::new(text_w, rect.height()),
    ));
    let title_color = v.strong_text_color();
    let weak = v.weak_text_color();
    let title = row_title(it);
    let title = if title.is_empty() {
        t!("item-empty")
    } else if compact && it.kind == Kind::Image && sub.is_empty() {
        // One line has no room for the size under the title: it follows it.
        format!("{title} · {}", subtitle(it))
    } else {
        title
    };

    let tg = one_line(
        ui,
        &title,
        terms,
        FontId::proportional(size::BODY),
        title_color,
        title_w,
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
        let sg = one_line(
            ui,
            &sub,
            terms,
            FontId::proportional(size::SMALL),
            weak,
            text_w,
        );
        p.galley(Pos2::new(line_x(sg.size().x), rect.top() + 32.0), sg, weak);
    }

    if hot {
        return;
    }
    // Far edge, on the first line: the time, then the pin and the
    // relevance bars before it.
    let one_line_row = compact || sub.is_empty();
    let time_y = if one_line_row {
        rect.center().y
    } else {
        rect.top() + 18.0
    };
    let time_at = m(Rect::from_min_size(
        Pos2::new(rect.right() - ROW_PADDING, time_y),
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
        &time,
        FontId::proportional(size::SMALL),
        weak,
    );
    let mut ix = rect.right() - ROW_PADDING - time_w;
    let iy = time_y;
    if show_pin {
        icons::paint(
            &p,
            m(Rect::from_center_size(
                Pos2::new(ix - PIN_SIZE / 2.0, iy),
                Vec2::splat(PIN_SIZE),
            )),
            Icon::PinFilled,
            pin_color(a, v.dark_mode),
        );
    }
    if show_pin || time_column.is_some() {
        ix -= PIN_SLOT;
    }
    if let Some(level) = relevance_level(hit) {
        paint_relevance(&p, rect, ix, iy + 6.0, level, a, weak);
    }
}

/// How big the pin of a pinned item is on rows and cards, big enough to
/// read as a pin.
pub(super) const PIN_SIZE: f32 = 15.0;

/// The pin's place before a row's time, with its gap.
const PIN_SLOT: f32 = PIN_SIZE + 5.0;

/// How wide a row's time is drawn.
fn time_width(ui: &Ui, time: &str) -> f32 {
    let color = ui.visuals().weak_text_color();
    ui.fonts_mut(|f| f.layout_no_wrap(time.to_string(), FontId::proportional(size::SMALL), color))
        .size()
        .x
}

/// How big the program's icon is on a row's symbol.
const PROGRAM_BADGE: f32 = 16.0;

/// How wide a row's time is at least, so the pins and the relevance bars
/// before short times line up.
const TIME_COLUMN: f32 = 56.0;

/// How big the numbers of quick paste are, drawn as keys.
const NUMBER_CAP: f32 = 20.0;

/// The room the line after the popup's pinned rows adds between them and
/// the others.
const GROUP_END_GAP: f32 = space::TINY;

/// How tall the fade at the end of the popup's list is.
const FADE_HEIGHT: f32 = 16.0;

/// A fade at the bottom of a scrolled list while it goes on below, into the
/// panel's color behind it.
fn fade_below<R>(ui: &Ui, out: &egui::scroll_area::ScrollAreaOutput<R>) {
    let (offset, view) = (out.state.clamped_offset().y, out.inner_rect.height());
    if offset + view < out.content_size.y - 1.0 {
        let rect = Rect::from_min_max(
            Pos2::new(out.inner_rect.left(), out.inner_rect.bottom() - FADE_HEIGHT),
            out.inner_rect.max,
        );
        paint_fade(ui.painter(), rect, ui.visuals().panel_fill);
    }
}

/// A fade over `rect`, from clear at its top to `color` at its bottom.
fn paint_fade(p: &egui::Painter, rect: Rect, color: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.right_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.right_bottom(), color);
    mesh.colored_vertex(rect.left_bottom(), color);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
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
            theme::search_hit(ui.visuals().dark_mode),
            width,
            1,
            base,
        )
        .into_iter()
        .next()
        .map(|(g, _)| g)
        .unwrap_or_else(|| {
            ui.fonts_mut(|f| {
                f.layout_no_wrap(String::new(), FontId::proportional(size::SMALL), color)
            })
        })
    } else {
        let v = ui.visuals();
        let hit = (v.strong_text_color(), theme::search_hit(v.dark_mode));
        ui.fonts_mut(|f| f.layout_job(highlighted(text, terms, font, color, hit, width, 1)))
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
        let centred = self.cfg.view == ViewMode::List && !in_popup;
        if !in_popup {
            // As wide as the rows, and where they are.
            let full = ui.available_width();
            let width = if centred {
                full.min(LIST_MAX_WIDTH)
            } else {
                full
            };
            self.checklist_above(ui, (full - width) / 2.0, width);
        }
        // The popup's rows take one line and no day headers, so its nine
        // numbered items fit without scrolling.
        let compact = self.cfg.density == Density::Compact || in_popup;
        let row_h = if compact { 40.0 } else { 58.0 };
        let header_h = 30.0;
        let searching = !self.query.trim().is_empty();
        // The details pane is always beside the list in Detail view, so rows
        // there don't need an "open" button; the popup has no details pane.
        let can_open = self.cfg.view != ViewMode::Detail && !in_popup;

        // Rows, with date/pinned headers when browsing.
        let mut lines: Vec<Line> = Vec::with_capacity(self.results.len() + 8);
        // Day headers only make sense in date order.
        let day_headers =
            !searching && !in_popup && self.cfg.group_by_date && self.sort() == SortOrder::Recent;
        let pinned_first = self.cfg.pinned_first;
        if day_headers {
            lines = with_headers(self.results.iter().map(|h| {
                self.library
                    .get(h.id)
                    .map_or(Group::Older, |it| Group::of(it, pinned_first))
            }));
        } else {
            lines.extend((0..self.results.len()).map(Line::Row));
        }
        // The link to the weaker matches: right above them while they are
        // open, after the results while they are folded under it.
        let link = self.weaker_link_place();
        if let Some(link) = link {
            let at = link
                .first_weak
                .and_then(|first| lines.iter().position(|l| *l == Line::Row(first)))
                .unwrap_or(lines.len());
            lines.insert(at, Line::WeakerLink);
        }
        // The popup has no day headers: while browsing with pinned items
        // first, a thin line ends their group, before the first other row.
        let first_unpinned = (in_popup && !searching && pinned_first)
            .then(|| {
                self.results
                    .iter()
                    .take_while(|h| self.library.get(h.id).is_some_and(|it| it.pinned))
                    .count()
            })
            .filter(|&n| n > 0 && n < self.results.len());
        let mut ys = Vec::with_capacity(lines.len());
        let mut y = 0.0;
        for l in &lines {
            ys.push(y);
            y += match l {
                Line::Header(..) => header_h,
                Line::Row(r) if Some(r + 1) == first_unpinned => row_h + 2.0 + GROUP_END_GAP,
                Line::Row(_) => row_h + 2.0,
                Line::WeakerLink => WEAKER_LINE,
            };
        }
        let total = y;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if in_popup {
            // Its own scroll place, apart from the full window's list.
            sa = sa.id_salt("popup-list");
        }
        let (off, view_h) = if in_popup {
            self.popup_list_view
        } else {
            self.list_view
        };
        // Until the list has been drawn once, its height is unknown and every
        // row would seem to be below it: the scroll waits for that frame.
        if self.scroll_to_selected && view_h > 0.0 {
            self.scroll_to_selected = false;
            if let Some(i) = lines.iter().position(
                |l| matches!(l, Line::Row(r) if Some(self.results[*r].id) == self.selected),
            ) {
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
        // A right-click, or Shift+F10 on the selected row, opens a menu.
        let mut open_menu = None;
        let menu_key = self.menu_by_key;
        // After a delete the rows below glide up into the gap.
        let moving = ui.ctx().style_of(ui.ctx().theme()).animation_time > 0.0;
        let glide = if moving && self.glide_until.is_some_and(|t| Instant::now() < t) {
            GLIDE_TIME
        } else {
            0.0
        };
        // The hover buttons' tips name their keys.
        let keys = self.cfg.keys.clone();
        // While an item's menu is open, the rows under it stay calm.
        let menu_open = self
            .item_menu
            .as_ref()
            .is_some_and(|m| m.in_popup == in_popup);
        let frame = ui.ctx().cumulative_pass_nr();
        let (library, results, thumbs, loader, selected, seen, programs, marked) = (
            &self.library,
            &self.results,
            &self.thumbs,
            &mut self.loader,
            self.selected,
            &mut self.thumb_seen,
            &mut self.programs,
            &self.marked,
        );
        let out = sa.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let full = ui.available_width();
            let width = if centred {
                full.min(LIST_MAX_WIDTH)
            } else {
                full
            };
            let left = origin.x + (full - width) / 2.0;
            let height = |l: &Line| match l {
                Line::Header(..) => header_h,
                Line::Row(_) => row_h,
                Line::WeakerLink => WEAKER_LINE,
            };
            let shown =
                |i: usize| ys[i] + height(&lines[i]) >= viewport.min.y && ys[i] <= viewport.max.y;
            // The popup's times share one column, as wide as the widest
            // shown, so its pins and times line up down the list.
            let time_column = in_popup.then(|| {
                (0..lines.len())
                    .filter(|&i| shown(i))
                    .filter_map(|i| match lines[i] {
                        Line::Row(r) => library.get(results[r].id),
                        _ => None,
                    })
                    .map(|it| time_width(ui, &row_time(it.created, false)))
                    .fold(0.0, f32::max)
            });
            if let Some(i) = first_unpinned
                .and_then(|n| lines.iter().position(|l| *l == Line::Row(n - 1)))
                .filter(|&i| shown(i))
            {
                let y = origin.y + ys[i] + row_h + (2.0 + GROUP_END_GAP) / 2.0;
                let x = (left + ROW_PADDING)..=(left + width - ROW_PADDING);
                let color = line_color(ui.visuals().dark_mode);
                ui.painter().hline(x, y, Stroke::new(1.0, color));
            }
            for (i, l) in lines.iter().enumerate() {
                let h = height(l);
                if !shown(i) {
                    continue;
                }
                let top = match l {
                    Line::Row(r) => ui.ctx().animate_value_with_time(
                        egui::Id::unique(("row-y", results[*r].id)),
                        ys[i],
                        glide,
                    ),
                    Line::Header(..) | Line::WeakerLink => ys[i],
                };
                let rect =
                    Rect::from_min_size(Pos2::new(left, origin.y + top), Vec2::new(width, h));
                match l {
                    Line::Header(label, n) => paint_header(ui, rect, *label, *n),
                    Line::WeakerLink => {
                        if let Some(link) = link {
                            actions.extend(link.clicked(ui, rect));
                        }
                    }
                    Line::Row(r) => {
                        let hit = &results[*r];
                        let Some(it) = library.get(hit.id) else {
                            continue;
                        };
                        let thumb = thumb_for(it, thumbs, loader, (&mut *seen, frame));
                        let program = it.program.as_deref().and_then(|p| programs.get(p));
                        let resp =
                            ui.interact(rect, egui::Id::unique(("row", it.id)), Sense::CLICK);
                        let chosen = Some(it.id) == selected;
                        // Selected alone or together with others.
                        let lit = chosen || (!in_popup && marked.contains(&it.id));
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::Role::ListBoxOption,
                                true,
                                lit,
                                item_name(it),
                            )
                        });
                        let hot = !menu_open && ui.rect_contains_pointer(rect);
                        paint_row(
                            ui,
                            rect,
                            hot,
                            it,
                            hit,
                            thumb,
                            program.and_then(|p| p.icon.as_ref()),
                            lit,
                            compact,
                            &terms,
                            in_popup,
                            (in_popup && *r < 9).then_some(*r + 1),
                            day_headers && Group::of(it, pinned_first) != Group::Pinned,
                            day_headers && Group::of(it, pinned_first) == Group::Pinned,
                            time_column,
                        );
                        if resp.clicked() {
                            // Ctrl and Shift select several, but not in the popup.
                            let keys = ui.input(|i| i.modifiers);
                            actions.push(match (in_popup, keys.command, keys.shift) {
                                (false, true, _) => Command::ToggleMark(it.id),
                                (false, _, true) => Command::MarkRange(it.id),
                                _ => Command::Select(it.id),
                            });
                        }
                        if resp.double_clicked() {
                            actions.push(Command::Copy(it.id, Paste::Yes));
                        }
                        if resp.secondary_clicked() {
                            let at = resp.interact_pointer_pos().unwrap_or(rect.center());
                            open_menu = Some((it.id, at, false));
                        } else if menu_key && chosen {
                            let below_text = Pos2::new(rect.left() + 56.0, rect.bottom() - 4.0);
                            let at = flip(rect, Rect::from_min_size(below_text, Vec2::ZERO)).min;
                            open_menu = Some((it.id, at, true));
                        }
                        if hot {
                            // Hover actions replace the time at the right edge;
                            // Delete keeps apart from the others.
                            let pin = if it.pinned {
                                (Icon::PinFilled, hint(&t!("item-unpin"), &keys.pin))
                            } else {
                                (Icon::Pin, hint(&t!("item-pin"), &keys.pin))
                            };
                            let mut buttons = vec![
                                (
                                    Icon::Delete,
                                    hint(&t!("item-delete"), &keys.delete),
                                    Command::Delete(it.id),
                                ),
                                (pin.0, pin.1, Command::Pin(it.id)),
                                (
                                    Icon::Copy,
                                    hint(&t!("item-copy"), &keys.copy_only),
                                    Command::Copy(it.id, Paste::No),
                                ),
                            ];
                            if can_open {
                                buttons.push((
                                    Icon::Open,
                                    t!("item-open-details"),
                                    Command::Open(it.id),
                                ));
                            }
                            let (size, apart) = (28.0, 14.0);
                            let mut x = rect.right()
                                - ROW_PADDING
                                - buttons.len() as f32 * (size + 2.0)
                                - apart;
                            for (i, (icon, tip, action)) in buttons.into_iter().enumerate() {
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
                                x += size + 2.0 + if i == 0 { apart } else { 0.0 };
                            }
                        }
                    }
                }
            }
            if let Some((Line::Header(label, n), y)) =
                pinned_header(&lines, &ys, viewport.min.y, header_h).map(|(i, y)| (&lines[i], y))
            {
                let (label, n) = (*label, *n);
                let rect =
                    Rect::from_min_size(Pos2::new(left, origin.y + y), Vec2::new(width, header_h));
                let below = Rect::from_min_max(
                    rect.left_bottom(),
                    rect.right_bottom() + Vec2::new(0.0, 10.0),
                );
                let shadow = egui::Shadow {
                    offset: [0, 1],
                    blur: 8,
                    spread: 0,
                    color: theme::header_shadow(),
                };
                ui.painter()
                    .with_clip_rect(below)
                    .add(shadow.as_shape(rect, CornerRadius::ZERO));
                ui.painter()
                    .rect_filled(rect, CornerRadius::ZERO, ui.visuals().panel_fill);
                // Rows under it don't take its clicks.
                let _ = ui.interact(rect, egui::Id::unique("day-header"), Sense::click());
                paint_header(ui, rect, label, n);
            }
        });
        let view = (out.state.clamped_offset().y, out.inner_rect.height());
        if in_popup {
            // The popup has no status bar: a fade where its list goes on.
            fade_below(ui, &out);
            self.popup_list_view = view;
        } else {
            self.list_view = view;
        }
        self.apply_actions(actions);
        self.after_rows(open_menu, in_popup);
        self.item_menu(ui, in_popup);
    }

    pub(super) fn empty_list(&mut self, ui: &mut Ui) {
        ui.add_space(space::EMPTY_PAGE);
        ui.vertical_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            let strong = ui.visuals().strong_text_color();
            if self.library.items().is_empty() {
                icons::glyph(ui, Icon::Clipboard, 40.0, weak);
                ui.add_space(space::SMALL);
                title_lbl(ui, &t!("list-empty-title"), size::LARGE, strong);
                ui.add_space(space::TINY);
                if self.cfg.checklist.closed {
                    let help = t!("list-empty-help", key = self.cfg.popup_hotkey.as_str());
                    ui.label(txt(&help, size::BODY, Some(weak)));
                } else {
                    ui.add_space(space::MEDIUM);
                    ui.scope(|ui| {
                        ui.set_max_width(ui.available_width().min(460.0));
                        self.checklist(ui);
                    });
                }
            } else if !self.query.trim().is_empty() {
                icons::glyph(ui, Icon::Search, 36.0, weak);
                ui.add_space(space::SMALL);
                let none = t!("list-no-matches", query = self.query.trim());
                ui.label(txt(&none, size::BODY, Some(strong)));
                ui.add_space(space::TINY);
                match (&self.backend, self.cfg.search_mode) {
                    (BackendState::Off, _) => {
                        lbl(ui, &t!("list-meaning-finds-more"), size::BODY, weak);
                        if ui
                            .link(txt(&t!("list-set-up-meaning"), size::BODY, None))
                            .clicked()
                        {
                            self.open_settings(SettingsPage::Search);
                        }
                    }
                    (_, SearchMode::Keyword) => {
                        if ui
                            .link(txt(&t!("list-try-smart"), size::BODY, None))
                            .clicked()
                        {
                            self.cfg.search_mode = SearchMode::Hybrid;
                            self.save_config();
                            self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                            self.dirty = true;
                        }
                    }
                    _ => {
                        lbl(ui, &t!("list-try-other-words"), size::BODY, weak);
                    }
                }
                if self.filter != Filter::All
                    && ui
                        .link(txt(&search_all_instead(self.filter), size::BODY, None))
                        .clicked()
                {
                    self.filter = Filter::All;
                    self.dirty = true;
                }
            } else {
                ui.label(txt(&none_yet(self.filter), size::BODY, Some(weak)));
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
    fn the_day_at_the_top_keeps_its_header_until_the_next_one_comes() {
        // Today's header, two rows, yesterday's header, a row.
        let lines = [
            Line::Header(Group::Today, 2),
            Line::Row(0),
            Line::Row(1),
            Line::Header(Group::Yesterday, 1),
            Line::Row(2),
        ];
        let ys = [0.0, 30.0, 90.0, 150.0, 180.0];
        // At the very top the header is in its own place.
        assert_eq!(pinned_header(&lines, &ys, 0.0, 30.0), None);
        // Scrolled into today's rows: today's header stays at the top.
        assert_eq!(pinned_header(&lines, &ys, 50.0, 30.0), Some((0, 50.0)));
        // Yesterday's header 20 points below the top pushes it 10 up.
        assert_eq!(pinned_header(&lines, &ys, 130.0, 30.0), Some((0, 120.0)));
        // Past yesterday's header, that one stays.
        assert_eq!(pinned_header(&lines, &ys, 160.0, 30.0), Some((3, 160.0)));
    }

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

    /// Where the parts of a pinned row with a second line go: its title's
    /// and its time's text, its relevance bars, and the row. A `compact`
    /// row shows one line.
    fn row_parts(compact: bool) -> (Rect, Rect, Vec<Rect>, Rect) {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut it = Item::new(
            1,
            Kind::Text,
            Origin::Clipboard,
            "Meeting moved to Thursday\nRoom 4B, at 10:30".into(),
        );
        it.pinned = true;
        let hit = Hit {
            id: 1,
            closeness: Some(1.0),
            weak: false,
        };
        let row_h = if compact { 40.0 } else { 58.0 };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = Rect::from_min_size(Pos2::new(0.0, 100.0), Vec2::new(600.0, row_h));
            let (thumb, program, numbered, number) = (None, None, false, None);
            paint_row(
                ui,
                rect,
                false,
                &it,
                &hit,
                thumb,
                program,
                false,
                compact,
                &[],
                numbered,
                number,
                false,
                false,
                None,
            );
        });
        output.textures_delta.clear();
        let mut texts = Vec::new();
        let mut bars = Vec::new();
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::Shape::Text(text) => {
                    texts.push((text.galley.text().to_string(), text.visual_bounding_rect()));
                }
                egui::Shape::Rect(shape) if shape.rect.width() == 4.0 => bars.push(shape.rect),
                _ => {}
            }
        }
        let find = |words: &str| {
            texts
                .iter()
                .find(|(text, _)| text.starts_with(words))
                .map(|(_, at)| *at)
                .expect(words)
        };
        let time = row_time(it.created, false);
        let row = Rect::from_min_size(Pos2::new(0.0, 100.0), Vec2::new(600.0, row_h));
        (find("Meeting"), find(&time), bars, row)
    }

    /// The pin and the relevance bars come before the time on a row's
    /// first line, also when the row has a second line: in reading order,
    /// bars, pin, time, after the title.
    #[test]
    fn a_rows_relevance_bars_sit_on_its_first_line() {
        for compact in [false, true] {
            let (title, time, bars, row) = row_parts(compact);
            assert_eq!(bars.len(), 3, "three bars");
            for bar in &bars {
                // On the title's line, beside the time.
                assert!(bar.bottom() <= title.bottom(), "{bar:?} {title:?}");
                assert!(bar.bottom() >= time.top(), "{bar:?} {time:?}");
                assert!(bar.top() >= row.top(), "{bar:?} {row:?}");
                // After the title and before the time, with the pin between.
                assert!(bar.left() > title.right(), "{bar:?} {title:?}");
                assert!(bar.right() + PIN_SIZE <= time.left(), "{bar:?} {time:?}");
            }
        }
    }

    /// Two of the popup's rows sharing a time column: a pinned one copied
    /// now, and one copied over a year ago, whose time is longer. Their
    /// relevance bars, after the pin's place, and their times line up.
    #[test]
    fn popup_rows_line_up_their_pins_and_times() {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut new = Item::new(1, Kind::Text, Origin::Clipboard, "Pinned note".into());
        new.pinned = true;
        let mut old = Item::new(2, Kind::Text, Origin::Clipboard, "Old note".into());
        old.created -= 400 * 86_400;
        let times = [&new, &old].map(|it| row_time(it.created, false));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let column = times
                .iter()
                .map(|time| time_width(ui, time))
                .fold(0.0, f32::max);
            for (n, it) in [&new, &old].into_iter().enumerate() {
                let hit = Hit {
                    id: it.id,
                    closeness: Some(1.0),
                    weak: false,
                };
                let top = 100.0 + 42.0 * n as f32;
                let rect = Rect::from_min_size(Pos2::new(0.0, top), Vec2::new(420.0, 40.0));
                paint_row(
                    ui,
                    rect,
                    false,
                    it,
                    &hit,
                    None,
                    None,
                    false,
                    true,
                    &[],
                    true,
                    Some(n + 1),
                    false,
                    false,
                    Some(column),
                );
            }
        });
        output.textures_delta.clear();
        let mut bars: Vec<Rect> = Vec::new();
        let mut time_ends = Vec::new();
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect.width() == 4.0 => bars.push(shape.rect),
                egui::Shape::Text(text) if times.contains(&text.galley.text().to_string()) => {
                    time_ends.push(text.pos.x + text.galley.rect.right());
                }
                _ => {}
            }
        }
        assert_eq!(bars.len(), 6, "three bars a row");
        assert_eq!(time_ends.len(), 2, "{times:?}");
        assert!(times[0] != times[1], "{times:?}");
        // Bar by bar, the second row's under the first's.
        bars.sort_by(|a, b| a.top().total_cmp(&b.top()));
        let (first, second) = bars.split_at(3);
        let lefts = |row: &[Rect]| {
            let mut x: Vec<f32> = row.iter().map(|bar| bar.left()).collect();
            x.sort_by(f32::total_cmp);
            x
        };
        assert_eq!(lefts(first), lefts(second));
        assert!((time_ends[0] - time_ends[1]).abs() <= 0.5, "{time_ends:?}");
    }

    /// Closed or open, a screen reader hears the link as a button with the
    /// words it shows.
    #[test]
    fn the_weaker_matches_link_is_a_button_named_by_its_words() {
        for (open, words) in [
            (false, t!("list-show-weaker", count = 14)),
            (true, t!("list-hide-weaker", count = 14)),
        ] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let rect = Rect::from_min_size(ui.max_rect().min, Vec2::new(400.0, WEAKER_LINE));
                weaker_line(ui, rect, 14, open);
            });
            // The font atlas upload no renderer takes here.
            output.textures_delta.clear();
            let update = output.platform_output.accesskit_update.expect("a tree");
            let buttons: Vec<String> = update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::Button)
                .filter_map(|(_, node)| node.label().map(str::to_string))
                .collect();
            assert_eq!(buttons, [words]);
        }
    }

    #[test]
    fn one_big_group_stays_linear() {
        let lines = with_headers(std::iter::repeat_n(Group::Today, 100_000));
        assert_eq!(lines[0], Line::Header(Group::Today, 100_000));
        assert_eq!(lines.len(), 100_001);
    }
}
