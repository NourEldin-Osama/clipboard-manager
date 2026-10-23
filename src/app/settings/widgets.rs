//! The building blocks of a settings page: rows, switches, segmented
//! choices, sections and notes.

use super::*;

/// iOS-style switch.
pub(super) fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let off = ui.visuals().widgets.inactive.bg_fill;
    let bg = Color32::from_rgba_premultiplied(
        (off.r() as f32 + (accent().r() as f32 - off.r() as f32) * t) as u8,
        (off.g() as f32 + (accent().g() as f32 - off.g() as f32) * t) as u8,
        (off.b() as f32 + (accent().b() as f32 - off.b() as f32) * t) as u8,
        255,
    );
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(11), bg);
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

/// Options side by side; the chosen one is tinted. Called inside a setting's
/// control area, which fills from the far end, so options are added in reverse.
pub(super) fn segmented<T: PartialEq + Copy, L: AsRef<str>>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, L)],
) -> bool {
    let mut changed = false;
    ui.spacing_mut().item_spacing.x = 2.0;
    for (v, label) in options.iter().rev() {
        let sel = *value == *v;
        let text = txt(
            tr(label.as_ref()),
            13.0,
            Some(if sel {
                accent()
            } else {
                ui.visuals().text_color()
            }),
        );
        let b = egui::Button::new(text)
            .frame_when_inactive(sel)
            .fill(if sel {
                accent().gamma_multiply(0.14)
            } else {
                Color32::TRANSPARENT
            });
        if ui.add(b).clicked() && !sel {
            *value = *v;
            changed = true;
        }
    }
    changed
}

pub(super) fn info(ui: &mut Ui, text: &str) {
    tip(
        icons::glyph(ui, Icon::Info, 14.0, ui.visuals().weak_text_color()),
        tr(text),
    );
}

pub(super) fn section(ui: &mut Ui, title: &str) {
    ui.add_space(18.0);
    let weak = ui.visuals().weak_text_color();
    row(ui, |ui| lbl(ui, &tr(title).to_uppercase(), 11.5, weak));
    ui.add_space(4.0);
}

/// One setting: label and ℹ on the left, control at the right edge. A value
/// changed from its default tints the row and offers a reset; the caller must
/// act on the returned click.
#[must_use = "a changed row shows a reset button; act on its click"]
pub(super) fn setting(
    ui: &mut Ui,
    label: &str,
    help: &str,
    changed: bool,
    control: impl FnOnce(&mut Ui),
) -> bool {
    setting_row(ui, label, help, changed, changed, control)
}

/// A row for one value with a default: tinted while the value differs, and
/// "Back to default" puts the default back. The control edits the value.
pub(super) fn setting_for<T: PartialEq, R>(
    ui: &mut Ui,
    label: &str,
    help: &str,
    value: &mut T,
    default: T,
    control: impl FnOnce(&mut Ui, &mut T) -> R,
) {
    if setting(ui, label, help, *value != default, |ui| {
        control(ui, value);
    }) {
        *value = default;
    }
}

/// A row with nothing to reset: an action, a path, or a fixed value.
pub(super) fn plain_setting(ui: &mut Ui, label: &str, help: &str, control: impl FnOnce(&mut Ui)) {
    setting_row(ui, label, help, false, false, control);
}

/// A row that marks the current choice in a list (a built-in model): tinted,
/// but with no reset, since there's no default to go back to.
pub(super) fn choice_setting(
    ui: &mut Ui,
    label: &str,
    help: &str,
    chosen: bool,
    control: impl FnOnce(&mut Ui),
) {
    setting_row(ui, label, help, chosen, false, control);
}

pub(super) fn setting_row(
    ui: &mut Ui,
    label: &str,
    help: &str,
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
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            row(ui, |ui| {
                ui.set_min_height(30.0);
                let color = if changed {
                    accent()
                } else {
                    ui.visuals().strong_text_color()
                };
                lbl(ui, label, 14.0, color);
                if !help.is_empty() {
                    info(ui, help);
                }
                far(ui, |ui| {
                    // The reset button's space is kept when it's hidden, so controls stay in one column.
                    if resettable {
                        if icons::icon_button_ex(
                            ui,
                            Icon::Reset,
                            "Back to default",
                            Vec2::splat(28.0),
                            14.0,
                            None,
                            None,
                        )
                        .clicked()
                        {
                            reset = true;
                        }
                    } else {
                        ui.add_space(28.0 + ui.spacing().item_spacing.x);
                    }
                    control(ui);
                });
            });
        });
    reset
}

pub(super) fn note(ui: &mut Ui, text: &str) {
    let weak = ui.visuals().weak_text_color();
    Frame::new()
        .inner_margin(Margin::symmetric(10, 0))
        .show(ui, |ui| {
            super::bidi::paragraph(ui, tr(text), FontId::proportional(12.5), weak);
        });
}

pub(super) fn path_field(ui: &mut Ui, p: &mut PathBuf, ext: &[&str]) {
    if ui.button(txt(tr("Browse…"), 14.0, None)).clicked() {
        let mut dlg = rfd::FileDialog::new();
        if !ext.is_empty() {
            dlg = dlg.add_filter("file", ext).add_filter("all", &["*"]);
        }
        if let Some(f) = dlg.pick_file() {
            *p = f;
        }
    }
    let mut s = p.to_string_lossy().into_owned();
    if ui
        .add(egui::TextEdit::singleline(&mut s).desired_width(280.0))
        .changed()
    {
        *p = PathBuf::from(&s);
    }
}
