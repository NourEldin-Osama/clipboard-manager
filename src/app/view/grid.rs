//! The grid view and the cards it paints.

use super::*;
use crate::app::programs::Program;

/// Between cards, across and down.
pub(crate) const GRID_GAP: f32 = 14.0;

/// The room beside the cards in the main area: the grid's margins and the
/// place it keeps for the scroll bar.
pub(crate) const GRID_SIDES: f32 = 16.0 + 4.0 + 12.0;

/// How many cards fit side by side in `width`, each at least 240 points
/// wide.
pub(crate) fn cards_per_row(width: f32) -> usize {
    ((width + GRID_GAP) / (240.0 + GRID_GAP)).floor().max(1.0) as usize
}

/// Height of the header above weaker matches shown with the strong ones,
/// as tall as a day's header in the list.
const WEAKER_HEADER: f32 = 30.0;

/// How tall a card is when cards are as tall as their content (D69): its
/// content measured roughly, between half and 1.6 times the height cards
/// have when they are all the same (`same`).
fn fitted_height(it: &Item, card_w: f32, same: f32) -> f32 {
    let pad = 16.0;
    let body_w = (card_w - 2.0 * pad).max(1.0);
    let tags = if it.tags.is_empty() { 0.0 } else { TAG_ROW };
    let body = if it.kind == Kind::Image {
        if it.width > 0 && it.height > 0 {
            (body_w * it.height as f32 / it.width as f32).min(it.height as f32 * 2.0)
        } else {
            80.0
        }
    } else if it.kind == Kind::Text && color_in(&it.text).is_some() {
        // The swatch, then its two lines.
        110.0
    } else if it.kind == Kind::Text && path_in(&it.text).is_some() {
        // The icon, then the name and the folder.
        80.0
    } else {
        let text = match it.kind {
            Kind::Link => row_title(it),
            _ => it.text.lines().take(14).collect::<Vec<_>>().join("\n"),
        };
        let code = is_code(it);
        let short = !code && text.chars().count() <= 60 && text.lines().count() <= 2;
        // As `paint_card` lays it out: its font, the line height and about
        // how wide a letter is.
        let (letter, line_h) = if code {
            (size::SMALL * 0.6, 17.0)
        } else if short {
            (size::LARGE * 0.5, 23.0)
        } else {
            (size::BODY * 0.5, 20.0)
        };
        let lines: f32 = text
            .lines()
            .map(|l| (l.chars().count() as f32 * letter / body_w).ceil().max(1.0))
            .sum();
        lines.max(1.0) * line_h + 2.0
    };
    (pad + body + 4.0 + tags + CARD_FOOTER).clamp(same * 0.5, same * 1.6)
}

/// Cards `heights` tall stacked in `cols` columns `card_w` wide, each in
/// the shortest column so far, the leftmost of equals: where each goes from
/// the top left.
fn stack(heights: &[f32], cols: usize, card_w: f32, gap: f32) -> Vec<Rect> {
    let mut tops = vec![0.0f32; cols.max(1)];
    heights
        .iter()
        .map(|&h| {
            let col = (0..tops.len())
                .min_by(|&a, &b| tops[a].total_cmp(&tops[b]))
                .unwrap_or(0);
            let place = Rect::from_min_size(
                Pos2::new(col as f32 * (card_w + gap), tops[col]),
                Vec2::new(card_w, h),
            );
            tops[col] += h + gap;
            place
        })
        .collect()
}

/// Height of the action strip a card shows while hovered.
pub(super) const CARD_FOOTER: f32 = 42.0;

/// Height of a card's tags row, above the action strip.
pub(super) const TAG_ROW: f32 = 28.0;

