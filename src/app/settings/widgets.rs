//! The building blocks of a settings page: sections, each a card of rows,
//! switches, segmented choices and notes.

use super::*;
use eframe::egui::layers::ShapeIdx;

fn row_title_id() -> egui::Id {
    egui::Id::unique("setting-row-title")
}

/// The title of the setting row being drawn, to name its control for
/// screen readers: a switch or a text field has no text of its own.
pub(super) fn row_title(ui: &Ui) -> String {
    ui.ctx()
        .data(|d| d.get_temp::<String>(row_title_id()))
        .unwrap_or_default()
}

/// iOS-style switch, named by its setting row for screen readers.
pub(super) fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let (title, now_on) = (row_title(ui), *on);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Switch, ui.is_enabled(), now_on, &title)
    });
    let t = ui.ctx().animate_bool(resp.id, *on);
    let off = ui.visuals().widgets.inactive.bg_fill;
    let bg = off.lerp_to_gamma(accent(), t);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(radius::ROUND), bg);
    // An outline while off, so the track stands out from the page.
    let edge = ui
        .visuals()
        .weak_text_color()
        .gamma_multiply(0.75 * (1.0 - t));
    p.rect_stroke(
        rect,
        CornerRadius::same(radius::ROUND),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    // Right to left, "on" is to the left.
    let travel = t * (rect.width() - 22.0);
    let x = if i18n::rtl() {
        rect.right() - 11.0 - travel
    } else {
        rect.left() + 11.0 + travel
    };
    p.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp
}

/// Options side by side on a track; the chosen one stands out on it.
/// Called inside a setting's control area, which fills from the far end,
/// so options are added in reverse.
pub(super) fn segmented<T: PartialEq + Copy, L: AsRef<str>>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, L)],
) -> bool {
    let mut changed = false;
    let (track, _) = choice_track(ui.visuals().dark_mode);
    Frame::new()
        .fill(track)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (v, label) in options.iter().rev() {
                let sel = *value == *v;
                let text = txt(
                    label.as_ref(),
                    size::CONTROL,
                    Some(if sel {
                        text_shade(accent(), ui.visuals().dark_mode)
                    } else {
                        ui.visuals().text_color()
                    }),
                );
                // The chosen one has the accent's edge as well as its text, so
                // it stands out from the track at a glance.
                let edge = if sel {
                    Stroke::new(1.0, accent().gamma_multiply(0.55))
                } else {
                    Stroke::NONE
                };
                let fill = if sel {
                    chosen_face(accent(), ui.visuals().dark_mode)
                } else {
                    Color32::TRANSPARENT
                };
                let b = egui::Button::new(text)
                    .frame_when_inactive(sel)
                    .corner_radius(CornerRadius::same(radius::SMALL))
                    .fill(fill)
                    .stroke(edge);
                if ui.add(b).clicked() && !sel {
                    *value = *v;
                    changed = true;
                }
            }
        });
    changed
}

/// Gives the menus drawn next in `ui` the look of a field: the face of a
/// chosen option and a thin edge, so they don't read as switched off.
pub(super) fn field_look(ui: &mut Ui) {
    let dark = ui.visuals().dark_mode;
    let (_, face) = choice_track(dark);
    let w = &mut ui.visuals_mut().widgets.inactive;
    w.weak_bg_fill = face;
    w.bg_fill = face;
    w.bg_stroke = Stroke::new(1.0, line_color(dark));
}

/// The arrow of a setting's menu: the same thin one as the window's menus.
pub(super) fn menu_arrow(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(MENU_ARROW)),
        Icon::Dropdown,
        visuals.fg_stroke.color,
    );
}

/// How big a menu's arrow is.
const MENU_ARROW: f32 = 10.0;

/// How big the info button is.
const INFO_BUTTON_SIZE: f32 = 22.0;

/// The info button beside a setting's name: a click, or Enter or Space,
/// opens its details; screen readers read them as the button's name. It
/// takes the name's `height` in its line, its square on the line's middle,
/// so a name has the same place and line with it or without it.
pub(super) fn info(ui: &mut Ui, text: &str, height: f32) {
    let (id, place) = ui.allocate_space(Vec2::new(INFO_BUTTON_SIZE, height));
    let rect = Rect::from_center_size(place.center(), Vec2::splat(INFO_BUTTON_SIZE));
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, text));
    let v = ui.visuals();
    if resp.hovered() || resp.has_focus() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.hovered.weak_bg_fill,
        );
    }
    let color = if resp.hovered() {
        v.strong_text_color()
    } else {
        v.weak_text_color()
    };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(14.0)),
        Icon::Info,
        color,
    );
    egui::Popup::from_toggle_button_response(&resp)
        .width(360.0)
        .show(|ui| {
            let strong = ui.visuals().strong_text_color();
            super::bidi::paragraph(ui, text, FontId::proportional(size::BODY), strong);
        });
}

/// How wide a setting's menu is, the same on every page, so the menus'
/// edges line up.
pub(super) const MENU_WIDTH: f32 = 280.0;

/// The reset button beside a changed setting.
const RESET_BUTTON_SIZE: f32 = 28.0;

/// How far a setting's short line is under its name.
const LINE_GAP: f32 = space::TINY;

/// The room between a setting's text and its control beside it.
const TEXT_GAP: f32 = space::HUGE;

/// The least width a setting's text keeps beside its control; with less,
/// the control goes under the text. A reset beside the control doesn't
/// count: it takes its room from the text, so it never moves the control.
const TEXT_LEAST_WIDTH: f32 = 260.0;

/// The room between a setting's text and its control under it.
const STACKED_GAP: f32 = space::SMALL;

/// The room inside a card's rows at their sides, where their text and
/// their controls end.
const ROW_SIDE_PADDING: f32 = space::LARGE;

/// The room inside a card's rows above and below their content.
const ROW_VERTICAL_PADDING: f32 = space::MEDIUM;

/// How far rows that belong to the row above them are set in.
const NESTED_INDENT: f32 = space::LARGE;

/// What a card holds so far, for the lines between its rows and the
/// corners of their tints.
#[derive(Clone, Copy, Default)]
struct CardState {
    /// What came last: nothing yet, a row or a note.
    last: Option<Part>,
    /// The tint of the last row, while that row is the card's last part: it
    /// takes the card's bottom corners when the card ends.
    tint: Option<Tint>,
    /// How far rows are set in from the reading-start side (`nested`).
    indent: f32,
    /// The gap between lines inside a row, as elsewhere on the page.
    line_gap: f32,
    /// The corners inside the card's edge, which tints take.
    corner: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Row,
    Note,
}

/// The accent's tint behind a changed row, painted under its content.
#[derive(Clone, Copy)]
struct Tint {
    shape: ShapeIdx,
    rect: Rect,
    color: Color32,
    /// The card's first part, so the tint takes its top corners.
    first: bool,
}

fn card_id() -> egui::Id {
    egui::Id::unique("settings-card")
}

fn card_state(ui: &Ui) -> Option<CardState> {
    ui.ctx().data(|d| d.get_temp::<CardState>(card_id()))
}

fn set_card_state(ui: &Ui, state: Option<CardState>) {
    ui.ctx().data_mut(|d| match state {
        Some(state) => {
            d.insert_temp(card_id(), state);
        }
        None => d.remove::<CardState>(card_id()),
    });
}

/// A part of a page: its title in SemiBold, a step larger than the
/// settings' gray lines, on the card's reading-start edge, and its
/// settings in one card under it.
pub(super) fn section<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(space::HUGE);
    let strong = ui.visuals().strong_text_color();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        row(ui, |ui| title_lbl(ui, title, size::LARGE, strong));
        ui.add_space(space::SMALL);
        card(ui, add)
    })
    .inner
}

