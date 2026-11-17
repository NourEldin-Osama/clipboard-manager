//! Small widgets and the language-aware text and layout helpers (right to
//! left in Arabic) used by the window and by Settings.

use super::*;

pub(super) fn search_id() -> egui::Id {
    egui::Id::unique("search")
}

/// Names a widget for screen readers: a text field, a row or a switch has
/// no text of its own that egui could use.
pub(crate) fn set_name(resp: &egui::Response, name: &str) {
    let name = name.to_string();
    resp.ctx
        .accesskit_node_builder(resp.id, |node| node.set_label(name));
}

/// Names a drop-down menu by its setting, with the chosen option as its
/// value (egui names it by the option alone).
pub(crate) fn set_choice_name(resp: &egui::Response, name: &str, chosen: &str) {
    let (name, chosen) = (name.to_string(), chosen.to_string());
    resp.ctx.accesskit_node_builder(resp.id, |node| {
        node.set_label(name);
        node.set_value(chosen);
    });
}

/// Notes whether the keyboard (Tab) or the mouse moved the focus last.
pub(super) fn keyboard_moved_focus(ctx: &egui::Context, before: bool) -> bool {
    ctx.input(|i| {
        if i.key_pressed(Key::Tab) {
            true
        } else if i.pointer.any_pressed() {
            false
        } else {
            before
        }
    })
}

/// An accent outline around the control with the keyboard focus. egui's
/// own widgets barely show the focus, and the app's drawn ones not at all.
pub(super) fn focus_outline(ctx: &egui::Context) {
    let Some(focused) = ctx
        .memory(|m| m.focused())
        .and_then(|id| ctx.read_response(id))
    else {
        return;
    };
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("focus-outline"),
    ))
    .rect_stroke(
        focused.interact_rect.expand(2.0),
        CornerRadius::same(radius::MEDIUM),
        Stroke::new(2.0, accent()),
        egui::StrokeKind::Outside,
    );
}

/// How often `slow_spinner` moves: 30 times a second looks smooth, and is
/// half the frames of drawing every one.
const SPINNER_STEP: Duration = Duration::from_millis(33);

/// egui's spinner, moving in steps 30 times a second. egui's own asks for
/// a new frame every frame, so the window redrew as fast as it could
/// while, say, a long indexing ran.
pub(crate) fn slow_spinner(ui: &mut Ui, size: f32, color: Option<Color32>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::new(egui::Role::ProgressIndicator));
    if ui.is_rect_visible(rect) {
        let color = color.unwrap_or_else(|| ui.visuals().strong_text_color());
        paint_spinner(ui, rect, color);
    }
    response
}

/// `slow_spinner` in `rect`, for what is placed by hand.
pub(crate) fn paint_spinner(ui: &Ui, rect: Rect, color: Color32) {
    ui.ctx().request_repaint_after(SPINNER_STEP);
    let step = SPINNER_STEP.as_secs_f64();
    let time = (ui.input(|i| i.time) / step).floor() * step;
    let radius = rect.height().min(rect.width()) * 0.5 - 2.0;
    let n_points = (radius.round() as u32).clamp(8, 128);
    let start = time * std::f64::consts::TAU;
    let end = start + 240f64.to_radians() * time.sin();
    let points = (0..n_points)
        .map(|i| {
            let angle = start + (end - start) * f64::from(i) / f64::from(n_points);
            let (sin, cos) = angle.sin_cos();
            rect.center() + radius * Vec2::new(cos as f32, sin as f32)
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(3.0, color)));
}

/// The height of a thin progress line.
pub(crate) const PROGRESS_LINE: f32 = 5.0;