/// One card: its content, the tags row when it has tags, and a quiet footer
/// with the program it came from and when. While hovered, the footer holds
/// the buttons instead, drawn by the grid.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_card(
    ui: &Ui,
    rect: Rect,
    hot: bool,
    it: &Item,
    thumb: Option<TextureHandle>,
    program: Option<&Program>,
    relevance: Option<usize>,
    selected: bool,
    terms: &[String],
) {
    let v = ui.visuals();
    let a = accent();
    let radius = CornerRadius::same(radius::LARGE);
    let p = ui.painter();
    // The surface settings cards share: fill, shadow and the dark edge.
    let surface = theme::card_frame(v, hot);
    p.add(surface.paint(rect - surface.total_margin()));
    // A selected card is tinted as well as outlined, so it stands out as
    // more than a focus ring. Its outline covers the edge under the tint.
    if selected {
        p.rect_filled(rect, radius, theme::selected_card(a, v.dark_mode));
    }
    let pad = 16.0;
    let weak = v.weak_text_color();

    // Bottom up: the footer, the tags row above it when there are tags,
    // then the content. Hovering only changes the footer, so nothing else
    // moves.
    let footer = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - CARD_FOOTER),
        rect.max,
    );
    if hot {
        let r = CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius::LARGE,
            se: radius::LARGE,
        };
        let fill = theme::card_footer(v.dark_mode);
        p.rect_filled(footer, r, fill);
    } else {
        paint_footer(
            ui,
            footer.shrink2(Vec2::new(pad, 0.0)),
            it,
            program,
            relevance,
        );
    }
    let strip_top = footer.top();
    let tags_top = if it.tags.is_empty() {
        strip_top
    } else {
        strip_top - TAG_ROW
    };
    if !it.tags.is_empty() {
        paint_tags(ui, rect, it, pad, tags_top, strip_top);
    }
    let body = Rect::from_min_max(
        Pos2::new(rect.left() + pad, rect.top() + pad),
        Pos2::new(rect.right() - pad, tags_top - 4.0),
    );
    let bp = ui.painter_at(body);
    let color = (it.kind == Kind::Text)
        .then(|| color_in(&it.text))
        .flatten();
    let path = (it.kind == Kind::Text).then(|| path_in(&it.text)).flatten();
    match (it.kind, thumb) {
        // Every picture in the same frame: as big as fits, in the middle.
        (Kind::Image, Some(t)) => {
            let s = t.size_vec2();
            let k = (body.width() / s.x).min(body.height() / s.y).min(2.0);
            let img = Rect::from_center_size(body.center(), s * k);
            egui::Image::new((t.id(), s * k))
                .tint(theme::picture_tint(v.dark_mode))
                .corner_radius(6.0)
                .paint_at(ui, img);
            bp.rect_stroke(
                img,
                CornerRadius::same(radius::MEDIUM),
                Stroke::new(1.0, weak.gamma_multiply(0.25)),
                egui::StrokeKind::Inside,
            );
        }
        (Kind::Image, None) => {
            bp.text(
                body.center(),
                Align2::CENTER_CENTER,
                format!("{} × {}", it.width, it.height),
                FontId::proportional(size::CONTROL),
                weak,
            );
        }
        _ if color.is_some() => {
            let color = color.expect("checked above");
            let swatch = Rect::from_min_size(
                body.min,
                Vec2::new(body.width(), (body.height() - 46.0).max(24.0)),
            );
            bp.rect_filled(swatch, CornerRadius::same(radius::MEDIUM), color);
            bp.rect_stroke(
                swatch,
                CornerRadius::same(radius::MEDIUM),
                Stroke::new(1.0, weak.gamma_multiply(0.3)),
                egui::StrokeKind::Inside,
            );
            let below = Pos2::new(body.left(), swatch.bottom() + 8.0);
            let lines = [
                (
                    it.text.trim().to_string(),
                    size::BODY,
                    v.strong_text_color(),
                ),
                (
                    t!("item-color", value = color_other_way(&it.text, color)),
                    size::SMALL,
                    weak,
                ),
            ];
            // On the side of the color's value, which is written left to right.
            let rtl = bidi::starts_rtl(&it.text, i18n::rtl());
            paint_lines(ui, &bp, body, below, &lines, rtl);
        }
        _ if path.is_some() => {
            let path = path.expect("checked above");
            let c = kind_color(Kind::Files);
            let icon = match path.extension() {
                Some(_) => Icon::File,
                None => Icon::Folder,
            };
            // On the side the path is written from, like its name.
            let rtl = bidi::starts_rtl(&it.text, i18n::rtl());
            let at = flip_if(rtl, body, Rect::from_min_size(body.min, Vec2::splat(28.0)));
            icons::paint(&bp, at, icon, text_shade(c, v.dark_mode));
            let name = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            let folder = path
                .parent()
                .map(|f| f.display().to_string())
                .unwrap_or_default();
            let below = Pos2::new(body.left(), at.bottom() + 10.0);
            let lines = [
                (name, size::LARGE, v.strong_text_color()),
                (folder, size::SMALL, weak),
            ];
            paint_lines(ui, &bp, body, below, &lines, rtl);
        }
        _ => {
            let code = is_code(it);
            let text: String = match it.kind {
                Kind::Files => it
                    .text
                    .lines()
                    .map(|l| {
                        std::path::Path::new(l)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| l.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                // As rows show it: without https:// and www., letters decoded.
                Kind::Link => row_title(it),
                _ => it
                    .text
                    .lines()
                    .take(14)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .chars()
                    .take(900)
                    .collect(),
            };
            let text = if text.trim().is_empty() {
                t!("item-empty")
            } else {
                text
            };
            // Short text shows larger, so the card is used.
            let short = !code && text.chars().count() <= 60 && text.lines().count() <= 2;
            // Code a step smaller than text, not two, so it reads as easily.
            let (font, line_h) = if code {
                (FontId::monospace(size::CONTROL), 18.0)
            } else if short {
                (FontId::proportional(size::LARGE), 23.0)
            } else {
                (FontId::proportional(size::BODY), 20.0)
            };
            let color = if it.kind == Kind::Link {
                v.hyperlink_color
            } else {
                v.strong_text_color()
            };
            let rows = ((body.height() / line_h).floor() as usize).max(1);
            if bidi::has_rtl(&text) {
                // Each line lines up with its own reading edge.
                let lines = bidi::lines(
                    ui,
                    text.trim_end(),
                    terms,
                    font,
                    color,
                    theme::search_hit(v.dark_mode),
                    body.width(),
                    rows,
                    None,
                );
                bidi::paint(&bp, body, lines, color, None, 0.0);
            } else {
                let g = ui.fonts_mut(|f| {
                    f.layout_job(highlighted(
                        text.trim_end(),
                        terms,
                        font,
                        color,
                        (v.strong_text_color(), theme::search_hit(v.dark_mode)),
                        body.width(),
                        rows,
                    ))
                });
                bp.galley(body.min, g, color);
            }
        }
    }
    // Outline last, so the footer doesn't cover it. On a dark card the
    // accent's lighter shade and a little wider, so it stands out from the
    // dark cards around.
    if selected {
        let (edge, width) = if v.dark_mode {
            (text_shade(a, true), 2.5)
        } else {
            (a, 2.0)
        };
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(width, edge),
            egui::StrokeKind::Inside,
        );
    } else if hot {
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, weak.gamma_multiply(0.35)),
            egui::StrokeKind::Inside,
        );
    }
}