/// Settings on a card with the surface of the window's cards, as wide as
/// the page. Rows, notes and states go in it, one under the other.
pub(super) fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let surface = card_frame(ui.visuals(), false);
    // A tint keeps inside the card's edge, so its corners are the card's
    // less the edge.
    let corner = surface
        .corner_radius
        .nw
        .saturating_sub(surface.stroke.width.round() as u8);
    let line_gap = ui.ctx().global_style().spacing.item_spacing.y;
    let outer = card_state(ui);
    let inner = surface
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            let state = CardState {
                line_gap,
                corner,
                ..Default::default()
            };
            set_card_state(ui, Some(state));
            let inner = add(ui);
            let state = card_state(ui).unwrap_or_default();
            if state.last == Some(Part::Note) {
                // As much room under a last note as a row has.
                ui.add_space(ROW_VERTICAL_PADDING);
            }
            if let Some(tint) = state.tint {
                paint_tint(ui, tint, corner, true);
            }
            inner
        })
        .inner;
    set_card_state(ui, outer);
    inner
}

/// Paints a row's tint in the place kept for it: square corners, except
/// the card's own where the row is its first or last part.
fn paint_tint(ui: &Ui, tint: Tint, corner: u8, last: bool) {
    let top = if tint.first { corner } else { 0 };
    let bottom = if last { corner } else { 0 };
    let corners = CornerRadius {
        nw: top,
        ne: top,
        sw: bottom,
        se: bottom,
    };
    let shape = egui::epaint::RectShape::filled(tint.rect, corners, tint.color);
    ui.painter().set(tint.shape, shape);
}

/// One row of a card: room at its sides and above and below, a thin line
/// between it and a row before it, and the accent's tint when `changed`.
/// Outside a card it is drawn as a card's first row.
pub(super) fn card_row<R>(ui: &mut Ui, changed: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let state = card_state(ui);
    let first = state.is_none_or(|s| s.last.is_none());
    let after_row = state.is_some_and(|s| s.last == Some(Part::Row));
    let indent = state.map_or(0.0, |s| s.indent);
    let line_gap = state.map_or(ui.spacing().item_spacing.y, |s| s.line_gap);
    // The tint goes under the content, which is drawn first.
    let background = ui.painter().add(egui::Shape::Noop);
    let start = ROW_SIDE_PADDING + indent;
    let (left, right) = if i18n::rtl() {
        (ROW_SIDE_PADDING, start)
    } else {
        (start, ROW_SIDE_PADDING)
    };
    let shown = Frame::new()
        .inner_margin(Margin {
            left: left as i8,
            right: right as i8,
            top: ROW_VERTICAL_PADDING as i8,
            bottom: ROW_VERTICAL_PADDING as i8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = line_gap;
            add(ui)
        });
    let rect = shown.response.rect;
    if after_row {
        // As far in as the row's text, so a nested row's line starts where
        // its text does.
        let line = flip(
            rect,
            Rect::from_x_y_ranges(
                rect.left() + start..=rect.right() - ROW_SIDE_PADDING,
                rect.top()..=rect.top(),
            ),
        );
        let color = line_color(ui.visuals().dark_mode);
        ui.painter()
            .hline(line.x_range(), rect.top(), Stroke::new(1.0, color));
    }
    let tint = changed.then(|| Tint {
        shape: background,
        rect,
        color: accent().gamma_multiply(if ui.visuals().dark_mode { 0.14 } else { 0.07 }),
        first,
    });
    if let Some(tint) = tint {
        paint_tint(ui, tint, state.map_or(0, |s| s.corner), false);
    }
    if let Some(mut state) = state {
        state.last = Some(Part::Row);
        state.tint = tint;
        set_card_state(ui, Some(state));
    }
    shown.inner
}

/// Rows that belong to the row above them, such as a model's settings
/// under the model: their text and the lines between them set in from the
/// reading-start side.
pub(super) fn nested<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let indent = |ui: &Ui, by: f32| {
        if let Some(mut state) = card_state(ui) {
            state.indent += by;
            set_card_state(ui, Some(state));
        }
    };
    indent(ui, NESTED_INDENT);
    let inner = add(ui);
    indent(ui, -NESTED_INDENT);
    inner
}

/// One setting as a row of its card: its name, a short gray line under it
/// and an info button with the details, and the control at the far edge.
/// A value changed from its default tints the row and offers a reset; the
/// caller must act on the returned click. An empty line or details are
/// left out.
#[must_use = "a changed row shows a reset button; act on its click"]
pub(super) fn setting(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    changed: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    setting_row(ui, label, line, details, changed, changed, control)
}

/// A row for one value with a default: tinted while the value differs, and
/// "Back to default" puts the default back. The control edits the value.
pub(super) fn setting_for<T: PartialEq, R>(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    value: &mut T,
    default: T,
    control: impl FnOnce(&mut Ui, &mut T) -> R,
) {
    if setting(ui, label, line, details, *value != default, |ui| {
        control(ui, value);
    }) {
        *value = default;
    }
}

/// A row with nothing to reset: an action, a path, or a fixed value.
pub(super) fn plain_setting(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    control: impl FnOnce(&mut Ui),
) {
    setting_row(ui, label, line, details, false, false, control);
}

/// A setting as a row of its card; see `setting`. `changed` tints it,
/// `resettable` offers the reset.
pub(super) fn setting_row(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    card_row(ui, changed, |ui| {
        setting_content(ui, label, line, details, changed, resettable, control)
    })
}

/// What a setting's row holds, for a row that shows more under it: its
/// text at the reading-start side, the name with its info button and the
/// gray line under it, its lines evened out when it wraps, and the control
/// at the far end, the two centred on each other. True when the reset was
/// clicked.
pub(super) fn setting_content(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    let weak = ui.visuals().weak_text_color();
    let under = (!line.is_empty()).then_some(|ui: &mut Ui| {
        super::bidi::balanced(ui, line, FontId::proportional(size::SMALL), weak);
    });
    setting_parts(ui, label, under, details, changed, resettable, control)
}

/// What a setting's row measured when it was drawn last, to lay its text
/// and its control out before they are drawn.
#[derive(Clone, Copy, Default)]
struct RowSizes {
    /// The control's size, without the reset beside it. The control gets
    /// the row's whole width and a place as tall as itself, beside the text
    /// as well as under it, so this is its own size wherever it is, and its
    /// width says whether it fits beside the text: a value cut to the room
    /// it gets, such as a path, is as wide as all of it.
    control: Vec2,
    /// How tall the text is.
    text: f32,
}

impl RowSizes {
    fn differs(self, other: Self) -> bool {
        let apart = |a: f32, b: f32| (a - b).abs() > 0.5;
        apart(self.control.x, other.control.x)
            || apart(self.control.y, other.control.y)
            || apart(self.text, other.text)
    }
}