/// A thin progress line in `track`, `done` from 0 to 1, filled from where
/// reading starts: the accent on a pale tint of it.
pub(crate) fn paint_progress(p: &egui::Painter, track: Rect, done: f32, dark: bool) {
    let (tint, fill) = super::theme::progress_colors(dark);
    let round = CornerRadius::same(radius::ROUND);
    p.rect_filled(track, round, tint);
    let filled = Rect::from_min_size(
        track.min,
        Vec2::new(track.width() * done.clamp(0.0, 1.0), track.height()),
    );
    p.rect_filled(flip(track, filled), round, fill);
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

/// Like `txt`, in the SemiBold weight of titles.
pub(crate) fn title_txt(text: &str, size: f32, color: Option<Color32>) -> egui::WidgetText {
    let font = semibold(size);
    if bidi::has_rtl(text) {
        bidi::job(text, font, color.unwrap_or(Color32::PLACEHOLDER)).into()
    } else {
        let r = RichText::new(text).font(font);
        match color {
            Some(c) => r.color(c).into(),
            None => r.into(),
        }
    }
}

/// A title as a label, in `size` and `color`.
pub(crate) fn title_lbl(ui: &mut Ui, text: &str, size: f32, color: Color32) -> egui::Response {
    ui.label(title_txt(text, size, Some(color)))
}

/// A small section title in SemiBold. Latin letters get a little space
/// between them; Arabic letters join, so they get none.
pub(crate) fn section_title(text: &str, color: Color32) -> egui::WidgetText {
    if bidi::has_rtl(text) {
        return title_txt(text, size::SMALL, Some(color));
    }
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: semibold(size::SMALL),
            color,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    job.into()
}

/// One line of `text` in reading order, cut with "…" to fit `width`; hovering
/// a cut line shows all of it. (A truncated egui label would cut a
/// right-to-left line at its start.)
pub(crate) fn cut_line(
    ui: &mut Ui,
    text: &str,
    width: f32,
    size: f32,
    color: Color32,
    sense: Sense,
) -> egui::Response {
    let font = FontId::proportional(size);
    let whole_width = ui
        .fonts_mut(|f| f.layout_no_wrap(text.to_string(), font.clone(), color))
        .size()
        .x;
    let line = bidi::lines(
        ui,
        text,
        &[],
        font,
        color,
        Color32::TRANSPARENT,
        width,
        1,
        None,
    );
    let galley = line.into_iter().next().map(|(g, _)| g);
    let (rect, resp) =
        ui.allocate_exact_size(galley.as_ref().map_or(Vec2::ZERO, |g| g.size()), sense);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, ui.is_enabled(), text));
    if let Some(g) = galley {
        ui.painter().galley(rect.min, g, color);
    }
    if whole_width > width {
        tip(resp, text)
    } else {
        resp
    }
}

/// A hover tooltip that wraps Arabic in reading order.
pub(crate) fn tip(resp: egui::Response, text: &str) -> egui::Response {
    if bidi::has_rtl(text) {
        resp.on_hover_ui(tip_text(text))
    } else {
        resp.on_hover_text(text)
    }
}

/// `tip` for a widget that can't be used now, which egui's own tooltip
/// leaves out.
pub(crate) fn tip_when_unavailable(resp: egui::Response, text: &str) -> egui::Response {
    if bidi::has_rtl(text) {
        resp.on_disabled_hover_ui(tip_text(text))
    } else {
        resp.on_disabled_hover_text(text)
    }
}