/// Lines of text from `at` down, on the right when `rtl`, else on the left.
fn paint_lines(
    ui: &Ui,
    p: &egui::Painter,
    body: Rect,
    at: Pos2,
    lines: &[(String, f32, Color32)],
    rtl: bool,
) {
    let mut y = at.y;
    let anchor = if rtl {
        Align2::RIGHT_TOP
    } else {
        Align2::LEFT_TOP
    };
    for (text, text_size, color) in lines {
        let start = flip_if(
            rtl,
            body,
            Rect::from_min_size(Pos2::new(body.left(), y), Vec2::ZERO),
        )
        .min;
        let painted = paint_text(
            ui,
            p,
            start,
            anchor,
            text,
            FontId::proportional(*text_size),
            *color,
        );
        y = painted.bottom() + 2.0;
    }
}

/// The program a card came from, with its icon, and when, with the pin and
/// how close a search match is to the best one.
fn paint_footer(
    ui: &Ui,
    row: Rect,
    it: &Item,
    program: Option<&Program>,
    relevance: Option<usize>,
) {
    let v = ui.visuals();
    let weak = v.weak_text_color();
    let p = ui.painter_at(row);
    let y = row.center().y;
    // The far end first: the time, and the pin before it.
    let end = flip(
        row,
        Rect::from_min_size(Pos2::new(row.right(), y), Vec2::ZERO),
    )
    .min;
    let end_anchor = if i18n::rtl() {
        Align2::LEFT_CENTER
    } else {
        Align2::RIGHT_CENTER
    };
    let age = paint_text(
        ui,
        &p,
        end,
        end_anchor,
        &ago(it.created),
        FontId::proportional(size::SMALL),
        weak,
    );
    let mut used = age.width() + 8.0;
    if it.pinned {
        // Big enough to read as a pin beside the time.
        let x = if i18n::rtl() {
            age.right() + 15.0
        } else {
            age.left() - 15.0
        };
        icons::paint(
            &p,
            Rect::from_center_size(Pos2::new(x, y), Vec2::splat(PIN_SIZE)),
            Icon::PinFilled,
            accent(),
        );
        used += 22.0;
    }
    if let Some(level) = relevance {
        paint_relevance(&p, row, row.right() - used, y + 5.0, level, accent(), weak);
        used += 26.0;
    }
    // Then the program at the reading start: its own icon, or the plain one
    // of a program without its own, and its name, or for older copies the
    // program's file name. The card shows its kind itself.
    let icon_rect = flip(
        row,
        Rect::from_center_size(Pos2::new(row.left() + 8.0, y), Vec2::splat(16.0)),
    );
    match program.and_then(|p| p.icon.as_ref()) {
        Some(texture) => {
            egui::Image::new((texture.id(), Vec2::splat(16.0))).paint_at(ui, icon_rect);
        }
        None => icons::paint(&p, icon_rect, Icon::AppGeneric, weak),
    }
    let name_x = 22.0;
    let room = row.width() - used - name_x;
    let name = match (program, &it.app) {
        (Some(program), _) => program.name.clone(),
        (None, Some(app)) => crate::app::programs::tidy(app),
        (None, None) => it.kind.label(),
    };
    let start = flip(
        row,
        Rect::from_min_size(Pos2::new(row.left() + name_x, y), Vec2::ZERO),
    )
    .min;
    let font = FontId::proportional(size::SMALL);
    let full = ui.fonts_mut(|f| f.layout_no_wrap(name.clone(), font.clone(), weak));
    let shown = if full.size().x > room {
        let mut cut = name.clone();
        while !cut.is_empty() {
            cut.pop();
            let candidate = format!("{}…", cut.trim_end());
            if ui
                .fonts_mut(|f| f.layout_no_wrap(candidate.clone(), font.clone(), weak))
                .size()
                .x
                <= room
            {
                break;
            }
        }
        format!("{}…", cut.trim_end())
    } else {
        name
    };
    // The program in the normal text color, a step above the time.
    paint_text(
        ui,
        &p,
        start,
        start_anchor(egui::Align::Center),
        &shown,
        font,
        v.text_color(),
    );
}

