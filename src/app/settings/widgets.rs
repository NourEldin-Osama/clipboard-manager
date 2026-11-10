//! The building blocks of a settings page: rows, switches, segmented
//! choices, sections and notes.

use super::*;

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
    let (track, face) = choice_track(ui.visuals().dark_mode);
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
                let b = egui::Button::new(text)
                    .frame_when_inactive(sel)
                    .corner_radius(CornerRadius::same(radius::SMALL))
                    .fill(if sel { face } else { Color32::TRANSPARENT });
                if ui.add(b).clicked() && !sel {
                    *value = *v;
                    changed = true;
                }
            }
        });
    changed
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

/// The reset button beside a changed setting.
pub(super) const RESET_BUTTON_SIZE: f32 = 28.0;

pub(super) fn section(ui: &mut Ui, title: &str) {
    ui.add_space(space::LARGE);
    let weak = ui.visuals().weak_text_color();
    row(ui, |ui| {
        ui.label(section_title(&title.to_uppercase(), weak))
    });
    ui.add_space(space::TINY);
}

/// One setting: its name, a short gray line under it and an info button
/// with the details, and the control at the far edge. A value changed from
/// its default tints the row and offers a reset; the caller must act on the
/// returned click. An empty line or details are left out.
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

pub(super) fn setting_row(
    ui: &mut Ui,
    label: &str,
    line: &str,
    details: &str,
    changed: bool,
    resettable: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    let mut reset = false;
    let fill = if changed {
        accent().gamma_multiply(if ui.visuals().dark_mode { 0.14 } else { 0.07 })
    } else {
        Color32::TRANSPARENT
    };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
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
                ui.spacing_mut().item_spacing.y = 0.0;
                let weak = ui.visuals().weak_text_color();
                super::bidi::paragraph(ui, line, FontId::proportional(size::SMALL), weak);
            }
        });
    reset
}

pub(super) fn note(ui: &mut Ui, text: &str) {
    let weak = ui.visuals().weak_text_color();
    Frame::new()
        .inner_margin(Margin::symmetric(10, 0))
        .show(ui, |ui| {
            super::bidi::paragraph(ui, text, FontId::proportional(size::SMALL), weak);
        });
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
}