/// A tooltip's text, wrapped in reading order.
fn tip_text(text: &str) -> impl FnOnce(&mut Ui) {
    let text = text.to_string();
    move |ui| {
        ui.set_max_width(340.0);
        let c = ui.visuals().text_color();
        bidi::paragraph(ui, &text, FontId::proportional(size::CONTROL), c);
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
    let height = ui.spacing().interact_size.y;
    row_of(ui, height, add)
}

/// A row `height` tall from the start. A row grows as it is filled and
/// centers each item in the height it has so far, so an item shorter than
/// the ones after it sits above their line unless the row starts as tall as
/// they are.
pub(crate) fn row_of<R>(ui: &mut Ui, height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = Vec2::new(ui.available_width(), height);
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
        // The count in the label's size, so it sits on the same line,
        // in a quieter color.
        let fmt = TextFormat {
            font_id: FontId::proportional(size),
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

/// One line of text laid out in display order, as `paint_text` draws it.
pub(super) fn line_galley(ui: &Ui, text: &str, font: FontId, color: Color32) -> Arc<egui::Galley> {
    if bidi::has_rtl(text) {
        ui.fonts_mut(|f| f.layout_job(bidi::job(text, font, color)))
    } else {
        ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, color))
    }
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
    let g = line_galley(ui, text, font, color);
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
    flip_if(i18n::rtl(), within, r)
}

/// Mirrors `r` inside `within` when `rtl`: for content that keeps the side
/// of its own script whatever the window's language.
pub(crate) fn flip_if(rtl: bool, within: Rect, r: Rect) -> Rect {
    if rtl {
        Rect::from_min_max(
            Pos2::new(within.left() + within.right() - r.right(), r.top()),
            Pos2::new(within.left() + within.right() - r.left(), r.bottom()),
        )
    } else {
        r
    }
}

/// The corners of `part`, a part of a `track` with corners `radius` that
/// takes the track's whole height: the track's round corners on the sides
/// where the part reaches the track's ends, square ones inside.
pub(crate) fn part_corners(track: Rect, part: Rect, radius: u8) -> CornerRadius {
    let round_if = |reaches: bool| if reaches { radius } else { 0 };
    let (start, end) = (
        round_if(part.left() <= track.left() + 0.5),
        round_if(part.right() >= track.right() - 0.5),
    );
    CornerRadius {
        nw: start,
        sw: start,
        ne: end,
        se: end,
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

/// What sits before a menu button's text: a program's own icon, or one of
/// the app's.
pub(super) enum Lead<'a> {
    Texture(&'a egui::TextureHandle),
    Icon(Icon),
}

/// A quiet text button with a painted ▾, for menus.
pub(super) fn mode_button(ui: &mut Ui, label: &str, emphasised: bool) -> egui::Response {
    lead_mode_button(ui, None, label, emphasised)
}

/// How big the lead before a mode button's text is.
const MODE_LEAD: f32 = 16.0;

/// The room a mode button's lead takes before its text, the gap included.
fn mode_lead_room(lead: bool) -> f32 {
    if lead {
        MODE_LEAD + 6.0
    } else {
        0.0
    }
}

/// How wide a mode button is with text `text_width` wide, with a lead or not.
pub(super) fn mode_button_width(text_width: f32, lead: bool) -> f32 {
    mode_lead_room(lead) + text_width + 8.0 + 10.0 + 18.0
}

/// `mode_button` with `lead` before its text.
pub(super) fn lead_mode_button(
    ui: &mut Ui,
    lead: Option<Lead>,
    label: &str,
    emphasised: bool,
) -> egui::Response {
    let v = ui.visuals();
    let color = if emphasised {
        text_shade(accent(), v.dark_mode)
    } else {
        v.weak_text_color()
    };
    let g = ui
        .fonts_mut(|f| f.layout_job(bidi::job(label, FontId::proportional(size::CONTROL), color)));
    let lead_room = mode_lead_room(lead.is_some());
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(mode_button_width(g.size().x, lead.is_some()), 28.0),
        Sense::click(),
    );
    let v = ui.visuals();
    if emphasised {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            accent().gamma_multiply(0.12),
        );
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.hovered.weak_bg_fill,
        );
    }
    let color = if resp.hovered() && !emphasised {
        v.text_color()
    } else {
        color
    };
    if let Some(lead) = lead {
        let at = flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.left() + 9.0 + MODE_LEAD / 2.0, rect.center().y),
                Vec2::splat(MODE_LEAD),
            ),
        );
        match lead {
            Lead::Texture(texture) => {
                egui::Image::new((texture.id(), Vec2::splat(MODE_LEAD))).paint_at(ui, at)
            }
            Lead::Icon(icon) => icons::paint(ui.painter(), at, icon, color),
        }
    }
    let text_at = flip(
        rect,
        Rect::from_min_size(
            Pos2::new(
                rect.left() + 9.0 + lead_room,
                rect.center().y - g.size().y / 2.0,
            ),
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

/// Rounded tinted label: kind, state or status. The tint is `color`; the
/// text takes its text shade.
pub(crate) fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    let text = text.into();
    let shade = text_shade(color, ui.visuals().dark_mode);
    Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(radius::ROUND))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| ui.label(txt(&text, size::SMALL, Some(shade))))
        .response
}

/// How tall the search field at the top of a menu is.
const MENU_SEARCH_HEIGHT: f32 = 32.0;

