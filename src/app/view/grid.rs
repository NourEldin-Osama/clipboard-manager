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

/// How tall a card is when cards are as tall as their content (D69): its
/// content as `paint_card` lays it out in a card `card_w` wide, with the
/// tags row when it has tags and the footer. A picture's card is at least
/// as tall as a card with one line of text; no card is taller than 1.6
/// times the height cards have when they are all the same (`same`), and
/// longer text is cut there.
fn fitted_height(ui: &Ui, it: &Item, card_w: f32, same: f32) -> f32 {
    let probe = Rect::from_min_size(Pos2::ZERO, Vec2::new(card_w, same * 1.6));
    let most = card_body(probe, !it.tags.is_empty());
    let around = probe.height() - most.height();
    let body = if it.kind == Kind::Image {
        let picture = if it.width > 0 && it.height > 0 {
            (most.width() * it.height as f32 / it.width as f32).min(it.height as f32 * 2.0)
        } else {
            80.0
        };
        picture.max(line_height(ui, &FontId::proportional(size::BODY)))
    } else if it.kind == Kind::Text && color_in(&it.text).is_some() {
        // The swatch, then its two lines.
        110.0
    } else if it.kind == Kind::Text && path_in(&it.text).is_some() {
        // The icon, then the name and the folder, as `paint_lines` puts them.
        PATH_ICON
            + PATH_ICON_GAP
            + line_height(ui, &FontId::proportional(size::LARGE))
            + LINE_GAP
            + line_height(ui, &FontId::proportional(size::SMALL))
    } else {
        let (lines, _) = card_text(ui, it, &[], most.size());
        lines.iter().map(|(g, _)| g.size().y).sum()
    };
    around + body.min(most.height())
}

/// Room around a card's content, and beside its tags and footer.
const CARD_PADDING: f32 = 16.0;

/// Between a card's content and its tags row or footer.
const BELOW_CONTENT: f32 = 4.0;

/// The icon of a card that shows a path, and the room below it.
const PATH_ICON: f32 = 28.0;
const PATH_ICON_GAP: f32 = 10.0;

/// Between the lines of a color's or a path's card.
const LINE_GAP: f32 = 2.0;

/// Where a card's content goes in `rect`: inside its padding, above its
/// tags row when it is `tagged`, and above its footer.
fn card_body(rect: Rect, tagged: bool) -> Rect {
    let tags = if tagged { TAG_ROW } else { 0.0 };
    Rect::from_min_max(
        rect.min + Vec2::splat(CARD_PADDING),
        Pos2::new(
            rect.right() - CARD_PADDING,
            rect.bottom() - CARD_FOOTER - tags - BELOW_CONTENT,
        ),
    )
}

/// The height of one line of text in `font`, as egui lays it out: on whole
/// pixels, so at the screen's scale.
fn line_height(ui: &Ui, font: &FontId) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(String::new(), font.clone(), Color32::WHITE))
        .size()
        .y
}

/// How many lines of `font` fit in `height`, at least one.
fn lines_that_fit(ui: &Ui, font: &FontId, height: f32) -> usize {
    // egui puts lines on whole pixels but sizes text in steps of 1/32 of a
    // point, so the height of some lines is not quite that many times one
    // line's: half a point over counts every line of a height laid out
    // for them. A line can then go past `height` by that much, which the
    // room below a card's text takes.
    ((height + 0.5) / line_height(ui, font)).floor().max(1.0) as usize
}

