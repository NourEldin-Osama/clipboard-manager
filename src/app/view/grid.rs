//! The grid view and the cards it paints.

use super::*;
use crate::app::programs::Program;

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
    let radius = CornerRadius::same(10);
    let p = ui.painter();
    let shadow = egui::epaint::Shadow {
        offset: [0, 2],
        blur: if hot { 14 } else { 8 },
        spread: 0,
        color: Color32::from_black_alpha(match (v.dark_mode, hot) {
            (true, _) => 90,
            (false, true) => 34,
            (false, false) => 20,
        }),
    };
    p.add(shadow.as_shape(rect, radius));
    p.rect_filled(rect, radius, v.extreme_bg_color);
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
            sw: 10,
            se: 10,
        };
        let fill = if v.dark_mode {
            Color32::from_gray(44)
        } else {
            Color32::from_gray(238)
        };
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
                .corner_radius(6.0)
                .paint_at(ui, img);
            bp.rect_stroke(
                img,
                CornerRadius::same(6),
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
            bp.rect_filled(swatch, CornerRadius::same(8), color);
            bp.rect_stroke(
                swatch,
                CornerRadius::same(8),
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
            paint_lines(ui, &bp, body, below, &lines);
        }
        _ if path.is_some() => {
            let path = path.expect("checked above");
            let c = kind_color(Kind::Files);
            let icon = match path.extension() {
                Some(_) => Icon::File,
                None => Icon::Folder,
            };
            let at = flip(body, Rect::from_min_size(body.min, Vec2::splat(28.0)));
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
            paint_lines(ui, &bp, body, below, &lines);
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
            let (font, line_h) = if code {
                (FontId::monospace(size::SMALL), 17.0)
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
                    a.gamma_multiply(0.28),
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
                        v.strong_text_color(),
                        body.width(),
                        rows,
                    ))
                });
                bp.galley(body.min, g, color);
            }
        }
    }
    // Outline last, so the footer doesn't cover it.
    if selected {
        p.rect_stroke(rect, radius, Stroke::new(2.0, a), egui::StrokeKind::Inside);
    } else if hot {
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, weak.gamma_multiply(0.35)),
            egui::StrokeKind::Inside,
        );
    }
}