/// The search field at the top of a menu, `width` wide, the search icon at
/// its start, as the window's own search looks: a thin edge, the accent's
/// while it has the keyboard.
pub(crate) fn menu_search(
    ui: &mut Ui,
    text: &mut String,
    id: egui::Id,
    hint: &str,
    width: f32,
) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let edge = if ui.memory(|m| m.has_focus(id)) {
        Stroke::new(1.5, accent())
    } else {
        Stroke::new(1.0, line_color(dark))
    };
    let side: i8 = 10;
    Frame::new()
        .fill(search_fill(dark))
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .stroke(edge)
        .inner_margin(Margin::symmetric(side, 0))
        .show(ui, |ui| {
            // The edge counts in the frame's size, so the field is as wide
            // with the accent's thicker one.
            ui.set_width(width - 2.0 * (f32::from(side) + edge.width));
            ui.set_height(MENU_SEARCH_HEIGHT);
            ui.with_layout(row_layout(), |ui| {
                let weak = ui.visuals().weak_text_color();
                icons::glyph(ui, Icon::Search, 14.0, weak);
                let field = egui::TextEdit::singleline(text)
                    .id(id)
                    .frame(Frame::NONE)
                    .hint_text(txt(hint, size::BODY, Some(weak)))
                    .font(FontId::proportional(size::BODY))
                    .align(if i18n::rtl() {
                        egui::Align2::RIGHT_CENTER
                    } else {
                        egui::Align2::LEFT_CENTER
                    })
                    .desired_width(ui.available_width());
                let resp = ui.add(field);
                set_name(&resp, hint);
                resp
            })
            .inner
        })
        .inner
}

/// The search at the top of a menu's list, `hint` in it while empty: kept
/// while the list is open and empty each time it opens, with the keyboard
/// in it then; and its words.
pub(crate) fn menu_search_words(
    ui: &mut Ui,
    id: egui::Id,
    opened: bool,
    hint: &str,
    width: f32,
) -> (egui::Response, Vec<String>) {
    let mut search = match opened {
        true => String::new(),
        false => ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default(),
    };
    let field = menu_search(ui, &mut search, id.with("field"), hint, width);
    if opened {
        field.request_focus();
    }
    let words = search_words(&search);
    ui.data_mut(|d| d.insert_temp(id, search));
    (field, words)
}

/// A menu's list, which keeps the height it had when the menu `opened`
/// while a search narrows it: a menu that shrank could jump from above its
/// button to below it, away from the field being typed in. It grows with a
/// longer list up to `max_height`, and scrolls past that.
pub(crate) fn steady_list<R>(
    ui: &mut Ui,
    id: egui::Id,
    opened: bool,
    max_height: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let kept = match opened {
        true => None,
        false => ui.data(|d| d.get_temp::<f32>(id)),
    };
    let area = egui::ScrollArea::vertical().max_height(kept.unwrap_or(max_height));
    let area = match kept {
        Some(_) => area.auto_shrink([false, false]),
        None => area,
    };
    let out = area.show(ui, add_contents);
    let natural = out.content_size.y.min(max_height);
    let height = kept.map_or(natural, |kept| kept.max(natural));
    if kept != Some(height) {
        ui.data_mut(|d| d.insert_temp(id, height));
        ui.ctx().request_repaint();
    }
    out.inner
}

/// What a menu's list shows when nothing in it has the words searched for.
pub(crate) fn nothing_matches(ui: &mut Ui, width: f32, text: &str) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 56.0), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
    paint_text(
        ui,
        ui.painter(),
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(size::BODY),
        ui.visuals().weak_text_color(),
    );
}