/// `setting_content` with `under` the name what the caller draws there,
/// such as a state, in place of the gray line.
///
/// The text takes the width the control and its reset leave, its lines
/// wrapping there. When the control alone leaves less than
/// `TEXT_LEAST_WIDTH`, or too little for the name on one line beside a
/// reset, the control goes under the text, where it may take the row's
/// whole width; so a reset that shows or goes never moves the control.
/// The control ends on the row's far edge whether the reset shows or not,
/// so the controls of a card share one edge. The reset sits on the
/// control's reading-start side and comes after the info button and
/// before the control for the keyboard, as it reads. The parts are placed
/// by their sizes from the frame before; a part that changed size has the
/// frame drawn again.
pub(super) fn setting_parts(
    ui: &mut Ui,
    label: &str,
    under: Option<impl FnOnce(&mut Ui)>,
    details: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    let sizes_id = ui.unique_id().with(("setting sizes", label));
    let last = ui.ctx().data(|d| d.get_temp::<RowSizes>(sizes_id));
    let sizes = last.unwrap_or_default();
    let spacing = ui.spacing().item_spacing.x;
    let reset_room = if resettable {
        RESET_BUTTON_SIZE + spacing
    } else {
        0.0
    };
    let whole = ui.available_rect_before_wrap();
    let width = whole.width();
    let color = if changed {
        text_shade(accent(), ui.visuals().dark_mode)
    } else {
        ui.visuals().strong_text_color()
    };
    let name = txt(label, size::BODY, Some(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontSelection::Default,
    );
    let name_height = name.size().y;
    let name_width = if details.is_empty() {
        name.size().x
    } else {
        name.size().x + spacing + INFO_BUTTON_SIZE
    };

    // Whether the control fits beside the text. The reset doesn't count, so
    // it never moves the control: it takes its room from the text, whose
    // lines may wrap a little more. The name, which never wraps, keeps room
    // for a reset whether one shows or not, so a reset never runs into it.
    let room = (width - TEXT_GAP - TEXT_LEAST_WIDTH).max(0.0);
    let name_room = width - sizes.control.x - TEXT_GAP - RESET_BUTTON_SIZE - spacing;
    let beside = sizes.control.x <= room && name_width <= name_room;
    let far_part = sizes.control.x + reset_room;
    // The control's place spans the row, less the reset's room, beside the
    // text as under it, so the control takes its own width in both; it is
    // beside the text only while that width leaves the text its room.
    let area_left = whole.left() + reset_room;
    let area_width = (width - reset_room).max(0.0);
    let (text_width, text_top, area) = if beside {
        // The taller of the two sets the row's height, and the other is
        // centred on it. The control's place is only as tall as the
        // control: controls in a row take all the height they are given,
        // and that height is kept as the control's size, which would stay
        // with it under the text.
        let height = sizes.text.max(sizes.control.y);
        let area = Rect::from_min_size(
            Pos2::new(area_left, whole.top() + (height - sizes.control.y) / 2.0),
            Vec2::new(area_width, sizes.control.y),
        );
        let top = whole.top() + (height - sizes.text) / 2.0;
        (width - far_part - TEXT_GAP, top, area)
    } else {
        let area = Rect::from_min_size(
            Pos2::new(area_left, whole.top() + sizes.text + STACKED_GAP),
            Vec2::new(area_width, sizes.control.y),
        );
        (width, whole.top(), area)
    };

    let text_rect = Rect::from_min_size(
        Pos2::new(whole.left(), text_top),
        Vec2::new(text_width, sizes.text.max(name_height)),
    );
    let start = if i18n::rtl() {
        egui::Align::Max
    } else {
        egui::Align::Min
    };
    let mut text_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(flip(whole, text_rect))
            .layout(egui::Layout::top_down(start)),
    );
    text_ui.spacing_mut().item_spacing.y = 0.0;
    row_of(&mut text_ui, name_height, |ui| {
        ui.label(name);
        if !details.is_empty() {
            info(ui, details, name_height);
        }
    });
    if let Some(under) = under {
        text_ui.add_space(LINE_GAP);
        under(&mut text_ui);
    }
    let text = text_ui.min_rect();

    let mut reset = false;
    let mut control_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(flip(whole, area))
            .layout(far_layout()),
    );
    // Placed first, by the control's width from the frame before, and kept
    // off the control's layout so the control doesn't move. Its place is
    // centred on the control's and at least as tall as the button, which
    // would otherwise be pushed down beside a shorter control.
    let reset_place = Rect::from_center_size(
        area.center(),
        Vec2::new(area.width(), area.height().max(RESET_BUTTON_SIZE)),
    );
    let mut slot = control_ui.new_child(egui::UiBuilder::new().max_rect(flip(whole, reset_place)));
    if resettable {
        slot.add_space(sizes.control.x + spacing);
        if icons::icon_button_ex(
            &mut slot,
            Icon::Reset,
            &t!("settings-back-to-default"),
            Vec2::splat(RESET_BUTTON_SIZE),
            14.0,
            None,
            None,
        )
        .clicked()
        {
            reset = true;
        }
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(row_title_id(), label.to_string()));
    control(&mut control_ui);
    ui.ctx().data_mut(|d| d.remove::<String>(row_title_id()));
    let drawn = control_ui.min_rect();

    let now = RowSizes {
        control: drawn.size(),
        text: text.height(),
    };
    if last.is_none_or(|last| last.differs(now)) {
        ui.ctx().data_mut(|d| d.insert_temp(sizes_id, now));
        ui.ctx()
            .request_discard("a setting's row follows its parts' sizes");
        // Drawn again later if this frame can't be.
        ui.ctx().request_repaint();
    }
    let bottom = text.bottom().max(drawn.bottom());
    ui.advance_cursor_after_rect(Rect::from_x_y_ranges(whole.x_range(), whole.top()..=bottom));
    reset
}

/// A note about the settings above it, as a callout in their card.
pub(super) fn note(ui: &mut Ui, text: &str) {
    callout(ui, Icon::Info, text, false);
}

/// A note that warns, as a callout in the card of the settings it is about.
pub(super) fn warning(ui: &mut Ui, text: &str) {
    callout(ui, Icon::Warning, text, true);
}

/// How big a note's icon is.
const NOTE_ICON: f32 = 14.0;

/// A note in a card: its icon at the reading start and its text, in the
/// gray of the rows' lines, on a faint tint; one that `warns` has its icon
/// in amber on a faint amber tint. It is set in as far as the rows' text
/// and has no line above or below it, so it reads as one note about the
/// rows around it and not as a row, a field or a button.
fn callout(ui: &mut Ui, icon: Icon, text: &str, warns: bool) {
    let state = card_state(ui);
    // A row before it has its room below.
    let above = match state.and_then(|s| s.last) {
        Some(Part::Row) => 0.0,
        Some(Part::Note) => space::SMALL,
        None => ROW_VERTICAL_PADDING,
    };
    ui.add_space(above);
    let start = ROW_SIDE_PADDING + state.map_or(0.0, |s| s.indent);
    let (left, right) = if i18n::rtl() {
        (ROW_SIDE_PADDING, start)
    } else {
        (start, ROW_SIDE_PADDING)
    };
    let weak = ui.visuals().weak_text_color();
    let dark = ui.visuals().dark_mode;
    let (fill, icon_color) = if warns {
        (warning_fill(dark), text_shade(AMBER, dark))
    } else {
        (note_fill(dark), weak)
    };
    let font = FontId::proportional(size::SMALL);
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::symmetric(space::MEDIUM as i8, space::SMALL as i8))
        .outer_margin(Margin {
            left: left as i8,
            right: right as i8,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(row_layout().with_cross_align(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.x = space::SMALL;
                // On the middle of the text's first line.
                let line = ui.fonts_mut(|f| f.row_height(&font));
                let (place, _) = ui.allocate_exact_size(Vec2::new(NOTE_ICON, line), Sense::hover());
                let at = Rect::from_center_size(place.center(), Vec2::splat(NOTE_ICON));
                icons::paint(ui.painter(), at, icon, icon_color);
                super::bidi::paragraph(ui, text, font, weak);
            });
        });
    if let Some(mut state) = state {
        state.last = Some(Part::Note);
        state.tint = None;
        set_card_state(ui, Some(state));
    }
}

