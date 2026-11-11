//! Settings, Appearance: theme, accent, layout and density.

use super::*;

pub(super) fn appearance_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-look"), |ui| {
        setting_for(
            ui,
            &t!("settings-theme"),
            &t!("settings-theme-hint"),
            "",
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
            &t!("settings-dark-panels"),
            &t!("settings-dark-panels-hint"),
            "",
            &mut d.dark_panels,
            def.dark_panels,
            |ui, panels| {
                segmented(
                    ui,
                    panels,
                    &[
                        (DarkPanels::Darker, t!("settings-dark-panels-darker")),
                        (DarkPanels::Lighter, t!("settings-dark-panels-lighter")),
                    ],
                )
            },
        );
        setting_for(
            ui,
            &t!("settings-accent"),
            &t!("settings-accent-hint"),
            "",
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
    });
    section(ui, &t!("settings-list"), |ui| {
        setting_for(
            ui,
            &t!("settings-layout"),
            &t!("settings-layout-hint"),
            "",
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
            &t!("settings-density-hint"),
            "",
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
            &t!("settings-card-height"),
            &t!("settings-card-height-hint"),
            "",
            &mut d.card_height,
            def.card_height,
            |ui, height| {
                segmented(
                    ui,
                    height,
                    &[
                        (CardHeight::Same, t!("settings-card-height-same")),
                        (CardHeight::Fit, t!("settings-card-height-fit")),
                    ],
                )
            },
        );
        setting_for(
            ui,
            &t!("settings-pinned-on-top"),
            &t!("settings-pinned-on-top-hint"),
            "",
            &mut d.pinned_first,
            def.pinned_first,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-group-by-day"),
            &t!("settings-group-by-day-hint"),
            "",
            &mut d.group_by_date,
            def.group_by_date,
            toggle,
        );
    });
}