/// The text of a card that shows text, laid out as the card shows it in
/// `size`: as many lines as fit, the last cut with "…" when there are
/// more. Text with Arabic comes line by line, each with whether it reads
/// right to left; other text is one galley. With the text's color.
fn card_text(
    ui: &Ui,
    it: &Item,
    terms: &[String],
    size: Vec2,
) -> (Vec<(Arc<egui::Galley>, bool)>, Color32) {
    let v = ui.visuals();
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
    let font = if code {
        FontId::monospace(size::CONTROL)
    } else if short {
        FontId::proportional(size::LARGE)
    } else {
        FontId::proportional(size::BODY)
    };
    let color = if it.kind == Kind::Link {
        v.hyperlink_color
    } else {
        v.strong_text_color()
    };
    let rows = lines_that_fit(ui, &font, size.y);
    let hit = theme::search_hit(v.dark_mode);
    let lines = if bidi::has_rtl(&text) {
        // Each line lines up with its own reading edge.
        bidi::lines(
            ui,
            text.trim_end(),
            terms,
            font,
            color,
            hit,
            size.x,
            rows,
            None,
        )
    } else {
        let job = highlighted(
            text.trim_end(),
            terms,
            font,
            color,
            (v.strong_text_color(), hit),
            size.x,
            rows,
        );
        vec![(ui.fonts_mut(|f| f.layout_job(job)), false)]
    };
    (lines, color)
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
    let pad = CARD_PADDING;
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
    let body = card_body(rect, !it.tags.is_empty());
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
            let at = flip_if(
                rtl,
                body,
                Rect::from_min_size(body.min, Vec2::splat(PATH_ICON)),
            );
            icons::paint(&bp, at, icon, text_shade(c, v.dark_mode));
            let name = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            let folder = path
                .parent()
                .map(|f| f.display().to_string())
                .unwrap_or_default();
            let below = Pos2::new(body.left(), at.bottom() + PATH_ICON_GAP);
            let lines = [
                (name, size::LARGE, v.strong_text_color()),
                (folder, size::SMALL, weak),
            ];
            paint_lines(ui, &bp, body, below, &lines, rtl);
        }
        _ => {
            let (lines, color) = card_text(ui, it, terms, body.size());
            // Arabic letters and their marks reach above and below their
            // lines, and a card as tall as its text ends at its last line:
            // the room around the text keeps them whole.
            let clip = Rect::from_min_max(
                body.min - Vec2::new(0.0, CARD_PADDING / 2.0),
                body.max + Vec2::new(0.0, BELOW_CONTENT),
            );
            bidi::paint(&ui.painter_at(clip), body, lines, color, None, 0.0);
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
        y = painted.bottom() + LINE_GAP;
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

/// How long a frame may spend measuring cards (see `CardHeights`).
const MEASURE_TIME: Duration = Duration::from_millis(4);

/// What the fitted heights of cards depend on besides the items: the
/// cards' width, the density, the language and the screen's scale.
#[derive(Clone, Copy, PartialEq)]
struct CardLayout {
    width: f32,
    compact: bool,
    rtl: bool,
    pixels_per_point: f32,
}

/// The fitted heights of cards (D69), kept so that each card is measured
/// once for each layout of the grid: a frame with nothing new measures
/// nothing. A new layout, such as each new width while the window is
/// resized, measures the cards again in the results' order for as long as
/// a frame may (`MEASURE_TIME`); until its turn, a card keeps the height
/// it had in an earlier layout. Heights are kept for one item each, and
/// those of items no longer in the history are dropped.
#[derive(Default)]
pub(crate) struct CardHeights {
    layout: Option<CardLayout>,
    /// Goes up with each new layout, so a height knows whether it is up to
    /// date.
    generation: u64,
    by_item: HashMap<u64, Measured>,
}

struct Measured {
    generation: u64,
    /// The item had tags, which add their row.
    tagged: bool,
    height: f32,
}

impl CardHeights {
    /// Starts a frame with cards laid out as `layout`.
    fn start(&mut self, layout: CardLayout) {
        if self.layout != Some(layout) {
            self.layout = Some(layout);
            self.generation += 1;
        }
    }

    /// The height of `it`'s card, and whether it is up to date: as
    /// measured for this layout; or measured now by `measure` before
    /// `deadline`; or else, until a later frame, its height in an earlier
    /// layout, or `stand_in` when it has none.
    fn height(
        &mut self,
        it: &Item,
        deadline: Instant,
        stand_in: f32,
        measure: impl FnOnce() -> f32,
    ) -> (f32, bool) {
        let tagged = !it.tags.is_empty();
        let known = self
            .by_item
            .get(&it.id)
            .filter(|m| m.tagged == tagged)
            .map(|m| (m.generation, m.height));
        if let Some((generation, height)) = known {
            if generation == self.generation {
                return (height, true);
            }
        }
        if Instant::now() >= deadline {
            return (known.map_or(stand_in, |(_, height)| height), false);
        }
        let height = measure();
        let generation = self.generation;
        self.by_item.insert(
            it.id,
            Measured {
                generation,
                tagged,
                height,
            },
        );
        (height, true)
    }

    /// Drops the heights of items no longer in `library`, once there are
    /// more heights than items.
    fn forget_gone(&mut self, library: &Library) {
        if self.by_item.len() > library.items().len() {
            self.by_item.retain(|id, _| library.get(*id).is_some());
        }
    }
}

impl App {
    /// The height of each result's card when cards are as tall as their
    /// content, `card_w` wide, `same` tall when they are all the same.
    fn fitted_heights(&mut self, ui: &Ui, card_w: f32, same: f32) -> Vec<f32> {
        self.card_heights.start(CardLayout {
            width: card_w,
            compact: self.cfg.density == Density::Compact,
            rtl: i18n::rtl(),
            pixels_per_point: ui.pixels_per_point(),
        });
        let deadline = Instant::now() + MEASURE_TIME;
        let mut behind = false;
        let heights = self
            .results
            .iter()
            .map(|hit| match self.library.get(hit.id) {
                Some(it) => {
                    let measure = || fitted_height(ui, it, card_w, same);
                    let (height, current) = self.card_heights.height(it, deadline, same, measure);
                    behind |= !current;
                    height
                }
                None => same,
            })
            .collect();
        self.card_heights.forget_gone(&self.library);
        // The cards not measured in time are measured in the next frames.
        if behind {
            ui.ctx().request_repaint();
        }
        heights
    }

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
        // one (D69); cards of one height make plain rows. The link to the
        // weaker matches follows the strong ones, and while it is open the
        // weaker ones start below it.
        let fit = self.cfg.card_height == CardHeight::Fit;
        let heights: Vec<f32> = if fit {
            self.fitted_heights(ui, card_w, card_h)
        } else {
            vec![card_h; self.results.len()]
        };
        let link = self.weaker_link_place();
        let split = link
            .and_then(|link| link.first_weak)
            .unwrap_or(self.results.len());
        let bottom = |places: &[Rect]| places.iter().map(|p| p.bottom() + gap).fold(0.0, f32::max);
        let mut places = stack(&heights[..split], cols, card_w, gap);
        let strong_h = bottom(&places);
        let link_h = if link.is_some() { WEAKER_LINE } else { 0.0 };
        let below = Vec2::new(0.0, strong_h + link_h);
        places.extend(
            stack(&heights[split..], cols, card_w, gap)
                .into_iter()
                .map(|p| p.translate(below)),
        );
        // Plain rows step by index; otherwise the arrow keys go by place.
        self.grid_places = if fit || split < self.results.len() {
            self.results
                .iter()
                .map(|h| h.id)
                .zip(places.iter().copied())
                .collect()
        } else {
            Vec::new()
        };
        let total = bottom(&places).max(strong_h + link_h);

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
            let link_shown = strong_h <= viewport.max.y && strong_h + link_h >= viewport.min.y;
            if let Some(link) = link.filter(|_| link_shown) {
                let rect = Rect::from_min_size(
                    Pos2::new(origin.x, origin.y + strong_h),
                    Vec2::new(width, link_h),
                );
                actions.extend(link.clicked(ui, rect));
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

    /// Runs `check` in a frame of a window with the app's fonts, at
    /// `scale` pixels per point.
    fn in_window(scale: f32, mut check: impl FnMut(&Ui)) {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(scale);
        add_fonts(&ctx);
        // The fonts apply from the next frame; the font atlas uploads no
        // renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx.run_ui(egui::RawInput::default(), |ui| check(ui))
            .textures_delta
            .clear();
        assert_eq!(ctx.pixels_per_point(), scale);
    }

    /// How many lines laid-out text has.
    fn rows(lines: &[(Arc<egui::Galley>, bool)]) -> usize {
        lines.iter().map(|(g, _)| g.rows.len()).sum()
    }

    /// A card as tall as its content shows every line the cap allows, with
    /// no room left over, and a one-line card is as tall as one line needs.
    #[test]
    fn a_fitted_card_holds_every_line_the_cap_allows() {
        let text = |id: u64, text: &str| Item::new(id, Kind::Text, Origin::Clipboard, text.into());
        let long = "word ".repeat(400);
        let samples = [
            text(1, "Lunch at noon"),
            text(2, "Call the bank\nPick up the parcel\nBook the flights"),
            text(3, "fn main() {\n    println!(\"Hello, world!\");\n}"),
            text(
                4,
                "Dear team, the new version is out. Please update and tell me what you think.",
            ),
            text(
                5,
                "SELECT name, total FROM orders WHERE total > 1000 ORDER BY total DESC;",
            ),
            text(6, "موعد الاجتماع يوم الخميس الساعة العاشرة والنصف صباحًا"),
            text(7, &long),
        ];
        let tagged: Vec<Item> = samples
            .iter()
            .map(|it| {
                let mut it = it.clone();
                it.tags = vec!["work".into()];
                it
            })
            .collect();
        for scale in [1.0, 1.25, 1.5] {
            in_window(scale, |ui| {
                for (it, width, same) in samples.iter().chain(&tagged).flat_map(|it| {
                    [240.0, 300.0, 370.0, 494.0]
                        .into_iter()
                        .flat_map(move |width| [150.0, 210.0].map(|same| (it, width, same)))
                }) {
                    let case = format!("{:?} at {width} by {same}, scale {scale}", it.text);
                    let tagged = !it.tags.is_empty();
                    let height = fitted_height(ui, it, width, same);
                    assert!(height <= same * 1.6, "{case}: {height}");
                    let card = Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height));
                    let body = card_body(card, tagged);
                    // As the card draws it, all of it, and at the cap.
                    let (shown, _) = card_text(ui, it, &[], body.size());
                    let every = Vec2::new(body.width(), f32::INFINITY);
                    let (all, _) = card_text(ui, it, &[], every);
                    let tallest = Rect::from_min_size(Pos2::ZERO, Vec2::new(width, same * 1.6));
                    let (capped, _) = card_text(ui, it, &[], card_body(tallest, tagged).size());
                    assert_eq!(rows(&shown), rows(&capped), "{case}");
                    if *it.text == *long {
                        assert!(rows(&shown) < rows(&all), "{case}: the long text is cut");
                        assert!(shown[0].0.elided, "{case}: it ends with the cut-off mark");
                    } else {
                        assert_eq!(rows(&shown), rows(&all), "{case}: a line was lost");
                    }
                    // Exactly as tall as its lines.
                    let lines_h: f32 = shown.iter().map(|(g, _)| g.size().y).sum();
                    assert!((lines_h - body.height()).abs() < 0.01, "{case}: {lines_h}");
                }
                let one_line = fitted_height(ui, &samples[0], 370.0, 210.0);
                let line = line_height(ui, &FontId::proportional(size::LARGE));
                let expected = CARD_PADDING + line + BELOW_CONTENT + CARD_FOOTER;
                assert!((one_line - expected).abs() < 0.01, "{one_line} {expected}");
            });
        }
    }

    #[test]
    fn a_card_is_measured_once_for_each_layout() {
        let mut heights = CardHeights::default();
        let mut it = Item::new(1, Kind::Text, Origin::Clipboard, "Lunch at noon".into());
        let layout = |width: f32| CardLayout {
            width,
            compact: false,
            rtl: false,
            pixels_per_point: 1.0,
        };
        let later = Instant::now() + Duration::from_secs(600);
        heights.start(layout(300.0));
        assert_eq!(heights.height(&it, later, 210.0, || 84.0), (84.0, true));
        // The next frame with nothing new measures nothing.
        heights.start(layout(300.0));
        let not_again = || -> f32 { panic!("measured again") };
        assert_eq!(heights.height(&it, later, 210.0, not_again), (84.0, true));
        // Another width measures again; out of time, the height of the
        // earlier width stands in.
        heights.start(layout(280.0));
        let no_time = Instant::now();
        assert_eq!(
            heights.height(&it, no_time, 210.0, not_again),
            (84.0, false)
        );
        assert_eq!(heights.height(&it, later, 210.0, || 106.0), (106.0, true));
        // Tags add their row: measured again, and until then the card is as
        // tall as cards of one height.
        it.tags = vec!["work".into()];
        assert_eq!(
            heights.height(&it, no_time, 210.0, not_again),
            (210.0, false)
        );
        assert_eq!(heights.height(&it, later, 210.0, || 134.0), (134.0, true));
    }
}