/// Whether Enter was pressed in the search field `field`: the first entry
/// found is then picked.
pub(crate) fn enter_in(ui: &Ui, field: &egui::Response) -> bool {
    field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// The words of a search as they are looked for: normalized as keyword
/// search compares text (`search::normalize`), so case and the ways of
/// typing an Arabic letter don't count.
pub(crate) fn search_words(search: &str) -> Vec<String> {
    crate::search::normalize(search)
        .split_whitespace()
        .map(String::from)
        .collect()
}

/// Whether each of the searched `words` is in one of `texts`.
pub(crate) fn matches(words: &[String], texts: &[&str]) -> bool {
    let texts: Vec<String> = texts
        .iter()
        .map(|text| crate::search::normalize(text))
        .collect();
    words
        .iter()
        .all(|word| texts.iter().any(|text| text.contains(word.as_str())))
}

/// One program in a menu: its own icon (the generic one while it has none)
/// and its own name, and `end` at the far end: its file name, or how many
/// copies it gave. With `checked` it can be picked with others, and shows a
/// check mark at the far end while picked.
pub(crate) fn program_row(
    ui: &mut Ui,
    name: &str,
    icon: Option<&egui::TextureHandle>,
    end: &str,
    checked: Option<bool>,
) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    match checked {
        Some(checked) => {
            r.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, checked, name))
        }
        None => r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MenuItem, true, name)),
    }
    let v = ui.visuals();
    if r.hovered() || r.has_focus() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.hovered.weak_bg_fill,
        );
    }
    let at = flip(
        rect,
        Rect::from_center_size(
            Pos2::new(rect.left() + 18.0, rect.center().y),
            Vec2::splat(18.0),
        ),
    );
    match icon {
        Some(texture) => egui::Image::new((texture.id(), Vec2::splat(18.0))).paint_at(ui, at),
        None => icons::paint(ui.painter(), at, Icon::AppGeneric, v.weak_text_color()),
    }
    let picked = checked == Some(true);
    let color = if picked {
        text_shade(accent(), v.dark_mode)
    } else {
        v.text_color()
    };
    let start = flip(
        rect,
        Rect::from_min_size(Pos2::new(rect.left() + 36.0, rect.center().y), Vec2::ZERO),
    )
    .min;
    paint_text(
        ui,
        ui.painter(),
        start,
        start_anchor(egui::Align::Center),
        name,
        FontId::proportional(size::BODY),
        color,
    );
    let mut end_x = rect.right() - 8.0;
    if picked {
        let mark = flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.right() - 16.0, rect.center().y),
                Vec2::splat(14.0),
            ),
        );
        icons::paint(ui.painter(), mark, Icon::Check, color);
        end_x = rect.right() - 30.0;
    }
    let end_at = flip(
        rect,
        Rect::from_min_size(Pos2::new(end_x, rect.center().y), Vec2::ZERO),
    )
    .min;
    let end_anchor = if i18n::rtl() {
        egui::Align2::LEFT_CENTER
    } else {
        egui::Align2::RIGHT_CENTER
    };
    paint_text(
        ui,
        ui.painter(),
        end_at,
        end_anchor,
        end,
        FontId::proportional(size::SMALL),
        v.weak_text_color(),
    );
    r
}

/// A menu row with a check mark at the far end when `checked`.
pub(super) fn check_row(ui: &mut Ui, text: &str, checked: bool) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, checked, text));
    let v = ui.visuals();
    if resp.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.hovered.weak_bg_fill,
        );
    }
    let color = if checked {
        text_shade(accent(), v.dark_mode)
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
        FontId::proportional(size::BODY),
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
        icons::paint(ui.painter(), mark, Icon::Check, color);
    }
    resp
}

/// A menu entry: its text at the reading start and its keys at the far end.
/// `color` tints the text, as red does for Delete. In a disabled `ui` it is
/// grayed out, and screen readers hear that it can't be used.
/// What a menu row shows at its start: one of the app's icons, in the
/// row's text color, or a picture in its own colors, such as another app's
/// icon.
#[derive(Clone, Copy)]
pub(super) enum MenuIcon {
    Glyph(Icon),
    Picture(egui::TextureId),
}