pub(super) fn path_field(ui: &mut Ui, p: &mut PathBuf, ext: &[&str]) {
    if ui
        .button(txt(&t!("settings-browse"), size::BODY, None))
        .clicked()
    {
        let mut dlg = rfd::FileDialog::new();
        if !ext.is_empty() {
            dlg = dlg
                .add_filter(t!("settings-filter-matching"), ext)
                .add_filter(t!("settings-filter-all"), &["*"]);
        }
        if let Some(f) = dlg.pick_file() {
            *p = f;
        }
    }
    let mut s = p.to_string_lossy().into_owned();
    let field = ui.add(egui::TextEdit::singleline(&mut s).desired_width(280.0));
    set_name(&field, &row_title(ui));
    if field.changed() {
        *p = PathBuf::from(&s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::arabic_run;

    #[test]
    fn a_switch_is_named_by_its_setting_row() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut on = true;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            plain_setting(ui, "Read text in images", "", "", |ui| {
                toggle(ui, &mut on);
            });
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.expect("a tree");
        let switch = update
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == egui::accesskit::Role::Switch)
            .expect("a switch");
        assert_eq!(switch.label(), Some("Read text in images"));
        assert_eq!(switch.toggled(), Some(egui::accesskit::Toggled::True));
    }

    /// Runs the test `name` of this module again with Arabic on.
    fn passes_in_arabic(name: &str) {
        crate::i18n::passes_in_arabic(module_path!(), name);
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    /// A window in the light theme with the app's fonts and style, which
    /// tells a screen reader what it shows.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.set_theme(egui::Theme::Light);
        ctx.enable_accesskit();
        // The fonts take effect from the next frame.
        frame(&ctx, egui::RawInput::default(), |_| {});
        ctx
    }

    /// One frame of `draw` with `input`, as the app runs it.
    fn frame(
        ctx: &egui::Context,
        input: egui::RawInput,
        mut draw: impl FnMut(&mut Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(input, |ui| {
            // As in the settings' column, where the pages set their gaps.
            ui.spacing_mut().item_spacing.y = 0.0;
            draw(ui);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        output
    }

    /// The screen reader's view of a frame: what has the focus, and each
    /// thing's name and place.
    fn spoken(output: egui::FullOutput) -> (Option<String>, Vec<(String, Rect)>) {
        let update = output.platform_output.accesskit_update.expect("a tree");
        let named: Vec<(egui::accesskit::NodeId, String, Rect)> = update
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                let bounds = node.bounds()?;
                let rect = Rect::from_min_max(
                    Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                    Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
                );
                Some((*id, node.label()?.to_string(), rect))
            })
            .collect();
        let focused = named
            .iter()
            .find(|(id, _, _)| *id == update.focus)
            .map(|(_, name, _)| name.clone());
        let places = named.into_iter().map(|(_, name, rect)| (name, rect));
        (focused, places.collect())
    }

    /// One frame of `draw` in a new window, and the shapes it painted.
    fn painted(draw: impl FnMut(&mut Ui)) -> Vec<egui::Shape> {
        let mut output = frame(&window(), egui::RawInput::default(), draw);
        let mut shapes = Vec::new();
        let mut left: Vec<egui::Shape> = std::mem::take(&mut output.shapes)
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
        while let Some(shape) = left.pop() {
            match shape {
                egui::Shape::Vec(inside) => left.extend(inside),
                shape => shapes.push(shape),
            }
        }
        shapes
    }

    /// Two rows of a card, then one set in under the second: what each row
    /// holds, and the lines painted between rows.
    fn rows_and_lines() -> (Vec<Rect>, Vec<[Pos2; 2]>) {
        let mut rows = Vec::new();
        let shapes = painted(|ui| {
            rows.clear();
            card(ui, |ui| {
                for name in ["First", "Second"] {
                    let held = card_row(ui, false, |ui| {
                        lbl(ui, name, size::BODY, Color32::GRAY);
                        ui.min_rect()
                    });
                    rows.push(held);
                }
                nested(ui, |ui| {
                    let held = card_row(ui, false, |ui| {
                        lbl(ui, "Third", size::BODY, Color32::GRAY);
                        ui.min_rect()
                    });
                    rows.push(held);
                });
            });
        });
        let color = line_color(false);
        let mut lines: Vec<[Pos2; 2]> = shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::LineSegment { points, stroke } if stroke.color == color => {
                    Some(*points)
                }
                _ => None,
            })
            .collect();
        lines.sort_by(|a, b| a[0].y.total_cmp(&b[0].y));
        (rows, lines)
    }

    fn check_lines_between_rows() {
        let (rows, lines) = rows_and_lines();
        assert_eq!(
            lines.len(),
            2,
            "a line between two rows, none above the first"
        );
        for (line, (above, below)) in lines.iter().zip([(rows[0], rows[1]), (rows[1], rows[2])]) {
            let y = line[0].y;
            assert!(
                near(y, above.bottom() + ROW_VERTICAL_PADDING),
                "{y} under {above:?}"
            );
            assert!(
                near(y, below.top() - ROW_VERTICAL_PADDING),
                "{y} over {below:?}"
            );
            // As far in as the text of the row under it, on both sides.
            assert!(near(line[0].x, below.left()) && near(line[1].x, below.right()));
        }
        // The third row is set in from the reading-start side only.
        let (start, end) = if i18n::rtl() {
            (
                rows[1].right() - rows[2].right(),
                rows[2].left() - rows[1].left(),
            )
        } else {
            (
                rows[2].left() - rows[1].left(),
                rows[1].right() - rows[2].right(),
            )
        };
        assert!(
            near(start, NESTED_INDENT) && near(end, 0.0),
            "{start}, {end}"
        );
    }

    #[test]
    fn a_line_parts_two_rows() {
        check_lines_between_rows();
    }

    #[test]
    fn a_line_parts_two_rows_right_to_left() {
        passes_in_arabic("a_line_parts_two_rows_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_line_parts_two_rows_right_to_left"]
    fn a_line_parts_two_rows_in_arabic() {
        if arabic_run() {
            check_lines_between_rows();
        }
    }

    #[test]
    fn a_tint_keeps_inside_the_cards_edge() {
        let mut inside = Rect::NOTHING;
        let shapes = painted(|ui| {
            card(ui, |ui| {
                for changed in [true, false, true] {
                    card_row(ui, changed, |ui| {
                        lbl(ui, "Setting", size::BODY, Color32::GRAY);
                    });
                }
                inside = ui.min_rect();
            });
        });
        let rects = |fill: Color32| -> Vec<egui::epaint::RectShape> {
            let mut found: Vec<_> = shapes
                .iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.clone()),
                    _ => None,
                })
                .collect();
            found.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
            found
        };
        // The card: one point of edge around what it holds.
        let card = rects(Color32::WHITE);
        let card = card
            .iter()
            .find(|shape| shape.corner_radius == CornerRadius::same(radius::LARGE))
            .expect("a card");
        assert_eq!(card.rect, inside.expand(1.0));
        // The first and last rows' tints, inside that edge, with its corners.
        let tints = rects(accent().gamma_multiply(0.07));
        assert_eq!(tints.len(), 2);
        let corner = radius::LARGE - 1;
        let (first, last) = (&tints[0], &tints[1]);
        assert_eq!(
            first.corner_radius,
            CornerRadius {
                nw: corner,
                ne: corner,
                sw: 0,
                se: 0
            }
        );
        assert_eq!(
            last.corner_radius,
            CornerRadius {
                nw: 0,
                ne: 0,
                sw: corner,
                se: corner
            }
        );
        for tint in [first, last] {
            assert_eq!(tint.rect.x_range(), inside.x_range());
        }
        assert_eq!(first.rect.top(), inside.top());
        assert_eq!(last.rect.bottom(), inside.bottom());
    }

    /// A section's title and the card under it: the title's rect and the
    /// card's outer one.
    fn check_title_on_the_cards_edge() {
        let mut inside = Rect::NOTHING;
        let shapes = painted(|ui| {
            section(ui, "Look", |ui| {
                card_row(ui, false, |ui| {
                    lbl(ui, "Theme", size::BODY, Color32::GRAY);
                });
                inside = ui.min_rect();
            });
        });
        let title = shapes
            .iter()
            .find_map(|shape| match shape {
                // A galley laid out from its right edge has its rect left
                // of its position.
                egui::Shape::Text(text) if text.galley.text() == "Look" => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("the title");
        let card = inside.expand(1.0);
        // Text snaps to whole pixels, up to half of one away.
        let off = if i18n::rtl() {
            title.right() - card.right()
        } else {
            title.left() - card.left()
        };
        assert!(off.abs() <= 0.5, "{title:?} on {card:?}");
        let gap = card.top() - title.bottom();
        assert!((space::SMALL..=space::SMALL + 2.0).contains(&gap), "{gap}");
    }

    #[test]
    fn a_title_sits_on_its_cards_edge() {
        check_title_on_the_cards_edge();
    }

    #[test]
    fn a_title_sits_on_its_cards_edge_right_to_left() {
        passes_in_arabic("a_title_sits_on_its_cards_edge_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_title_sits_on_its_cards_edge_right_to_left"]
    fn a_title_sits_on_its_cards_edge_in_arabic() {
        if arabic_run() {
            check_title_on_the_cards_edge();
        }
    }

    /// The row's far edge: where its controls end.
    fn far_edge(rect: Rect) -> f32 {
        if i18n::rtl() {
            rect.left()
        } else {
            rect.right()
        }
    }

    /// A card of a switch row, then a changed switch row and a changed menu
    /// row with their resets showing, over two frames: every control ends on
    /// the rows' far edge, each reset sits beside its own control, and the
    /// second frame needs no pass again.
    fn check_controls_share_one_edge() {
        let ctx = window();
        let (mut on, mut size) = (true, 1);
        let mut controls = [Rect::NOTHING; 3];
        let mut inside = Rect::NOTHING;
        let mut draw = |ui: &mut Ui| {
            card(ui, |ui| {
                let _ = setting_row(ui, "Tray", "Keeps running", "", false, false, |ui| {
                    controls[0] = toggle(ui, &mut on).rect;
                });
                let _ = setting_row(ui, "Start", "At sign-in", "", true, true, |ui| {
                    controls[1] = toggle(ui, &mut on).rect;
                });
                inside = card_row(ui, true, |ui| {
                    setting_content(ui, "Keep up to", "Items", "", true, true, |ui| {
                        controls[2] = egui::ComboBox::from_id_salt("size")
                            .width(MENU_WIDTH)
                            .selected_text("1,000")
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut size, 1, "1,000");
                            })
                            .response
                            .rect;
                    });
                    ui.min_rect()
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        assert!(!output.platform_output.requested_discard(), "steady");
        let edge = far_edge(inside);
        for control in controls {
            assert!(near(far_edge(control), edge), "{control:?} ends at {edge}");
        }
        let gap = ctx.global_style().spacing.item_spacing.x;
        let reset = t!("settings-back-to-default");
        let (_, places) = spoken(output);
        let mut resets: Vec<Pos2> = places
            .into_iter()
            .filter(|(name, _)| *name == reset)
            .map(|(_, rect)| rect.min)
            .collect();
        resets.sort_by(|a, b| a.y.total_cmp(&b.y));
        let beside = [reset_min(controls[1], gap), reset_min(controls[2], gap)];
        assert_eq!(resets, beside);
    }

    #[test]
    fn controls_share_one_edge() {
        check_controls_share_one_edge();
    }

    #[test]
    fn controls_share_one_edge_right_to_left() {
        passes_in_arabic("controls_share_one_edge_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by controls_share_one_edge_right_to_left"]
    fn controls_share_one_edge_in_arabic() {
        if arabic_run() {
            check_controls_share_one_edge();
        }
    }

    /// A setting before and after it changes: its switch, and its reset
    /// button when it shows.
    fn switch_and_reset(ctx: &egui::Context, changed: bool) -> (Rect, Option<Rect>) {
        let mut on = true;
        let mut switch = Rect::NOTHING;
        let output = frame(ctx, egui::RawInput::default(), |ui| {
            card(ui, |ui| {
                let _ = setting_row(ui, "Tray", "Keeps running", "", changed, changed, |ui| {
                    switch = toggle(ui, &mut on).rect;
                });
            });
        });
        let reset = t!("settings-back-to-default");
        let (_, places) = spoken(output);
        let found = places.into_iter().find(|(name, _)| *name == reset);
        (switch, found.map(|(_, rect)| rect))
    }

    fn check_reset_beside_its_control() {
        let ctx = window();
        let (before, none) = switch_and_reset(&ctx, false);
        assert!(none.is_none(), "no reset for a default value");
        let (after, reset) = switch_and_reset(&ctx, true);
        assert_eq!(before, after, "the switch stays where it was");
        let reset = reset.expect("a reset button");
        // On the switch's reading-start side, the usual gap from it, on its line.
        let gap = ctx.global_style().spacing.item_spacing.x;
        let apart = if i18n::rtl() {
            reset.left() - after.right()
        } else {
            after.left() - reset.right()
        };
        assert!(near(apart, gap), "{reset:?} beside {after:?}");
        assert!(near(reset.center().y, after.center().y));
        // The same in a new window, whose first frame has measured nothing.
        let (first, reset) = switch_and_reset(&window(), true);
        assert_eq!(
            (first, reset.map(|r| r.min)),
            (after, Some(reset_min(after, gap)))
        );
    }

    /// Where the reset button's corner goes beside `control`.
    fn reset_min(control: Rect, gap: f32) -> Pos2 {
        let x = if i18n::rtl() {
            control.right() + gap
        } else {
            control.left() - gap - RESET_BUTTON_SIZE
        };
        Pos2::new(x, control.center().y - RESET_BUTTON_SIZE / 2.0)
    }

    #[test]
    fn a_reset_shows_beside_its_control_without_moving_it() {
        check_reset_beside_its_control();
    }

    #[test]
    fn a_reset_shows_beside_its_control_right_to_left() {
        passes_in_arabic("a_reset_shows_beside_its_control_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_reset_shows_beside_its_control_right_to_left"]
    fn a_reset_shows_beside_its_control_in_arabic() {
        if arabic_run() {
            check_reset_beside_its_control();
        }
    }

    /// Tab goes through a changed row as it reads: the details, the reset,
    /// then the control.
    #[test]
    fn the_keyboard_reaches_a_reset_before_its_control() {
        let ctx = window();
        let mut on = true;
        let tab = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut order = Vec::new();
        for _ in 0..3 {
            let output = frame(&ctx, tab.clone(), |ui| {
                card(ui, |ui| {
                    let _ = setting_row(ui, "Tray", "", "Details", true, true, |ui| {
                        toggle(ui, &mut on);
                    });
                });
            });
            order.push(spoken(output).0.unwrap_or_default());
        }
        let reset = t!("settings-back-to-default");
        assert_eq!(order, ["Details", reset.as_str(), "Tray"]);
    }

    /// A row with no details over one with details: the first paints no
    /// info button and takes no focus stop for one, so Tab goes straight to
    /// its switch. Its name and line sit where the second's do, and it is
    /// as tall.
    fn check_row_without_details() {
        let ctx = window();
        let mut on = true;
        let mut rows = [Rect::NOTHING; 2];
        let mut draw = |ui: &mut Ui| {
            card(ui, |ui| {
                for (row, details) in rows.iter_mut().zip(["", "Details"]) {
                    *row = card_row(ui, false, |ui| {
                        let _ = setting_content(
                            ui,
                            "Tray",
                            "Keeps running",
                            details,
                            false,
                            false,
                            |ui| {
                                toggle(ui, &mut on);
                            },
                        );
                        ui.min_rect()
                    });
                }
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let tab = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let output = frame(&ctx, tab, &mut draw);
        // From the row's reading-start corner.
        let place = |text: Rect, row: Rect| {
            let x = if i18n::rtl() {
                row.right() - text.right()
            } else {
                text.left() - row.left()
            };
            Vec2::new(x, text.top() - row.top())
        };
        for words in ["Tray", "Keeps running"] {
            let texts = text_rects(&output, words);
            assert_eq!(texts.len(), 2, "{words}");
            let (without, with) = (place(texts[0], rows[0]), place(texts[1], rows[1]));
            assert!(
                (without - with).length() <= 0.5,
                "{words}: {without:?} {with:?}"
            );
        }
        assert!(
            (rows[0].height() - rows[1].height()).abs() <= 0.5,
            "{rows:?}"
        );
        let (focused, places) = spoken(output);
        assert_eq!(focused.as_deref(), Some("Tray"), "Tab goes to the switch");
        let buttons: Vec<Rect> = places
            .into_iter()
            .filter(|(name, _)| name == "Details")
            .map(|(_, rect)| rect)
            .collect();
        assert_eq!(buttons.len(), 1, "one info button");
        assert!(
            rows[1].contains(buttons[0].center()),
            "{buttons:?} in {rows:?}"
        );
    }

    #[test]
    fn a_row_without_details_has_no_info_button() {
        check_row_without_details();
    }

    #[test]
    fn a_row_without_details_has_no_info_button_right_to_left() {
        passes_in_arabic("a_row_without_details_has_no_info_button_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_row_without_details_has_no_info_button_right_to_left"]
    fn a_row_without_details_has_no_info_button_in_arabic() {
        if arabic_run() {
            check_row_without_details();
        }
    }

    /// `draw` in a settings column of the widest kind.
    fn in_column(ui: &mut Ui, draw: impl FnOnce(&mut Ui)) {
        let column = Rect::from_min_size(
            Pos2::new(20.0, 20.0),
            Vec2::new(super::super::COLUMN_WIDTH, 2000.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(column), draw);
    }

    /// Where the text that reads `words` was painted, one rect per line.
    fn text_rects(output: &egui::FullOutput, words: &str) -> Vec<Rect> {
        let mut found: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if !text.galley.text().is_empty() && words.contains(text.galley.text()) =>
                {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| a.top().total_cmp(&b.top()));
        found
    }

    /// A row with a short line and a switch shorter than its text, a row
    /// with only a name and a recorder taller than it, and a row whose line
    /// wraps beside a wide menu: each line starts 4 points under its name,
    /// no line reaches under a control, and each control's middle is its
    /// text's middle.
    fn check_text_and_control() {
        let ctx = window();
        let (mut on, mut size, mut problem) = (true, 1, None);
        let mut keys = "Ctrl+Shift+V".to_string();
        let lines = [
            "Keeps running",
            "",
            &"each model keeps its own data ".repeat(4),
        ];
        let names = ["Tray", "Paste", "Model"];
        let mut controls = [Rect::NOTHING; 3];
        let mut draw = |ui: &mut Ui| {
            in_column(ui, |ui| {
                card(ui, |ui| {
                    let _ = setting_row(ui, names[0], lines[0], "Details", false, false, |ui| {
                        controls[0] = toggle(ui, &mut on).rect;
                    });
                    let _ = setting_row(ui, names[1], lines[1], "", true, true, |ui| {
                        crate::app::shortcut::recorder(ui, &mut keys, false, false, &mut problem);
                        controls[1] = ui.min_rect();
                    });
                    let _ = setting_row(ui, names[2], lines[2], "Details", false, false, |ui| {
                        controls[2] = egui::ComboBox::from_id_salt("model")
                            .width(MENU_WIDTH)
                            .selected_text("EmbeddingGemma")
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut size, 1, "EmbeddingGemma");
                            })
                            .response
                            .rect;
                    });
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        assert!(!output.platform_output.requested_discard(), "steady");
        for ((name, line), control) in names.iter().zip(lines).zip(controls) {
            let name = text_rects(&output, name)[0];
            let mut text = name;
            if !line.is_empty() {
                let lines = text_rects(&output, line);
                let gap = lines[0].top() - name.bottom();
                assert!((gap - LINE_GAP).abs() <= 0.5, "{gap} under {name:?}");
                for line in &lines {
                    let apart = if i18n::rtl() {
                        line.left() - control.right()
                    } else {
                        control.left() - line.right()
                    };
                    assert!(apart >= TEXT_GAP, "{line:?} beside {control:?}");
                    text = text.union(*line);
                }
            }
            let off = control.center().y - text.center().y;
            assert!(off.abs() <= 0.5, "{control:?} on {text:?}");
        }
        assert!(text_rects(&output, lines[2]).len() > 1, "a long line wraps");
    }

    #[test]
    fn a_settings_text_and_its_control_are_centred_on_each_other() {
        check_text_and_control();
    }

    #[test]
    fn a_settings_text_and_its_control_are_centred_on_each_other_right_to_left() {
        passes_in_arabic("a_settings_text_and_its_control_are_centred_on_each_other_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_settings_text_and_its_control_are_centred_on_each_other_right_to_left"]
    fn a_settings_text_and_its_control_are_centred_on_each_other_in_arabic() {
        if arabic_run() {
            check_text_and_control();
        }
    }

    /// The "Read by" row of Settings, Search & AI, in the widest column: its
    /// choice of three leaves its gray line two lines, which come out about
    /// equally long, the last at least half as wide as the first, and the
    /// choice is centred on the name and the two lines.
    #[test]
    fn a_wrapped_gray_line_has_even_lines() {
        let ctx = window();
        let (name, line) = (t!("settings-read-engine"), t!("settings-read-engine-hint"));
        let mut engine = 2;
        let mut control = Rect::NOTHING;
        let mut draw = |ui: &mut Ui| {
            in_column(ui, |ui| {
                card(ui, |ui| {
                    plain_setting(ui, &name, &line, "", |ui| {
                        let engines = [
                            (0, t!("settings-engine-off")),
                            (1, t!("settings-engine-windows")),
                            (2, t!("settings-engine-server")),
                        ];
                        segmented(ui, &mut engine, &engines);
                        control = ui.min_rect();
                    });
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        assert!(!output.platform_output.requested_discard(), "steady");
        let lines = text_rects(&output, &line);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[1].width() >= lines[0].width() / 2.0, "{lines:?}");
        let text = lines
            .iter()
            .fold(text_rects(&output, &name)[0], |text, line| {
                text.union(*line)
            });
        let off = control.center().y - text.center().y;
        assert!(off.abs() <= 0.5, "{control:?} on {text:?}");
    }

    /// The widest controls in the widest column, the choice and the recorder
    /// changed so their resets show: the model menu, a three-way choice and
    /// a key recorder with a note beside it stay beside their text, centred
    /// on it, and leave it at least `TEXT_LEAST_WIDTH`. A reset takes its
    /// room from the text, which ends `TEXT_GAP` before it.
    fn check_widest_controls_leave_room() {
        let ctx = window();
        let (mut mode, mut size, mut problem) = (SearchMode::Hybrid, 1, None);
        let mut keys = "Ctrl+Shift+V".to_string();
        let mut rows = [(Rect::NOTHING, Rect::NOTHING); 3];
        let mut draw = |ui: &mut Ui| {
            in_column(ui, |ui| {
                card(ui, |ui| {
                    rows[0] = card_row(ui, true, |ui| {
                        let mut menu = Rect::NOTHING;
                        let model = t!("settings-model");
                        let line = t!("settings-search-model-hint");
                        setting_content(ui, &model, &line, "", false, false, |ui| {
                            field_look(ui);
                            menu = egui::ComboBox::from_id_salt("model")
                                .icon(menu_arrow)
                                .width(MENU_WIDTH)
                                .selected_text("EmbeddingGemma · llama.cpp")
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut size, 1, "EmbeddingGemma");
                                })
                                .response
                                .rect;
                        });
                        (ui.min_rect(), menu)
                    });
                    rows[1] = card_row(ui, true, |ui| {
                        let mut choice = Rect::NOTHING;
                        let name = t!("settings-search-mode");
                        let line = t!("search-mode-smart-help");
                        setting_content(ui, &name, &line, "", true, true, |ui| {
                            let modes = [
                                (SearchMode::Hybrid, t!("search-mode-smart")),
                                (SearchMode::Semantic, t!("search-mode-meaning")),
                                (SearchMode::Keyword, t!("search-mode-words")),
                            ];
                            segmented(ui, &mut mode, &modes);
                            choice = ui.min_rect();
                        });
                        (ui.min_rect(), choice)
                    });
                    rows[2] = card_row(ui, true, |ui| {
                        let mut recorder = Rect::NOTHING;
                        let name = t!("settings-shortcut-popup");
                        let line = t!("settings-shortcut-popup-hint");
                        setting_content(ui, &name, &line, "", true, true, |ui| {
                            crate::app::shortcut::recorder(
                                ui,
                                &mut keys,
                                false,
                                true,
                                &mut problem,
                            );
                            chip(ui, t!("settings-shortcut-taken"), RED);
                            recorder = ui.min_rect();
                        });
                        (ui.min_rect(), recorder)
                    });
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        let texts: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
                _ => None,
            })
            .collect();
        let reset = t!("settings-back-to-default");
        let (_, places) = spoken(output);
        for (row, control) in rows {
            let room = if i18n::rtl() {
                row.right() - control.right() - TEXT_GAP
            } else {
                control.left() - row.left() - TEXT_GAP
            };
            assert!(
                room >= TEXT_LEAST_WIDTH,
                "{room} beside {control:?} in {row:?}"
            );
            let with_reset = places
                .iter()
                .filter(|(name, rect)| *name == reset && row.y_range().contains(rect.center().y))
                .fold(control, |beside, (_, rect)| beside.union(*rect));
            // The row's name and gray line, outside the control.
            let text = texts
                .iter()
                .filter(|at| row.contains(at.center()) && !control.contains(at.center()));
            for at in text {
                let apart = if i18n::rtl() {
                    at.left() - with_reset.right()
                } else {
                    with_reset.left() - at.right()
                };
                assert!(apart >= TEXT_GAP - 0.5, "{at:?} beside {with_reset:?}");
            }
            let off = control.center().y - row.center().y;
            assert!(off.abs() <= 0.5, "{control:?} beside its text in {row:?}");
        }
    }

    #[test]
    fn the_widest_controls_leave_room_for_their_text() {
        check_widest_controls_leave_room();
    }

    #[test]
    fn the_widest_controls_leave_room_for_their_text_right_to_left() {
        passes_in_arabic("the_widest_controls_leave_room_for_their_text_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by the_widest_controls_leave_room_for_their_text_right_to_left"]
    fn the_widest_controls_leave_room_for_their_text_in_arabic() {
        if arabic_run() {
            check_widest_controls_leave_room();
        }
    }

    /// The Version and Data folder rows of Settings, About, as a development
    /// build draws them, in the widest column: the update's state and the
    /// version, and the folder's Open button and path, would leave their
    /// text less than `TEXT_LEAST_WIDTH`. Each goes under its text, at the
    /// row's far edge, the gray line takes one line, and the path shows in
    /// full.
    #[test]
    fn a_long_value_goes_under_its_text() {
        let ctx = window();
        let path = r"C:\Users\Someone\AppData\Roaming\clipboard-manager\data";
        let lines = [t!("settings-version-hint"), t!("settings-data-folder-hint")];
        let mut rows = [(Rect::NOTHING, Rect::NOTHING); 2];
        let mut draw = |ui: &mut Ui| {
            in_column(ui, |ui| {
                card(ui, |ui| {
                    rows[0] = card_row(ui, false, |ui| {
                        let mut control = Rect::NOTHING;
                        let name = t!("settings-version");
                        setting_content(ui, &name, &lines[0], "", false, false, |ui| {
                            // The state, cut to the room the version leaves.
                            let weak = ui.visuals().weak_text_color();
                            let version = txt(&crate::update::current_label(), size::CONTROL, None)
                                .into_galley(
                                    ui,
                                    Some(egui::TextWrapMode::Extend),
                                    f32::INFINITY,
                                    egui::FontSelection::Default,
                                );
                            let spacing = ui.spacing().item_spacing.x;
                            let room = ui.available_width() - version.size().x - 2.0 * spacing;
                            let state = t!("settings-updates-off-dev");
                            cut_line(ui, &state, room.max(0.0), size::SMALL, weak, Sense::hover());
                            ui.label(version);
                            control = ui.min_rect();
                        });
                        (ui.min_rect(), control)
                    });
                    rows[1] = card_row(ui, false, |ui| {
                        let mut control = Rect::NOTHING;
                        let name = t!("settings-data-folder");
                        setting_content(ui, &name, &lines[1], "", false, false, |ui| {
                            let _ = ui.button(txt(&t!("settings-open"), size::BODY, None));
                            let weak = ui.visuals().weak_text_color();
                            let shown = RichText::new(path).size(size::SMALL).color(weak);
                            ui.add(egui::Label::new(shown).truncate());
                            control = ui.min_rect();
                        });
                        (ui.min_rect(), control)
                    });
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        assert!(!output.platform_output.requested_discard(), "steady");
        for ((row, control), line) in rows.into_iter().zip(&lines) {
            let shown = text_rects(&output, line);
            assert_eq!(shown.len(), 1, "{line:?} on one line: {shown:?}");
            assert!(
                control.top() >= shown[0].bottom() + STACKED_GAP - 0.5,
                "{control:?} under {shown:?}"
            );
            let end = far_edge(control) - far_edge(row);
            assert!(end.abs() <= 0.5, "{control:?} at the end of {row:?}");
        }
        assert_eq!(text_rects(&output, path).len(), 1, "the whole path");
    }

    /// In a row too narrow for a menu beside its text, the menu goes under
    /// the text, at the row's far edge.
    fn check_control_under_text_when_narrow() {
        let ctx = window();
        let mut size = 1;
        let (mut row, mut menu) = (Rect::NOTHING, Rect::NOTHING);
        let mut draw = |ui: &mut Ui| {
            let narrow = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(300.0, 600.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(narrow), |ui| {
                card(ui, |ui| {
                    row = card_row(ui, false, |ui| {
                        setting_content(ui, "Model", "The one in use", "", false, false, |ui| {
                            menu = egui::ComboBox::from_id_salt("model")
                                .width(MENU_WIDTH / 2.0)
                                .selected_text("EmbeddingGemma")
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut size, 1, "EmbeddingGemma");
                                })
                                .response
                                .rect;
                        });
                        ui.min_rect()
                    });
                });
            });
        };
        frame(&ctx, egui::RawInput::default(), &mut draw);
        let output = frame(&ctx, egui::RawInput::default(), &mut draw);
        assert!(!output.platform_output.requested_discard(), "steady");
        let line = text_rects(&output, "The one in use")[0];
        assert!(
            menu.top() >= line.bottom() + STACKED_GAP - 0.5,
            "{menu:?} under {line:?}"
        );
        let end = far_edge(menu) - far_edge(row);
        assert!(end.abs() <= 0.5, "{menu:?} at the end of {row:?}");
        assert!(
            (row.bottom() - menu.bottom()).abs() <= 0.5,
            "{row:?} holds {menu:?}"
        );
    }

    #[test]
    fn a_control_goes_under_its_text_in_a_narrow_row() {
        check_control_under_text_when_narrow();
    }

    #[test]
    fn a_control_goes_under_its_text_in_a_narrow_row_right_to_left() {
        passes_in_arabic("a_control_goes_under_its_text_in_a_narrow_row_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_control_goes_under_its_text_in_a_narrow_row_right_to_left"]
    fn a_control_goes_under_its_text_in_a_narrow_row_in_arabic() {
        if arabic_run() {
            check_control_under_text_when_narrow();
        }
    }

    /// A prompt row of a model's settings, `name` with the gray line `line`,
    /// set in under its model, in the widest column, with its field `width`
    /// points wide and its reset showing when `changed`, drawn in `ctx` as
    /// the app draws it until a frame needs no pass again: the row's rect,
    /// the field's, and that frame.
    fn settled_prompt_row(
        ctx: &egui::Context,
        name: &str,
        line: &str,
        width: f32,
        changed: bool,
    ) -> (Rect, Rect, egui::FullOutput) {
        let mut prompt = String::new();
        let (mut row, mut field) = (Rect::NOTHING, Rect::NOTHING);
        let mut draw = |ui: &mut Ui| {
            in_column(ui, |ui| {
                card(ui, |ui| {
                    nested(ui, |ui| {
                        row = card_row(ui, changed, |ui| {
                            let _ = setting_content(ui, name, line, "", changed, changed, |ui| {
                                let edit = egui::TextEdit::singleline(&mut prompt);
                                field = ui.add(edit.desired_width(width)).rect;
                            });
                            ui.min_rect()
                        });
                    });
                });
            });
        };
        let mut output = frame(ctx, egui::RawInput::default(), &mut draw);
        for _ in 0..3 {
            if !output.platform_output.requested_discard() {
                break;
            }
            output = frame(ctx, egui::RawInput::default(), &mut draw);
        }
        assert!(
            !output.platform_output.requested_discard(),
            "the row settles"
        );
        (row, field, output)
    }

    /// Whether two rects are in the same place, up to half a point.
    fn same_place(a: Rect, b: Rect) -> bool {
        (a.min - b.min).length() + (a.max - b.max).length() <= 0.5
    }

    /// The width of a field that leaves the prompt row's text its room,
    /// and of one that doesn't.
    const FIELD_BESIDE: f32 = 280.0;
    const FIELD_UNDER: f32 = 400.0;

    /// Whether `field` is beside its text, centred on it, rather than under
    /// it, at the bottom of `row`.
    fn beside_its_text(row: Rect, field: Rect) -> bool {
        let centred = (field.center().y - row.center().y).abs() <= 0.5;
        let under = (field.bottom() - row.bottom()).abs() <= 0.5 && field.top() > row.center().y;
        assert!(centred != under, "{field:?} in {row:?}");
        centred
    }

    /// The prompt row's field beside its text, then wide enough to go under
    /// it, and the other way round, while the row is drawn: once it
    /// settles, the field and the row's height are those of the same row
    /// drawn in a new window in that state.
    fn check_row_changing_its_layout() {
        let name = t!("settings-server-query-prompt");
        let line = t!("settings-server-query-prompt-hint");
        let settled = |ctx: &egui::Context, width: f32| {
            let (row, field, _) = settled_prompt_row(ctx, &name, &line, width, false);
            (row, field)
        };
        for (before, after) in [(FIELD_BESIDE, FIELD_UNDER), (FIELD_UNDER, FIELD_BESIDE)] {
            let ctx = window();
            let (row, field) = settled(&ctx, before);
            assert_eq!(beside_its_text(row, field), before == FIELD_BESIDE);
            let (row, field) = settled(&ctx, after);
            let (fresh_row, fresh_field) = settled(&window(), after);
            assert_eq!(
                beside_its_text(fresh_row, fresh_field),
                after == FIELD_BESIDE
            );
            assert!(
                same_place(field, fresh_field),
                "{field:?} drawn on, {fresh_field:?} new"
            );
            let taller = row.height() - fresh_row.height();
            assert!(taller.abs() <= 0.5, "{row:?} drawn on, {fresh_row:?} new");
        }
    }

    #[test]
    fn a_row_that_changes_its_layout_keeps_no_old_size() {
        check_row_changing_its_layout();
    }

    #[test]
    fn a_row_that_changes_its_layout_keeps_no_old_size_right_to_left() {
        passes_in_arabic("a_row_that_changes_its_layout_keeps_no_old_size_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_row_that_changes_its_layout_keeps_no_old_size_right_to_left"]
    fn a_row_that_changes_its_layout_keeps_no_old_size_in_arabic() {
        if arabic_run() {
            check_row_changing_its_layout();
        }
    }

    /// The Query prompt and Item prompt rows of a model's settings before
    /// and after a change shows their reset, as when the first letter is
    /// typed: each field stays beside its text, as far along the row, and
    /// the text gives the reset its room, ending `TEXT_GAP` before it. The
    /// Item prompt's gray line keeps its two lines there, so its field
    /// stays where it was; the Query prompt's takes a third, so its field,
    /// centred on the text, moves down half a line.
    fn check_reset_keeps_its_control_beside_the_text() {
        let rows = [
            (
                t!("settings-server-query-prompt"),
                t!("settings-server-query-prompt-hint"),
                false,
            ),
            (
                t!("settings-server-item-prompt"),
                t!("settings-server-item-prompt-hint"),
                true,
            ),
        ];
        let reset = t!("settings-back-to-default");
        for (name, line, keeps_its_lines) in rows {
            let ctx = window();
            let (row, without, _) = settled_prompt_row(&ctx, &name, &line, FIELD_BESIDE, false);
            assert!(beside_its_text(row, without), "{name}: {without:?}");
            let (row, with, output) = settled_prompt_row(&ctx, &name, &line, FIELD_BESIDE, true);
            assert!(
                beside_its_text(row, with),
                "{name}: {with:?} with the reset"
            );
            let along =
                (with.left() - without.left()).abs() + (with.right() - without.right()).abs();
            assert!(
                along <= 0.5,
                "{name}: {with:?} with the reset, {without:?} without"
            );
            if keeps_its_lines {
                assert!(same_place(with, without), "{name}: {with:?}, {without:?}");
            }
            // The name and the gray line's lines.
            let texts: Vec<Rect> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
                    _ => None,
                })
                .filter(|at| row.contains(at.center()) && !with.contains(at.center()))
                .collect();
            assert!(texts.len() >= 3, "{name}: {texts:?}");
            let (_, places) = spoken(output);
            let button = places
                .iter()
                .find(|(said, _)| *said == reset)
                .map(|(_, at)| *at)
                .expect("a reset");
            for text in texts {
                let room = if i18n::rtl() {
                    text.left() - button.right()
                } else {
                    button.left() - text.right()
                };
                assert!(room >= TEXT_GAP - 0.5, "{name}: {text:?} before {button:?}");
            }
        }
    }

    #[test]
    fn a_reset_keeps_its_control_beside_the_text() {
        check_reset_keeps_its_control_beside_the_text();
    }

    #[test]
    fn a_reset_keeps_its_control_beside_the_text_right_to_left() {
        passes_in_arabic("a_reset_keeps_its_control_beside_the_text_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_reset_keeps_its_control_beside_the_text_right_to_left"]
    fn a_reset_keeps_its_control_beside_the_text_in_arabic() {
        if arabic_run() {
            check_reset_keeps_its_control_beside_the_text();
        }
    }

    /// A row, a note, a row and a last note that warns, in a card: the
    /// notes keep the rows' room and inset, have no lines beside them, and
    /// their text follows their icon from the reading-start side.
    fn check_notes_in_a_card() {
        let mut rows = Vec::new();
        let shapes = painted(|ui| {
            rows.clear();
            card(ui, |ui| {
                for (name, about) in [("Tray", "A note."), ("Start", "A warning.")] {
                    let held = card_row(ui, false, |ui| {
                        lbl(ui, name, size::BODY, Color32::GRAY);
                        ui.min_rect()
                    });
                    rows.push(held);
                    if name == "Tray" {
                        note(ui, about);
                    } else {
                        warning(ui, about);
                    }
                }
            });
        });
        let filled = |fill: Color32| -> Vec<Rect> {
            let mut found: Vec<Rect> = shapes
                .iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
                    _ => None,
                })
                .collect();
            found.sort_by(|a, b| a.top().total_cmp(&b.top()));
            found
        };
        let (note, warning) = (filled(note_fill(false)), filled(warning_fill(false)));
        assert_eq!(
            (note.len(), warning.len()),
            (1, 1),
            "a gray note, an amber warning"
        );
        let notes = [note[0], warning[0]];
        let room = ROW_VERTICAL_PADDING;
        for (row, note) in rows.iter().zip(&notes) {
            assert!(near(note.left(), row.left()) && near(note.right(), row.right()));
            assert!(
                near(note.top(), row.bottom() + room),
                "{note:?} under {row:?}"
            );
        }
        assert!(near(rows[1].top(), notes[0].bottom() + room));
        // The card ends a row's room under its last note, past its edge.
        let card = filled(Color32::WHITE)
            .into_iter()
            .find(|rect| rect.width() > notes[1].width())
            .expect("a card");
        assert!(near(card.bottom(), notes[1].bottom() + room + 1.0));
        let color = line_color(false);
        let lines = shapes.iter().filter(|shape| {
            matches!(shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == color)
        });
        assert_eq!(lines.count(), 0, "no line next to a note");
        let text = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) if text.galley.text() == "A note." => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("the note's text");
        let past_icon = space::MEDIUM + NOTE_ICON + space::SMALL;
        let off = if i18n::rtl() {
            notes[0].right() - past_icon - text.right()
        } else {
            text.left() - notes[0].left() - past_icon
        };
        assert!(off.abs() <= 0.5, "{text:?} in {:?}", notes[0]);
    }

    #[test]
    fn notes_sit_in_their_card() {
        check_notes_in_a_card();
    }

    #[test]
    fn notes_sit_in_their_card_right_to_left() {
        passes_in_arabic("notes_sit_in_their_card_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by notes_sit_in_their_card_right_to_left"]
    fn notes_sit_in_their_card_in_arabic() {
        if arabic_run() {
            check_notes_in_a_card();
        }
    }
}