/// The tags row: as many tags as fit, then "+N tags".
fn paint_tags(ui: &Ui, rect: Rect, it: &Item, pad: f32, tags_top: f32, strip_top: f32) {
    let v = ui.visuals();
    let weak = v.weak_text_color();
    let row = Rect::from_min_max(
        Pos2::new(rect.left() + pad, tags_top),
        Pos2::new(rect.right() - pad, strip_top - 2.0),
    );
    let tp = ui.painter_at(row);
    let pill = |text: &str, c: Color32| {
        ui.fonts_mut(|f| f.layout_job(bidi::job(text, FontId::proportional(size::SMALL), c)))
    };
    let galleys: Vec<_> = it
        .tags
        .iter()
        .map(|t| {
            let c = tag_color(t);
            (pill(&format!("# {t}"), text_shade(c, v.dark_mode)), c)
        })
        .collect();
    // As many tags as fit on the row; the rest become "+N tags".
    let gap = 6.0;
    let widths: Vec<f32> = galleys.iter().map(|(g, _)| g.size().x + 14.0).collect();
    let more_w = |n: usize| pill(&t!("tags-more", count = n), weak).size().x + 14.0;
    let mut shown = galleys.len();
    while shown > 0 {
        let used: f32 = widths[..shown].iter().sum::<f32>() + gap * shown.saturating_sub(1) as f32;
        let extra = if shown < galleys.len() {
            gap + more_w(galleys.len() - shown)
        } else {
            0.0
        };
        if used + extra <= row.width() {
            break;
        }
        shown -= 1;
    }
    // The row starts on the side its first tag is written from.
    let rtl = it
        .tags
        .first()
        .is_some_and(|t| bidi::starts_rtl(t, i18n::rtl()));
    let mut x = row.left();
    let mut place = |g: Arc<egui::Galley>, c: Color32, fill: Color32| {
        let r = flip_if(
            rtl,
            rect,
            Rect::from_min_size(
                Pos2::new(x, row.center().y - (g.size().y + 6.0) / 2.0),
                g.size() + Vec2::new(14.0, 6.0),
            ),
        );
        tp.rect_filled(r, CornerRadius::same(radius::MEDIUM), fill);
        x += r.width() + gap;
        tp.galley(r.min + Vec2::new(7.0, 3.0), g, c);
    };
    for (g, c) in galleys.iter().take(shown) {
        place(
            g.clone(),
            *c,
            c.gamma_multiply(if v.dark_mode { 0.3 } else { 0.13 }),
        );
    }
    if shown < galleys.len() {
        let rest = galleys.len() - shown;
        place(
            pill(&t!("tags-more", count = rest), weak),
            weak,
            weak.gamma_multiply(0.15),
        );
    }
}