/// Lines of text from `at` down, each on the side reading starts.
fn paint_lines(ui: &Ui, p: &egui::Painter, body: Rect, at: Pos2, lines: &[(String, f32, Color32)]) {
    let mut y = at.y;
    for (text, text_size, color) in lines {
        let start = flip(
            body,
            Rect::from_min_size(Pos2::new(body.left(), y), Vec2::ZERO),
        )
        .min;
        let painted = paint_text(
            ui,
            p,
            start,
            start_anchor(egui::Align::Min),
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
        let x = if i18n::rtl() {
            age.right() + 12.0
        } else {
            age.left() - 12.0
        };
        icons::paint(
            &p,
            Rect::from_center_size(Pos2::new(x, y), Vec2::splat(12.0)),
            Icon::PinFilled,
            accent(),
        );
        used += 20.0;
    }
    if let Some(level) = relevance {
        paint_relevance(&p, row, row.right() - used, y + 5.0, level, accent(), weak);
        used += 26.0;
    }
    let room = row.width() - used;
    // Then the program at the reading start: its icon and its own name, or
    // for older copies the kind's icon and the program's file name.
    let icon_rect = flip(
        row,
        Rect::from_center_size(Pos2::new(row.left() + 8.0, y), Vec2::splat(16.0)),
    );
    match program.and_then(|p| p.icon.as_ref()) {
        Some(texture) => {
            egui::Image::new((texture.id(), Vec2::splat(16.0))).paint_at(ui, icon_rect);
        }
        None => icons::paint(&p, icon_rect, item_icon(it), weak),
    }
    let name = match (program, &it.app) {
        (Some(program), _) => program.name.clone(),
        (None, Some(app)) => app.clone(),
        (None, None) => it.kind.label(),
    };
    let start = flip(
        row,
        Rect::from_min_size(Pos2::new(row.left() + 22.0, y), Vec2::ZERO),
    )
    .min;
    let font = FontId::proportional(size::SMALL);
    let full = ui.fonts_mut(|f| f.layout_no_wrap(name.clone(), font.clone(), weak));
    let shown = if full.size().x > room - 22.0 {
        let mut cut = name.clone();
        while !cut.is_empty() {
            cut.pop();
            let candidate = format!("{}…", cut.trim_end());
            if ui
                .fonts_mut(|f| f.layout_no_wrap(candidate.clone(), font.clone(), weak))
                .size()
                .x
                <= room - 22.0
            {
                break;
            }
        }
        format!("{}…", cut.trim_end())
    } else {
        name
    };
    paint_text(
        ui,
        &p,
        start,
        start_anchor(egui::Align::Center),
        &shown,
        font,
        weak,
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
    let mut x = row.left();
    let mut place = |g: Arc<egui::Galley>, c: Color32, fill: Color32| {
        let r = flip(
            rect,
            Rect::from_min_size(
                Pos2::new(x, row.center().y - (g.size().y + 6.0) / 2.0),
                g.size() + Vec2::new(14.0, 6.0),
            ),
        );
        tp.rect_filled(r, CornerRadius::same(9), fill);
        x += r.width() + gap;
        tp.galley(r.min + Vec2::new(7.0, 3.0), g, c);
    };
    for (g, c) in galleys.iter().take(shown) {
        place(
            g.clone(),
            *c,
            c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }),
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
        let compact = self.cfg.density == Density::Compact;
        let gap = 14.0;
        let card_h = if compact { 150.0 } else { 210.0 };
        // Leave room for the scroll bar so cards don't jump when it appears.
        let width = ui.available_width() - 12.0;
        let cols = ((width + gap) / (240.0 + gap)).floor().max(1.0) as usize;
        let card_w = (width - gap * (cols as f32 - 1.0)) / cols as f32;
        self.grid_cols = cols;
        let rows = self.results.len().div_ceil(cols);
        let pitch = card_h + gap;
        let weaker = self.weaker.len();
        let fold_h = if weaker > 0 { WEAKER_LINE } else { 0.0 };
        let total = rows as f32 * pitch + fold_h;

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
                let top = (i / cols) as f32 * pitch;
                if top < off {
                    sa = sa.vertical_scroll_offset(top);
                } else if top + card_h > off + view_h {
                    sa = sa.vertical_scroll_offset(top + card_h + gap - view_h);
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
        let frame = ui.ctx().cumulative_pass_nr();
        let (library, results, thumbs, loader, selected, seen, programs) = (
            &self.library,
            &self.results,
            &self.thumbs,
            &mut self.loader,
            self.selected,
            &mut self.thumb_seen,
            &mut self.programs,
        );
        let out = sa.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let first = ((viewport.min.y / pitch).floor().max(0.0) as usize) * cols;
            let last = (((viewport.max.y / pitch).ceil() as usize + 1) * cols).min(results.len());
            for (r, hit) in results.iter().enumerate().take(last).skip(first) {
                let (row, col) = (r / cols, r % cols);
                // Cards fill from the reading-start side.
                let col = if i18n::rtl() { cols - 1 - col } else { col };
                let rect = Rect::from_min_size(
                    Pos2::new(
                        origin.x + col as f32 * (card_w + gap),
                        origin.y + row as f32 * pitch + 2.0,
                    ),
                    Vec2::new(card_w, card_h),
                );
                let Some(it) = library.get(hit.id) else {
                    continue;
                };
                let thumb = thumb_for(it, thumbs, loader, (&mut *seen, frame));
                let resp = ui.interact(rect, egui::Id::unique(("card", it.id)), Sense::CLICK);
                let chosen = Some(it.id) == selected;
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::Role::ListBoxOption,
                        true,
                        chosen,
                        item_name(it),
                    )
                });
                let hot = ui.rect_contains_pointer(rect);
                let program = it.program.as_deref().and_then(|p| programs.get(p));
                let chosen_card = Some(it.id) == selected;
                let relevance = relevance_level(hit);
                paint_card(
                    ui,
                    rect,
                    hot,
                    it,
                    thumb,
                    program,
                    relevance,
                    chosen_card,
                    &terms,
                );
                if resp.clicked() {
                    actions.push(Command::Select(it.id));
                }
                if resp.double_clicked() {
                    actions.push(Command::Copy(it.id, Paste::Yes));
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
                        (Icon::PinFilled, t!("item-unpin"))
                    } else {
                        (Icon::Pin, t!("item-pin"))
                    };
                    for (b, icon, tip, action) in [
                        (
                            at(rect.left() + 8.0),
                            Icon::Delete,
                            t!("item-delete"),
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
                            t!("item-copy"),
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
            let line_top = rows as f32 * pitch;
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
    }
}
