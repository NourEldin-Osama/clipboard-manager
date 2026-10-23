//! Settings, Appearance: theme, accent, layout and density.

use super::*;

pub(super) fn appearance_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Look");
    setting_for(
        ui,
        "Theme",
        "System follows your Windows light/dark setting.",
        &mut d.theme,
        def.theme,
        |ui, theme| {
            segmented(
                ui,
                theme,
                &[
                    (Theme::System, "System"),
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                ],
            )
        },
    );
    setting_for(
        ui,
        "Accent colour",
        "Used for selection, buttons and highlights.",
        &mut d.accent,
        def.accent,
        |ui, chosen| {
            for (i, (name, c)) in ACCENTS.iter().enumerate().rev() {
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                ui.painter().circle_filled(rect.center(), 10.0, *c);
                if *chosen == i {
                    ui.painter()
                        .circle_stroke(rect.center(), 12.0, Stroke::new(2.0, *c));
                }
                if tip(r, tr(name)).clicked() {
                    *chosen = i;
                }
            }
        },
    );
    section(ui, "List");
    setting_for(ui, "Layout", "Grid shows cards, List shows rows; both open details on demand. Detail keeps the selected item beside the list. The view button next to the search box switches too.", &mut d.view, def.view, |ui, view| {
        let options: Vec<(ViewMode, &str)> = ViewMode::ALL.iter().map(|v| (*v, v.label())).collect();
        segmented(ui, view, &options)
    });
    setting_for(
        ui,
        "Density",
        "Compact fits more items on screen by hiding the second line.",
        &mut d.density,
        def.density,
        |ui, density| {
            segmented(
                ui,
                density,
                &[
                    (Density::Comfortable, "Comfortable"),
                    (Density::Compact, "Compact"),
                ],
            )
        },
    );
    setting_for(ui, "Pinned on top", "Pinned items come first while you browse, whatever the order. Search results are ordered by your sort choice only.", &mut d.pinned_first, def.pinned_first, toggle);
    setting_for(
        ui,
        "Group by day",
        "Show Pinned, Today, Yesterday… headers while browsing (not while searching).",
        &mut d.group_by_date,
        def.group_by_date,
        toggle,
    );
}