pub(super) fn menu_entry(
    ui: &mut Ui,
    text: &str,
    keys: &str,
    color: Option<Color32>,
    icon: Option<MenuIcon>,
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), MENU_ROW), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MenuItem, ui.is_enabled(), text));
    let v = ui.visuals();
    // The keyboard moves through a menu by focus.
    let lit = resp.hovered() || resp.has_focus();
    if lit {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.hovered.weak_bg_fill,
        );
    }
    let color = match color {
        Some(c) => text_shade(c, v.dark_mode),
        None if lit => v.strong_text_color(),
        None => v.text_color(),
    };
    let at = |x: f32| {
        flip(
            rect,
            Rect::from_min_size(Pos2::new(x, rect.center().y), Vec2::ZERO),
        )
        .min
    };
    // The action's icon at the start, as the app shows it elsewhere.
    let text_at = match icon {
        Some(icon) => {
            let place = flip(
                rect,
                Rect::from_center_size(
                    Pos2::new(rect.left() + 18.0, rect.center().y),
                    Vec2::splat(16.0),
                ),
            );
            match icon {
                MenuIcon::Glyph(icon) => icons::paint(ui.painter(), place, icon, color),
                MenuIcon::Picture(picture) => {
                    let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    ui.painter().image(picture, place, whole, Color32::WHITE);
                }
            }
            rect.left() + 34.0
        }
        None => rect.left() + 10.0,
    };
    paint_text(
        ui,
        ui.painter(),
        at(text_at),
        start_anchor(egui::Align::Center),
        text,
        FontId::proportional(size::BODY),
        color,
    );
    if !keys.trim().is_empty() {
        let end = if i18n::rtl() {
            Align2::LEFT_CENTER
        } else {
            Align2::RIGHT_CENTER
        };
        paint_text(
            ui,
            ui.painter(),
            at(rect.right() - 10.0),
            end,
            keys,
            FontId::proportional(size::SMALL),
            v.weak_text_color(),
        );
    }
    resp
}

/// How tall a menu's row is.
const MENU_ROW: f32 = 28.0;

/// Wide enough for the widest of `entries`, each its text and its keys.
pub(super) fn menu_width<'a>(
    ui: &Ui,
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> f32 {
    let width = |text: &str, size: f32| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(text.to_string(), FontId::proportional(size), Color32::WHITE)
                .size()
                .x
        })
    };
    entries
        .into_iter()
        .map(|(text, keys)| width(text, size::BODY) + 60.0 + width(keys, size::SMALL) + 20.0)
        .fold(MENU_W, f32::max)
        .min(440.0)
}

/// A thin line between groups of a menu, as tall as egui's separator.
pub(crate) fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
    let color = super::theme::menu_line(ui.visuals().dark_mode);
    ui.painter().hline(
        rect.x_range().shrink(4.0),
        rect.center().y,
        Stroke::new(1.0, color),
    );
}

/// Width of the small icon menus (add files, more tags).
pub(super) const MENU_W: f32 = 220.0;

/// The search modes, in the order their menu shows them.
pub(super) const MODES: [SearchMode; 3] = [
    SearchMode::Hybrid,
    SearchMode::Semantic,
    SearchMode::Keyword,
];

/// Where a search-mode choice's name and help start, from the row's edge.
const MODE_TEXT_START: f32 = 14.0;

/// The room a choice's name leaves at the row's end for the tick, which
/// ends 11 points from the edge.
const MODE_TICK_ROOM: f32 = 34.0;

/// Wide enough for the longest name with its tick and the longest help
/// line, no wider, so no text runs past the menu in any language.
pub(super) fn mode_menu_width(ui: &Ui) -> f32 {
    let width = |text: &str, size: f32| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(text.to_string(), FontId::proportional(size), Color32::WHITE)
                .size()
                .x
        })
    };
    MODES
        .iter()
        .map(|&m| {
            let name = width(&mode_label(m), size::BODY) + MODE_TICK_ROOM;
            let help = width(&mode_help(m), size::SMALL) + MODE_TEXT_START;
            MODE_TEXT_START + name.max(help)
        })
        .fold(MENU_W, f32::max)
        .ceil()
}