impl App {
    /// Cards in as many columns as fit, like a pinboard.
    pub(super) fn grid(&mut self, ui: &mut Ui) {
        if self.results.is_empty() {
            // Until the first results come, show nothing rather than "no matches".
            if !self.searching {
                self.empty_list(ui);
            }
            return;
        }
        // As wide as the cards, which leave room for the scroll bar.
        let full = ui.available_width();
        self.checklist_above(ui, 0.0, full - 12.0);
        let compact = self.cfg.density == Density::Compact;
        let gap = GRID_GAP;
        let card_h = if compact { 150.0 } else { 210.0 };
        // Leave room for the scroll bar so cards don't jump when it appears.
        let width = ui.available_width() - 12.0;
        let cols = cards_per_row(width);
        let card_w = (width - gap * (cols as f32 - 1.0)) / cols as f32;
        self.grid_cols = cols;
        // Where each card goes from the grid's top left, in the results'
        // order: in rows, or stacked in columns, each card in the shortest
        // one (D69); cards of one height make plain rows. Weaker matches
        // shown with the strong ones start below them, under their header.
        let fit = self.cfg.card_height == CardHeight::Fit;
        let heights: Vec<f32> = self
            .results
            .iter()
            .map(|hit| match self.library.get(hit.id) {
                Some(it) if fit => fitted_height(it, card_w, card_h),
                _ => card_h,
            })
            .collect();
        let weaker_from = self.results.iter().position(|h| h.weak);
        let split = weaker_from.unwrap_or(self.results.len());
        let bottom = |places: &[Rect]| places.iter().map(|p| p.bottom() + gap).fold(0.0, f32::max);
        let mut places = stack(&heights[..split], cols, card_w, gap);
        let strong_h = bottom(&places);
        let below = Vec2::new(0.0, strong_h + WEAKER_HEADER);
        places.extend(
            stack(&heights[split..], cols, card_w, gap)
                .into_iter()
                .map(|p| p.translate(below)),
        );
        // Plain rows step by index; otherwise the arrow keys go by place.
        self.grid_places = if fit || weaker_from.is_some() {
            self.results
                .iter()
                .map(|h| h.id)
                .zip(places.iter().copied())
                .collect()
        } else {
            Vec::new()
        };
        let grid_h = bottom(&places);
        let weaker = self.weaker.len();
        let fold_h = if weaker > 0 { WEAKER_LINE } else { 0.0 };
        let total = grid_h + fold_h;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        let (off, view_h) = self.list_view;
        // Until the grid has been drawn once, its height is unknown: the
        // scroll waits for that frame.
        if self.scroll_to_selected && view_h > 0.0 {
            self.scroll_to_selected = false;
            if let Some(i) = self
                .selected
                .and_then(|id| self.results.iter().position(|h| h.id == id))
            {
                let place = places[i];
                if place.top() < off {
                    sa = sa.vertical_scroll_offset(place.top());
                } else if place.bottom() > off + view_h {
                    sa = sa.vertical_scroll_offset(place.bottom() + gap - view_h);
                }
            }
        }

        let searching = !self.query.trim().is_empty();
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
        // A right-click, or Shift+F10 on the selected card, opens a menu.
        let mut open_menu = None;
        let menu_key = self.menu_by_key;
        // After a delete the cards after it glide into the gap.
        let moving = ui.ctx().style_of(ui.ctx().theme()).animation_time > 0.0;
        let glide = if moving && self.glide_until.is_some_and(|t| Instant::now() < t) {
            GLIDE_TIME
        } else {
            0.0
        };
        // The hover buttons' tips name their keys.
        let keys = self.cfg.keys.clone();
        // While an item's menu is open, the cards under it stay calm.
        let menu_open = self.item_menu.as_ref().is_some_and(|m| !m.in_popup);
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
            for (hit, place) in results.iter().zip(&places) {
                if place.bottom() < viewport.min.y || place.top() > viewport.max.y {
                    continue;
                }
                // Cards fill from the reading-start side.
                let x = if i18n::rtl() {
                    (cols as f32 - 1.0) * (card_w + gap) - place.left()
                } else {
                    place.left()
                };
                let ctx = ui.ctx().clone();
                let to = |axis: &str, value: f32| {
                    ctx.animate_value_with_time(
                        egui::Id::unique(("card", axis, hit.id)),
                        value,
                        glide,
                    )
                };
                let rect = Rect::from_min_size(
                    Pos2::new(origin.x + to("x", x), origin.y + to("y", place.top()) + 2.0),
                    place.size(),
                );
                let Some(it) = library.get(hit.id) else {
                    continue;
                };
                let thumb = thumb_for(it, thumbs, loader, (&mut *seen, frame));
                let resp = ui.interact(rect, egui::Id::unique(("card", it.id)), Sense::CLICK);
                let chosen = Some(it.id) == selected;
                // Selected alone or together with others.
                let lit = chosen || marked.contains(&it.id);
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, lit, item_name(it))
                });
                let hot = !menu_open && ui.rect_contains_pointer(rect);
                let program = it.program.as_deref().and_then(|p| programs.get(p));
                let relevance = relevance_level(hit);
                paint_card(ui, rect, hot, it, thumb, program, relevance, lit, &terms);
                if resp.clicked() {
                    // Ctrl and Shift select several.
                    let keys = ui.input(|i| i.modifiers);
                    actions.push(match (keys.command, keys.shift) {
                        (true, _) => Command::ToggleMark(it.id),
                        (_, true) => Command::MarkRange(it.id),
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
                    let corner = Pos2::new(rect.left() + 16.0, rect.bottom() - 8.0);
                    let at = flip(rect, Rect::from_min_size(corner, Vec2::ZERO)).min;
                    open_menu = Some((it.id, at, true));
                }
                if hot {
                    let size = 30.0;
                    let y = rect.bottom() - CARD_FOOTER / 2.0 - size / 2.0;
                    let at = |x: f32| {
                        flip(
                            rect,
                            Rect::from_min_size(Pos2::new(x, y), Vec2::splat(size)),
                        )
                    };
                    let pin = if it.pinned {
                        (Icon::PinFilled, hint(&t!("item-unpin"), &keys.pin))
                    } else {
                        (Icon::Pin, hint(&t!("item-pin"), &keys.pin))
                    };
                    for (b, icon, tip, action) in [
                        (
                            at(rect.left() + 8.0),
                            Icon::Delete,
                            hint(&t!("item-delete"), &keys.delete),
                            Command::Delete(it.id),
                        ),
                        (
                            at(rect.right() - 8.0 - 3.0 * size - 8.0),
                            pin.0,
                            pin.1,
                            Command::Pin(it.id),
                        ),
                        (
                            at(rect.right() - 8.0 - 2.0 * size - 4.0),
                            Icon::Copy,
                            hint(&t!("item-copy"), &keys.copy_only),
                            Command::Copy(it.id, Paste::No),
                        ),
                        (
                            at(rect.right() - 8.0 - size),
                            Icon::Open,
                            t!("item-open-details"),
                            Command::Open(it.id),
                        ),
                    ] {
                        if icon_button_at(
                            ui,
                            b,
                            egui::Id::unique(("card-action", it.id, &tip)),
                            icon,
                            &tip,
                        )
                        .clicked()
                        {
                            actions.push(action);
                        }
                    }
                }
            }
            if let Some(from) = weaker_from.filter(|_| strong_h <= viewport.max.y) {
                let rect = Rect::from_min_size(
                    Pos2::new(origin.x, origin.y + strong_h),
                    Vec2::new(width, WEAKER_HEADER),
                );
                paint_header(ui, rect, Group::Weaker, results.len() - from);
            }
            let line_top = grid_h;
            if weaker > 0 && line_top <= viewport.max.y {
                let rect = Rect::from_min_size(
                    Pos2::new(origin.x, origin.y + line_top),
                    Vec2::new(width, fold_h),
                );
                if weaker_line(ui, rect, weaker) {
                    actions.push(Command::ShowWeaker);
                }
            }
        });
        self.list_view = (out.state.clamped_offset().y, out.inner_rect.height());
        self.apply_actions(actions);
        self.after_rows(open_menu, false);
        self.item_menu(ui, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_stack_in_the_shortest_column() {
        let places = stack(&[100.0, 50.0, 80.0, 30.0], 2, 200.0, 10.0);
        let at = |i: usize| (places[i].left(), places[i].top());
        assert_eq!(at(0), (0.0, 0.0));
        assert_eq!(at(1), (210.0, 0.0));
        // The second column ends at 60, the first at 110.
        assert_eq!(at(2), (210.0, 60.0));
        assert_eq!(at(3), (0.0, 110.0));
    }

    #[test]
    fn a_card_fits_its_content_within_limits() {
        let short = Item::new(1, Kind::Text, Origin::Clipboard, "Lunch at noon".into());
        let long = Item::new(2, Kind::Text, Origin::Clipboard, "word ".repeat(400).into());
        let same = 210.0;
        let (short, long) = (
            fitted_height(&short, 370.0, same),
            fitted_height(&long, 370.0, same),
        );
        assert!(short < same, "{short}");
        assert!(short >= same * 0.5, "{short}");
        assert_eq!(long, same * 1.6);
    }
}
