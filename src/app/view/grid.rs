//! The grid view and the cards it paints.

use super::*;

/// Height of the action strip a card shows while hovered.
pub(super) const CARD_FOOTER: f32 = 42.0;

/// Height of a card's tags row, above the action strip.
pub(super) const TAG_ROW: f32 = 28.0;

/// A grid card: kind and age on top, as much of the content as fits below,
/// and delete / pin / copy / open buttons in a footer strip while hovered.
pub(super) fn paint_card(
    ui: &Ui,
    rect: Rect,
    hot: bool,
    it: &Item,
    thumb: Option<TextureHandle>,
    selected: bool,
    terms: &[String],
) {
    let v = ui.visuals();
    let a = accent();
    let radius = CornerRadius::same(10);
    let p = ui.painter();
    let m = |r: Rect| flip(rect, r);
    let rtl = i18n::rtl();
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

    // Header: kind chip at the reading start, pin and age at the far end.
    let pad = 16.0;
    let c = kind_color(it.kind);
    let shade = text_shade(c, v.dark_mode);
    let cg = ui.fonts_mut(|f| {
        f.layout_job(bidi::job(
            &it.kind.label(),
            FontId::proportional(12.5),
            shade,
        ))
    });
    let chip_rect = m(Rect::from_min_size(
        Pos2::new(rect.left() + pad, rect.top() + 14.0),
        Vec2::new(cg.size().x + 12.0 + 6.0 + 16.0, cg.size().y + 6.0),
    ));
    p.rect_filled(
        chip_rect,
        CornerRadius::same(7),
        c.gamma_multiply(if v.dark_mode { 0.25 } else { 0.13 }),
    );
    icons::paint_icon_text(
        p,
        chip_rect.shrink2(Vec2::new(8.0, 3.0)),
        icons::kind(it.kind),
        12.0,
        cg,
        shade,
    );
    let weak = v.weak_text_color();
    let age_at = m(Rect::from_min_size(
        Pos2::new(rect.right() - pad, chip_rect.center().y),
        Vec2::ZERO,
    ))
    .min;
    let age = paint_text(
        ui,
        p,
        age_at,
        if rtl {
            Align2::LEFT_CENTER
        } else {
            Align2::RIGHT_CENTER
        },
        &ago(it.created),
        FontId::proportional(12.0),
        weak,
    );
    if it.pinned {
        let x = if rtl {
            age.right() + 12.0
        } else {
            age.left() - 12.0
        };
        icons::paint(
            p,
            Rect::from_center_size(Pos2::new(x, chip_rect.center().y), Vec2::splat(12.0)),
            Icon::PinFilled,
            a,
        );
    }

    // Bottom up: the button strip (empty until hovered), the tags row above it
    // when there are tags, then the content. Hovering only fills the strip,
    // so nothing else moves, shrinks or hides.
    let strip_top = rect.bottom() - CARD_FOOTER;
    let tags_top = if it.tags.is_empty() {
        strip_top
    } else {
        strip_top - TAG_ROW
    };
    if !it.tags.is_empty() {
        let row = Rect::from_min_max(
            Pos2::new(rect.left() + pad, tags_top),
            Pos2::new(rect.right() - pad, strip_top - 2.0),
        );
        let tp = ui.painter_at(row);
        let pill = |text: &str, c: Color32| {
            ui.fonts_mut(|f| f.layout_job(bidi::job(text, FontId::proportional(11.5), c)))
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
            let used: f32 =
                widths[..shown].iter().sum::<f32>() + gap * shown.saturating_sub(1) as f32;
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
    let bottom = tags_top - 4.0;
    let body = Rect::from_min_max(
        Pos2::new(rect.left() + pad, chip_rect.bottom() + 12.0),
        Pos2::new(rect.right() - pad, bottom),
    );
    let bp = ui.painter_at(body);
    match (it.kind, thumb) {
        (Kind::Image, Some(t)) => {
            let s = t.size_vec2();
            let k = (body.width() / s.x).min(body.height() / s.y).min(2.0);
            let img = flip(body, Rect::from_min_size(body.min, s * k));
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
            let at = flip(body, Rect::from_min_size(body.left_top(), Vec2::ZERO)).min;
            bp.text(
                at,
                if rtl {
                    Align2::RIGHT_TOP
                } else {
                    Align2::LEFT_TOP
                },
                format!("{} × {}", it.width, it.height),
                FontId::proportional(13.0),
                weak,
            );
        }
        _ => {
            let code = it.kind == Kind::Text
                && looks_like_code(&it.text.chars().take(600).collect::<String>());
            let (font, line_h) = if code {
                (FontId::monospace(12.5), 17.0)
            } else {
                (FontId::proportional(14.5), 20.0)
            };
            let color = if it.kind == Kind::Link {
                v.hyperlink_color
            } else {
                v.strong_text_color()
            };
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
                bidi::paint(&bp, body, lines, color, None);
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

    if hot {
        let footer = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - CARD_FOOTER),
            rect.max,
        );
        let r = CornerRadius {
            nw: 0,
            ne: 0,
            sw: 10,
            se: 10,
        };
        p.rect_filled(
            footer,
            r,
            if v.dark_mode {
                Color32::from_gray(44)
            } else {
                Color32::from_gray(238)
            },
        );
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
        let total = rows as f32 * pitch;

        let mut sa = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
            if let Some(i) = self
                .selected
                .and_then(|id| self.results.iter().position(|h| h.id == id))
            {
                let (off, view_h) = self.list_view;
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
                paint_card(ui, rect, hot, it, thumb, Some(it.id) == selected, &terms);
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
        });
        self.list_view = (out.state.clamped_offset().y, out.inner_rect.height());
        self.apply_actions(actions);
    }
}