/// One search-mode choice, `width` wide: name and help in a single target,
/// the active one tinted, edged and ticked.
pub(super) fn mode_option(ui: &mut Ui, m: SearchMode, active: bool, width: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 46.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::RadioButton, true, active, mode_label(m))
    });
    let v = ui.visuals();
    let p = ui.painter();
    let a = accent();
    let r = CornerRadius::same(radius::MEDIUM);
    if active {
        p.rect_filled(
            rect,
            r,
            a.gamma_multiply(if v.dark_mode { 0.16 } else { 0.10 }),
        );
        p.rect_filled(
            flip(
                rect,
                Rect::from_min_size(
                    rect.min + Vec2::new(0.0, 10.0),
                    Vec2::new(3.0, rect.height() - 20.0),
                ),
            ),
            CornerRadius::same(radius::SMALL),
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
    let title = if active {
        text_shade(a, v.dark_mode)
    } else {
        v.strong_text_color()
    };
    let at = |y: f32| {
        flip(
            rect,
            Rect::from_min_size(
                Pos2::new(rect.left() + MODE_TEXT_START, rect.top() + y),
                Vec2::ZERO,
            ),
        )
        .min
    };
    paint_text(
        ui,
        p,
        at(8.0),
        start_anchor(egui::Align::Min),
        &mode_label(m),
        FontId::proportional(size::BODY),
        title,
    );
    paint_text(
        ui,
        p,
        at(27.0),
        start_anchor(egui::Align::Min),
        &mode_help(m),
        FontId::proportional(size::SMALL),
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
            title,
        );
    }
    resp
}

pub(crate) fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding.x = 16.0;
        let r = ui.add(
            egui::Button::new(txt(text, size::BODY, Some(Color32::WHITE)))
                .fill(button_fill(accent()))
                .min_size(Vec2::new(0.0, 32.0)),
        );
        react(ui, &r);
        r
    })
    .inner
}

/// `react` as a step in a chain of a button's response.
pub(crate) trait Reacts {
    fn react(self, ui: &Ui) -> Self;
}

impl Reacts for egui::Response {
    fn react(self, ui: &Ui) -> Self {
        react(ui, &self);
        self
    }
}

/// A filled button's answer to the pointer: lighter under it, darker while
/// pressed. egui leaves a button with a fill of its own as it is.
pub(crate) fn react(ui: &Ui, r: &egui::Response) {
    if r.hovered() && r.enabled() {
        let shade = super::theme::lit(r.is_pointer_button_down_on());
        ui.painter()
            .rect_filled(r.rect, CornerRadius::same(radius::MEDIUM), shade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How tall `steady_list` is in one frame, with `rows` rows of 20 points
    /// and room for 100.
    fn list_height(ctx: &egui::Context, opened: bool, rows: usize) -> f32 {
        let mut height = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let list = ui.scope(|ui| {
                steady_list(ui, egui::Id::unique("list"), opened, 100.0, |ui| {
                    for _ in 0..rows {
                        ui.allocate_exact_size(Vec2::new(50.0, 20.0), Sense::hover());
                    }
                });
            });
            height = list.response.rect.height();
        });
        // The texture uploads no renderer takes here.
        output.textures_delta.clear();
        height
    }

    /// A menu's search finds a name however its Arabic letters were typed,
    /// and English in any case, as the history's search does.
    #[test]
    fn a_menu_search_matches_as_the_history_search_does() {
        let found = |search: &str, name: &str| matches(&search_words(search), &[name]);
        assert!(found("إجتماع", "الاجتماعات"));
        assert!(found("عربى", "برنامج عربي"));
        assert!(found("STUDIO code", "Visual Studio Code"));
        assert!(!found("studio teams", "Visual Studio Code"));
    }

    /// A search that narrows a menu's list leaves the list as tall as it
    /// was when the menu opened, so the menu can't jump to the other side
    /// of its button; a longer list makes it taller, up to its most.
    #[test]
    fn a_menu_list_keeps_its_height_while_a_search_narrows_it() {
        let ctx = egui::Context::default();
        let full = list_height(&ctx, true, 3);
        assert!(full > 60.0 && full < 100.0, "as tall as its 3 rows: {full}");
        assert_eq!(list_height(&ctx, false, 1), full, "one row found");
        assert_eq!(list_height(&ctx, false, 0), full, "none found");
        list_height(&ctx, false, 9);
        assert_eq!(list_height(&ctx, false, 9), 100.0, "a longer list");
        assert_eq!(list_height(&ctx, false, 2), 100.0, "narrowed again");
        assert!(
            list_height(&ctx, true, 1) < full,
            "opened again with one row"
        );
    }
}
