//! Settings, Appearance: theme, accent, layout and density.

use super::*;

pub(super) fn appearance_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-look"));
    setting_for(
        ui,
        &t!("settings-theme"),
        &t!("settings-theme-help"),
        &mut d.theme,
        def.theme,
        |ui, theme| {
            segmented(
                ui,
                theme,
                &[
                    (Theme::System, t!("settings-theme-system")),
                    (Theme::Light, t!("settings-theme-light")),
                    (Theme::Dark, t!("settings-theme-dark")),
                ],
            )
        },
    );
    setting_for(
        ui,
        &t!("settings-accent"),
        &t!("settings-accent-help"),
        &mut d.accent,
        def.accent,
        |ui, chosen| {
            for (i, c) in ACCENTS.iter().enumerate().rev() {
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                ui.painter().circle_filled(rect.center(), 10.0, *c);
                if *chosen == i {
                    ui.painter()
                        .circle_stroke(rect.center(), 12.0, Stroke::new(2.0, *c));
                }
                if tip(r, &accent_name(i)).clicked() {
                    *chosen = i;
                }
            }
        },
    );
    section(ui, &t!("settings-list"));
    setting_for(
        ui,
        &t!("settings-layout"),
        &t!("settings-layout-help"),
        &mut d.view,
        def.view,
        |ui, view| {
            let options: Vec<(ViewMode, String)> =
                ViewMode::ALL.iter().map(|v| (*v, v.label())).collect();
            segmented(ui, view, &options)
        },
    );
    setting_for(
        ui,
        &t!("settings-density"),
        &t!("settings-density-help"),
        &mut d.density,
        def.density,
        |ui, density| {
            segmented(
                ui,
                density,
                &[
                    (Density::Comfortable, t!("settings-density-comfortable")),
                    (Density::Compact, t!("settings-density-compact")),
                ],
            )
        },
    );
    setting_for(
        ui,
        &t!("settings-pinned-on-top"),
        &t!("settings-pinned-on-top-help"),
        &mut d.pinned_first,
        def.pinned_first,
        toggle,
    );
    setting_for(
        ui,
        &t!("settings-group-by-day"),
        &t!("settings-group-by-day-help"),
        &mut d.group_by_date,
        def.group_by_date,
        toggle,
    );
}
