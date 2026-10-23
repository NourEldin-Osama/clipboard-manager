//! Small widgets and the language-aware text and layout helpers (right to
//! left in Arabic) used by the window and by Settings.

use super::*;

pub(super) fn search_id() -> egui::Id {
    egui::Id::unique("search")
}

/// Text for a label or button in display order. `None` keeps the widget's own colour.
pub(crate) fn txt(text: &str, size: f32, color: Option<Color32>) -> egui::WidgetText {
    if bidi::has_rtl(text) {
        bidi::job(
            text,
            FontId::proportional(size),
            color.unwrap_or(Color32::PLACEHOLDER),
        )
        .into()
    } else {
        let r = RichText::new(text).size(size);
        match color {
            Some(c) => r.color(c).into(),
            None => r.into(),
        }
    }
}

/// `text` as a label, in `size` and `color`.
pub(crate) fn lbl(ui: &mut Ui, text: &str, size: f32, color: Color32) -> egui::Response {
    ui.label(txt(text, size, Some(color)))
}

/// A hover tooltip that wraps Arabic in reading order.
pub(crate) fn tip(resp: egui::Response, text: &str) -> egui::Response {
    if bidi::has_rtl(text) {
        let text = text.to_string();
        resp.on_hover_ui(move |ui| {
            ui.set_max_width(340.0);
            let c = ui.visuals().text_color();
            bidi::paragraph(ui, &text, FontId::proportional(13.0), c);
        })
    } else {
        resp.on_hover_text(text)
    }
}

/// Main-axis direction for a row: right to left in Arabic.
pub(crate) fn row_layout() -> Layout {
    if i18n::rtl() {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    }
}

/// The opposite direction, for pushing controls to the far end of a row.
pub(crate) fn far_layout() -> Layout {
    if i18n::rtl() {
        Layout::left_to_right(Align::Center)
    } else {
        Layout::right_to_left(Align::Center)
    }
}

/// Like `ui.horizontal_wrapped`, in reading direction.
pub(crate) fn row_wrapped<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = Vec2::new(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, row_layout().with_main_wrap(true), add)
        .inner
}

/// Like `ui.horizontal`, in reading direction.
pub(crate) fn row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = Vec2::new(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, row_layout(), add).inner
}

/// The rest of the row, filled from its far end.
pub(crate) fn far<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(far_layout(), add).inner
}

/// A label plus an optional smaller count ("Text  5") for pills; the count
/// trails the label in reading order.
pub(crate) fn pill_job(
    text: &str,
    size: f32,
    color: Color32,
    count: Option<(String, Color32)>,
) -> LayoutJob {
    let mut job = if bidi::has_rtl(text) {
        bidi::job(text, FontId::proportional(size), color)
    } else {
        let mut j = LayoutJob::default();
        j.append(
            text,
            0.0,
            TextFormat {
                font_id: FontId::proportional(size),
                color,
                ..Default::default()
            },
        );
        j
    };
    job.halign = egui::Align::LEFT;
    if let Some((n, c)) = count {
        let fmt = TextFormat {
            font_id: FontId::proportional(size - 1.5),
            color: c,
            ..Default::default()
        };
        if i18n::rtl() {
            let mut j = LayoutJob::default();
            j.append(&format!("{n}  "), 0.0, fmt);
            for s in job.sections.iter() {
                j.append(
                    &job.text[s.byte_range.start.0..s.byte_range.end.0],
                    0.0,
                    s.format.clone(),
                );
            }
            job = j;
        } else {
            job.append(&format!("  {n}"), 0.0, fmt);
        }
    }
    job
}

/// Paints one line of text anchored at `pos`, in display order. Returns its rect.
pub(crate) fn paint_text(
    ui: &Ui,
    p: &egui::Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    let g = if bidi::has_rtl(text) {
        ui.fonts_mut(|f| f.layout_job(bidi::job(text, font, color)))
    } else {
        ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, color))
    };
    let rect = anchor.anchor_size(pos, g.size());
    p.galley(rect.min, g, color);
    rect
}

/// The anchor on the reading-start side: left, or right in Arabic.
pub(crate) fn start_anchor(vertical: egui::Align) -> Align2 {
    let h = if i18n::rtl() {
        egui::Align::Max
    } else {
        egui::Align::Min
    };
    Align2([h, vertical])
}

/// Mirrors `r` inside `within` for right-to-left layout.
pub(crate) fn flip(within: Rect, r: Rect) -> Rect {
    if i18n::rtl() {
        Rect::from_min_max(
            Pos2::new(within.left() + within.right() - r.right(), r.top()),
            Pos2::new(within.left() + within.right() - r.left(), r.bottom()),
        )
    } else {
        r
    }
}

/// "Pin (Ctrl+P)", or just "Pin" when the shortcut is off.
pub(super) fn hint(what: &str, keys: &str) -> String {
    if keys.trim().is_empty() {
        what.to_string()
    } else {
        t!("hint-with-keys", action = what, keys = keys)
    }
}

