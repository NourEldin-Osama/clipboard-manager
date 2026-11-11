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

/// The info button beside a setting's name: a click, or Enter or Space,
/// opens its details; screen readers read them as the button's name.
pub(super) fn info(ui: &mut Ui, text: &str) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
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
pub(super) const RESET_BUTTON_SIZE: f32 = 28.0;

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

/// What a setting's row holds, for a row that shows more under it: the
/// name with its info button and the control on one line, and the gray
/// line under them. True when the reset was clicked.
pub(super) fn setting_content(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    let mut reset = false;
    row(ui, |ui| {
        ui.set_min_height(30.0);
        let color = if changed {
            text_shade(accent(), ui.visuals().dark_mode)
        } else {
            ui.visuals().strong_text_color()
        };
        lbl(ui, label, size::BODY, color);
        if !details.is_empty() {
            info(ui, details);
        }
        far(ui, |ui| {
            // The reset button's space is kept when it's hidden, so controls stay in one column.
            if resettable {
                if icons::icon_button_ex(
                    ui,
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
            } else {
                // Keeps the row's control where it is when there is no reset.
                ui.add_space(RESET_BUTTON_SIZE + ui.spacing().item_spacing.x);
            }
            ui.ctx()
                .data_mut(|d| d.insert_temp(row_title_id(), label.to_string()));
            control(ui);
            ui.ctx().data_mut(|d| d.remove::<String>(row_title_id()));
        });
    });
    if !line.is_empty() {
        // Right under the name, as one block with it.
        let weak = ui.visuals().weak_text_color();
        super::bidi::paragraph(ui, line, FontId::proportional(size::SMALL), weak);
    }
    reset
}

/// A note about the settings above it in their card, marked by a thin bar
/// at its start so it reads as one note and not as a part of the row before
/// it, and not as a field or a button. It has no line above or below it.
pub(super) fn note(ui: &mut Ui, text: &str) {
    let rtl = i18n::rtl();
    let weak = ui.visuals().weak_text_color();
    let state = card_state(ui);
    // A row before it has its own room below.
    let above = match state.and_then(|s| s.last) {
        Some(Part::Row) => 0.0,
        Some(Part::Note) => space::SMALL,
        None => ROW_VERTICAL_PADDING,
    };
    ui.add_space(above);
    // The bar where the rows' text starts, the text a little apart from it.
    let start = (ROW_SIDE_PADDING + state.map_or(0.0, |s| s.indent)) as i8;
    let end = ROW_SIDE_PADDING as i8;
    let (left, right) = if rtl { (end, start) } else { (start, end) };
    let gap = 12;
    let (inner_left, inner_right) = if rtl { (0, gap) } else { (gap, 0) };
    let content = Frame::new()
        .inner_margin(Margin {
            left: inner_left,
            right: inner_right,
            top: 2,
            bottom: 2,
        })
        .outer_margin(Margin {
            left,
            right,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            super::bidi::paragraph(ui, text, FontId::proportional(size::SMALL), weak);
            ui.min_rect()
        })
        .inner
        .expand2(Vec2::new(0.0, 2.0));
    let x = if rtl {
        content.right() + f32::from(gap) - NOTE_BAR
    } else {
        content.left() - f32::from(gap)
    };
    let bar = Rect::from_min_size(
        Pos2::new(x, content.top()),
        Vec2::new(NOTE_BAR, content.height()),
    );
    ui.painter().rect_filled(
        bar,
        CornerRadius::same(radius::SMALL),
        weak.gamma_multiply(0.35),
    );
    if let Some(mut state) = state {
        state.last = Some(Part::Note);
        state.tint = None;
        set_card_state(ui, Some(state));
    }
}

/// How wide the bar at a note's start is.
const NOTE_BAR: f32 = 3.0;

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

    /// Set by `passes_in_arabic` for the process it starts.
    const ARABIC_RUN: &str = "CLIPBOARD_MANAGER_TEST_ARABIC";

    /// Runs the test `name` of this module again in a process of its own,
    /// with Arabic on, and fails if it fails there. The language is one
    /// setting for the whole program, and the tests running beside this one
    /// read it.
    fn passes_in_arabic(name: &str) {
        let module = module_path!().split_once("::").map_or("", |(_, path)| path);
        let test = format!("{module}::{name}");
        let output = std::process::Command::new(std::env::current_exe().expect("the tests"))
            .args([&test, "--exact", "--include-ignored", "--test-threads=1"])
            .env(ARABIC_RUN, "1")
            .output()
            .expect("the tests run");
        let said = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && said.contains("1 passed"),
            "{test} in Arabic:\n{said}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Turns Arabic on when this process was started by `passes_in_arabic`;
    /// false otherwise, and the test has nothing to do.
    fn arabic_run() -> bool {
        let started = std::env::var_os(ARABIC_RUN).is_some();
        if started {
            crate::i18n::set(Language::Arabic, TimeFormat::H24);
        }
        started
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    /// One frame of `draw` in the light theme with the app's fonts and
    /// style, and the shapes it painted.
    fn painted(mut draw: impl FnMut(&mut Ui)) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.set_theme(egui::Theme::Light);
        // The fonts take effect from the next frame. The font atlas uploads
        // no renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
        output.textures_delta.clear();
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
}
