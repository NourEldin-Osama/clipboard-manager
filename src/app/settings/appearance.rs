//! Settings, Appearance: theme, accent, layout and density.

use super::*;

pub(super) fn appearance_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Look");
    if setting(
        ui,
        "Theme",
        "System follows your Windows light/dark setting.",
        d.theme != def.theme,
        |ui| {
            segmented(
                ui,
                &mut d.theme,
                &[
                    (Theme::System, "System"),
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                ],
            );
        },
    ) {
        d.theme = def.theme;
    }
    if setting(
        ui,
        "Accent colour",
        "Used for selection, buttons and highlights.",
        d.accent != def.accent,
        |ui| {
            for (i, (name, c)) in ACCENTS.iter().enumerate().rev() {
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                ui.painter().circle_filled(rect.center(), 10.0, *c);
                if d.accent == i {
                    ui.painter()
                        .circle_stroke(rect.center(), 12.0, Stroke::new(2.0, *c));
                }
                if tip(r, tr(name)).clicked() {
                    d.accent = i;
                }
            }
        },
    ) {
        d.accent = def.accent;
    }
    section(ui, "List");
    if setting(ui, "Layout", "Grid shows cards, List shows rows; both open details on demand. Detail keeps the selected item beside the list. The view button next to the search box switches too.", d.view != def.view, |ui| {
        let options: Vec<(ViewMode, &str)> = ViewMode::ALL.iter().map(|v| (*v, v.label())).collect();
        segmented(ui, &mut d.view, &options);
    }) {
        d.view = def.view;
    }
    if setting(
        ui,
        "Density",
        "Compact fits more items on screen by hiding the second line.",
        d.density != def.density,
        |ui| {
            segmented(
                ui,
                &mut d.density,
                &[
                    (Density::Comfortable, "Comfortable"),
                    (Density::Compact, "Compact"),
                ],
            );
        },
    ) {
        d.density = def.density;
    }
    if setting(ui, "Pinned on top", "Pinned items come first while you browse, whatever the order. Search results are ordered by your sort choice only.", d.pinned_first != def.pinned_first, |ui| {
        toggle(ui, &mut d.pinned_first);
    }) {
        d.pinned_first = def.pinned_first;
    }
    if setting(
        ui,
        "Group by day",
        "Show Pinned, Today, Yesterday… headers while browsing (not while searching).",
        d.group_by_date != def.group_by_date,
        |ui| {
            toggle(ui, &mut d.group_by_date);
        },
    ) {
        d.group_by_date = def.group_by_date;
    }
}