/// A quiet text button with a painted ▾, for menus.
pub(super) fn mode_button(ui: &mut Ui, label: &str, emphasised: bool) -> egui::Response {
    let v = ui.visuals();
    let color = if emphasised {
        accent()
    } else {
        v.weak_text_color()
    };
    let g = ui.fonts_mut(|f| f.layout_job(bidi::job(label, FontId::proportional(13.0), color)));
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(g.size().x + 8.0 + 10.0 + 18.0, 28.0),
        Sense::click(),
    );
    let v = ui.visuals();
    if emphasised {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), accent().gamma_multiply(0.12));
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.weak_bg_fill);
    }
    let color = if resp.hovered() && !emphasised {
        v.text_color()
    } else {
        color
    };
    let text_at = flip(
        rect,
        Rect::from_min_size(
            Pos2::new(rect.left() + 9.0, rect.center().y - g.size().y / 2.0),
            g.size(),
        ),
    )
    .min;
    ui.painter().galley(text_at, g, color);
    icons::paint(
        ui.painter(),
        flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.right() - 14.0, rect.center().y),
                Vec2::splat(10.0),
            ),
        ),
        Icon::Dropdown,
        color,
    );
    resp
}

/// Rounded tinted label: kind, state or status.
pub(crate) fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    let text = text.into();
    Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| ui.label(txt(&text, 12.0, Some(color))))
        .response
}

/// A menu row with a check mark at the far end when `checked`.
pub(super) fn check_row(ui: &mut Ui, text: &str, checked: bool) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, checked, text));
    let v = ui.visuals();
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), v.widgets.hovered.weak_bg_fill);
    }
    let color = if checked {
        accent()
    } else if resp.hovered() {
        v.strong_text_color()
    } else {
        v.text_color()
    };
    let at = flip(
        rect,
        Rect::from_min_size(Pos2::new(rect.left() + 10.0, rect.center().y), Vec2::ZERO),
    )
    .min;
    paint_text(
        ui,
        ui.painter(),
        at,
        start_anchor(egui::Align::Center),
        text,
        FontId::proportional(14.0),
        color,
    );
    if checked {
        let mark = flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.right() - 16.0, rect.center().y),
                Vec2::splat(14.0),
            ),
        );
        icons::paint(ui.painter(), mark, Icon::Check, accent());
    }
    resp
}

/// Width of the small icon menus (add files, more tags).
pub(super) const MENU_W: f32 = 220.0;

/// Wide enough for the longest help line, no wider.
pub(super) const MODE_MENU_W: f32 = 300.0;

/// One search-mode choice: name and help in a single full-width target, the
/// active one tinted, edged and ticked.
pub(super) fn mode_option(ui: &mut Ui, m: SearchMode, active: bool) -> egui::Response {
    let w = MODE_MENU_W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::RadioButton, true, active, mode_label(m))
    });
    let v = ui.visuals();
    let p = ui.painter();
    let a = accent();
    let r = CornerRadius::same(8);
    if active {
        p.rect_filled(
            rect,
            r,
            a.gamma_multiply(if v.dark_mode { 0.22 } else { 0.10 }),
        );
        p.rect_filled(
            flip(
                rect,
                Rect::from_min_size(
                    rect.min + Vec2::new(0.0, 10.0),
                    Vec2::new(3.0, rect.height() - 20.0),
                ),
            ),
            CornerRadius::same(2),
            a,
        );
    }
    if resp.hovered() {
        p.rect_filled(
            rect,
            r,
            if active {
                a.gamma_multiply(0.06)
            } else {
                v.widgets.hovered.weak_bg_fill
            },
        );
    }
    let title = if active { a } else { v.strong_text_color() };
    let at = |y: f32| {
        flip(
            rect,
            Rect::from_min_size(Pos2::new(rect.left() + 14.0, rect.top() + y), Vec2::ZERO),
        )
        .min
    };
    paint_text(
        ui,
        p,
        at(8.0),
        start_anchor(egui::Align::Min),
        &mode_label(m),
        FontId::proportional(14.0),
        title,
    );
    paint_text(
        ui,
        p,
        at(27.0),
        start_anchor(egui::Align::Min),
        &mode_help(m),
        FontId::proportional(11.5),
        v.weak_text_color(),
    );
    if active {
        // Level with the title, clear of the longer help line.
        icons::paint(
            p,
            flip(
                rect,
                Rect::from_center_size(
                    Pos2::new(rect.right() - 18.0, rect.top() + 16.0),
                    Vec2::splat(14.0),
                ),
            ),
            Icon::Check,
            a,
        );
    }
    resp
}

pub(crate) fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding.x = 16.0;
        ui.add(
            egui::Button::new(txt(text, 14.0, Some(Color32::WHITE)))
                .fill(accent())
                .min_size(Vec2::new(0.0, 32.0)),
        )
    })
    .inner
}
