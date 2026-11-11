//! The top bar: search field, search mode, filters, sort and tags.

use super::*;

impl App {
    pub(super) fn top_bar(&mut self, ui: &mut Ui) {
        let fill = ui.visuals().panel_fill;
        egui::Panel::top("top")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 16,
                        right: 16,
                        top: 12,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    // The search field takes the room the buttons at the far
                    // end used last time, less its outline (up to 1.5 px on
                    // each side). When their width changes, a second pass of
                    // the same frame lays the bar out again.
                    let gap = ui.spacing().item_spacing.x;
                    let w = (ui.available_width() - self.top_actions_w - gap - 3.0).max(200.0);
                    self.search_field(ui, w);
                    let used = far(ui, |ui| {
                        // On the button background of the controls beside it.
                        let fill = Some(ui.visuals().widgets.inactive.weak_bg_fill);
                        let size = Vec2::splat(34.0);
                        let name = t!("top-settings");
                        if icons::icon_button_ex(ui, Icon::Settings, &name, size, 16.0, fill, None)
                            .clicked()
                        {
                            self.open_settings(SettingsPage::General);
                        }
                        // Add with its word, so it is not a bare icon.
                        let import = word_button(ui, Icon::Add, &t!("top-add"));
                        let import = tip(import, &t!("top-add-files-tip"));
                        egui::Popup::menu(&import).show(|ui| {
                            ui.set_width(MENU_W);
                            if icons::menu_item(ui, Icon::File, &t!("top-add-files")).clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_files() {
                                    self.run(Command::Import(f));
                                }
                                self.after_dialog();
                            }
                            if icons::menu_item(ui, Icon::Folder, &t!("top-add-folder")).clicked() {
                                if let Some(f) = rfd::FileDialog::new().pick_folder() {
                                    self.run(Command::Import(vec![f]));
                                }
                                self.after_dialog();
                            }
                        });
                        self.view_switch(ui);
                        ui.min_rect().width()
                    });
                    if (used - self.top_actions_w).abs() > 0.5 {
                        self.top_actions_w = used;
                        ui.ctx()
                            .request_discard("the top bar's buttons changed width");
                    }
                });
                ui.add_space(space::TINY);
                self.filter_chips(ui);
            });
    }

    /// The layout controls: Grid or List, and the details pane's button.
    /// The view key still steps through Grid, List and Detail.
    fn view_switch(&mut self, ui: &mut Ui) {
        let open = self.pane_shown();
        let shown = match layout_controls(ui, self.cfg.view, open, &self.cfg.keys) {
            Some(LayoutClick::Grid) => layout_chosen(true, open),
            Some(LayoutClick::List) => layout_chosen(false, open),
            Some(LayoutClick::Pane) => pane_clicked(self.cfg.view, open),
            None => return,
        };
        self.show_layout(shown);
    }

    pub(super) fn search_field(&mut self, ui: &mut Ui, width: f32) {
        let focused_before = ui
            .memory(|m| m.focused())
            .is_some_and(|id| id == search_id());
        let bg = theme::search_fill(ui.visuals().dark_mode);
        let out = Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(radius::MEDIUM))
            // A thin edge while it is not in use, so it still reads as a field.
            .stroke(if focused_before {
                Stroke::new(1.5, accent())
            } else {
                Stroke::new(1.0, line_color(ui.visuals().dark_mode))
            })
            .inner_margin(if i18n::rtl() {
                Margin {
                    left: 6,
                    right: 12,
                    top: 0,
                    bottom: 0,
                }
            } else {
                Margin {
                    left: 12,
                    right: 6,
                    top: 0,
                    bottom: 0,
                }
            })
            .show(ui, |ui| {
                ui.set_width(width - 18.0);
                ui.set_height(40.0);
                ui.with_layout(row_layout(), |ui| {
                    icons::glyph(ui, Icon::Search, 16.0, ui.visuals().weak_text_color());
                    let hint = match &self.backend {
                        BackendState::Ready { images: true, .. } => t!("search-hint-images"),
                        BackendState::Ready { .. } => t!("search-hint-meaning"),
                        _ => t!("search-hint"),
                    };
                    let in_popup = ui.ctx().viewport_id() != egui::ViewportId::ROOT;
                    // The popup is too narrow for the long hint; screen
                    // readers still read it.
                    let shown_hint = if in_popup {
                        t!("search-hint-short")
                    } else {
                        hint.clone()
                    };
                    // Arabic is shown in reading order (the cursor still follows typing order).
                    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let color = ui.visuals().text_color();
                        let job = if bidi::has_rtl(text.as_str()) {
                            bidi::job(text.as_str(), FontId::proportional(size::LARGE), color)
                        } else {
                            LayoutJob::simple_singleline(
                                text.as_str().to_string(),
                                FontId::proportional(size::LARGE),
                                color,
                            )
                        };
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let te = egui::TextEdit::singleline(&mut self.query)
                        .id(search_id())
                        .frame(Frame::NONE)
                        .hint_text(txt(
                            &shown_hint,
                            size::LARGE,
                            Some(ui.visuals().weak_text_color()),
                        ))
                        .font(FontId::proportional(size::LARGE))
                        .align(if i18n::rtl() {
                            Align2::RIGHT_CENTER
                        } else {
                            Align2::LEFT_CENTER
                        })
                        .layouter(&mut layouter)
                        .desired_width((ui.available_width() - 130.0).max(80.0));
                    let resp = ui.add(te);
                    set_name(&resp, hint.trim_end_matches('…'));
                    // While the popup is open, its search field takes the
                    // keyboard, not the full window's: the popup's first
                    // frame may come after the full window's.
                    if (in_popup || !self.win.is_popup_open())
                        && self.focus_search.swap(false, Ordering::SeqCst)
                    {
                        resp.request_focus();
                    }
                    if resp.changed() {
                        self.query_changed = Some(Instant::now());
                        self.dirty = true;
                        // New words: the selection goes to their best match.
                        self.auto_selected = self.selected;
                    }
                    far(ui, |ui| {
                        // Search mode, quiet until needed.
                        let mode = self.cfg.search_mode;
                        let non_default = mode != SearchMode::Hybrid;
                        let b = mode_button(ui, &mode_label(mode), non_default);
                        egui::Popup::menu(&b).show(|ui| {
                            ui.set_width(MODE_MENU_W);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for m in [
                                SearchMode::Hybrid,
                                SearchMode::Semantic,
                                SearchMode::Keyword,
                            ] {
                                if mode_option(ui, m, mode == m).clicked() {
                                    self.cfg.search_mode = m;
                                    self.save_config();
                                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                                    self.dirty = true;
                                }
                            }
                        });
                        if !self.query.is_empty()
                            && icon_button_ex(
                                ui,
                                Icon::Close,
                                &t!("search-clear"),
                                Vec2::splat(26.0),
                                11.0,
                                None,
                                None,
                            )
                            .clicked()
                        {
                            self.query.clear();
                            self.query_vec = None;
                            self.dirty = true;
                        }
                    });
                });
            });
        let _ = out;
    }

    pub(super) fn filter_chips(&mut self, ui: &mut Ui) {
        const DIMMED: f32 = 0.45;
        row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for f in Filter::ALL {
                let total = self.library.items().iter().filter(|i| f.accepts(i)).count();
                let selected = self.filter == f;
                // Empty categories add noise; keep them out unless selected.
                if total == 0 && f != Filter::All && !selected {
                    continue;
                }
                // While searching, a filter counts the results it would show,
                // and one with none is dimmed (18.3).
                let folded = self.weaker_folded();
                let n = self
                    .search_counts
                    .as_ref()
                    .map_or(total, |c| c.kind(f, folded));
                let dim = self.search_counts.is_some() && n == 0 && !selected;
                let (fg, weak) = if selected {
                    let shade = text_shade(accent(), ui.visuals().dark_mode);
                    (shade, shade)
                } else {
                    (ui.visuals().text_color(), ui.visuals().weak_text_color())
                };
                let job = pill_job(&f.label(), size::CONTROL, fg, Some((n.to_string(), weak)));
                let pill = ui
                    .scope(|ui| {
                        if dim {
                            ui.multiply_opacity(DIMMED);
                        }
                        icons::pill(ui, filter_icon(f), job, accent(), selected)
                    })
                    .inner;
                if pill.clicked() {
                    self.filter = f;
                    self.dirty = true;
                }
            }
            self.tag_chips(ui);
            far(ui, |ui| {
                ui.add_space(space::TINY);
                self.sort_menu(ui);
                if !self.query.trim().is_empty() {
                    let text = t!("top-results", count = self.results.len());
                    ui.label(txt(&text, size::SMALL, Some(ui.visuals().text_color())));
                }
            });
        });
    }

    /// "Sort by" button and menu. Best match is offered only while searching.
    pub(super) fn sort_menu(&mut self, ui: &mut Ui) {
        let current = self.sort();
        let b = tip(mode_button(ui, &current.label(), false), &t!("top-sort-by"));
        let searching = !self.query.trim().is_empty();
        let mut pick = None;
        egui::Popup::menu(&b).show(|ui| {
            ui.set_width(MENU_W);
            let weak = ui.visuals().weak_text_color();
            row(ui, |ui| {
                ui.add_space(space::MEDIUM);
                lbl(ui, &t!("top-sort-by-heading"), size::SMALL, weak);
            });
            for s in SortOrder::ALL {
                if s == SortOrder::BestMatch && !searching {
                    continue;
                }
                if check_row(ui, &s.label(), s == current).clicked() {
                    pick = Some(s);
                }
            }
        });
        if let Some(s) = pick {
            self.set_sort(s);
        }
    }

    /// The most used tags as filters after the kind chips; the rest in a menu.
    pub(super) fn tag_chips(&mut self, ui: &mut Ui) {
        const DIMMED: f32 = 0.45;
        let tags = self.library.all_tags();
        if tags.is_empty() {
            return;
        }
        ui.add_space(space::SMALL);
        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
        ui.painter().rect_filled(
            sep_rect,
            0.0,
            ui.visuals().weak_text_color().gamma_multiply(0.3),
        );
        ui.add_space(space::SMALL);
        const SHOWN: usize = 6;
        let mut clicked: Option<String> = None;
        let mut delete: Option<String> = None;
        let active = self.tag_filter.clone();
        let is_active = |t: &str| active.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(t));
        // The active tag always shows, even when it isn't among the most used.
        let mut shown: Vec<&(String, usize)> = tags.iter().take(SHOWN).collect();
        if let Some(a) = tags.iter().skip(SHOWN).find(|(t, _)| is_active(t)) {
            shown.push(a);
        }
        for (t, all) in &shown {
            let sel = is_active(t);
            // While searching, the tag counts the results it would show.
            let folded = self.weaker_folded();
            let n = self
                .search_counts
                .as_ref()
                .map_or(*all, |c| c.tag(t, folded));
            let dim = self.search_counts.is_some() && n == 0 && !sel;
            let c = tag_color(t);
            let shade = tag_shade(c, ui.visuals().dark_mode);
            let job = pill_job(
                t,
                size::CONTROL,
                if sel {
                    shade
                } else {
                    ui.visuals().text_color()
                },
                Some((
                    n.to_string(),
                    if sel {
                        shade
                    } else {
                        ui.visuals().weak_text_color()
                    },
                )),
            );
            let pill = ui
                .scope(|ui| {
                    if dim {
                        ui.multiply_opacity(DIMMED);
                    }
                    icons::pill(ui, Icon::Tag, job, c, sel)
                })
                .inner;
            let r = tip(
                pill,
                &if sel {
                    t!("tags-show-all")
                } else {
                    t!("tags-filter-tip")
                },
            );
            if r.clicked() {
                clicked = Some(t.clone());
            }
            r.context_menu(|ui| {
                if ui
                    .button(txt(
                        &t!("tags-remove-everywhere", tag = t.as_str(), count = *all),
                        size::BODY,
                        None,
                    ))
                    .clicked()
                {
                    delete = Some(t.clone());
                }
            });
        }
        let rest: Vec<&(String, usize)> = tags
            .iter()
            .skip(SHOWN)
            .filter(|(t, _)| !is_active(t))
            .collect();
        if !rest.is_empty() {
            let more = mode_button(ui, &t!("tags-more", count = rest.len()), false);
            egui::Popup::menu(&more).show(|ui| {
                ui.set_width(MENU_W);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (t, n) in rest {
                            if icons::menu_item(ui, Icon::Tag, &format!("{t}   {n}")).clicked() {
                                clicked = Some(t.clone());
                            }
                        }
                    });
            });
        }
        if let Some(t) = clicked {
            self.tag_filter = if is_active(&t) { None } else { Some(t) };
            self.dirty = true;
        }
        if let Some(t) = delete {
            self.run(Command::DeleteTag(t));
        }
    }
}

/// What was clicked in the layout controls.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LayoutClick {
    Grid,
    List,
    /// The details pane's button.
    Pane,
}

/// What the layout controls light for `view` with the details pane `open`
/// beside it or not: Grid or List in the switch (List also for Detail, the
/// list with its pane), and whether the pane's button is lit.
fn layout_lit(view: ViewMode, open: bool) -> (ViewMode, bool) {
    match view {
        ViewMode::Grid => (ViewMode::Grid, open),
        ViewMode::List => (ViewMode::List, open),
        ViewMode::Detail => (ViewMode::List, true),
    }
}

/// Grid or the list chosen in the switch, from a view with the details
/// pane `open` or not: the pane stays as it is, and the list with its pane
/// is Detail. The view to show, and whether the pane is open beside it.
fn layout_chosen(grid: bool, open: bool) -> (ViewMode, bool) {
    match (grid, open) {
        (true, open) => (ViewMode::Grid, open),
        (false, true) => (ViewMode::Detail, true),
        (false, false) => (ViewMode::List, false),
    }
}

/// The details pane's button clicked in `view`, with the pane `open` or
/// not: with the list it switches between List and Detail; with the grid,
/// and with the pane Open details put beside the list, it opens or closes
/// the pane.
fn pane_clicked(view: ViewMode, open: bool) -> (ViewMode, bool) {
    match (view, open) {
        (ViewMode::Detail, _) => (ViewMode::List, false),
        (ViewMode::List, true) => (ViewMode::List, false),
        (ViewMode::List, false) => (ViewMode::Detail, true),
        (ViewMode::Grid, open) => (ViewMode::Grid, !open),
    }
}

/// How big a layout button is, and the room its track keeps around it.
const LAYOUT_BUTTON: f32 = 30.0;
const LAYOUT_PAD: f32 = 2.0;

/// Grid and List on one track in reading order, the one shown lit, and
/// after a small gap the details pane's button on a track of its own, lit
/// while the pane is open. Each tip names its key: the view key for Grid
/// and List, the key that opens details for the pane. In Arabic the pane's
/// icon is mirrored, as the pane opens on the left. The click, if any; a
/// click on the lit part of the switch is none.
fn layout_controls(
    ui: &mut Ui,
    view: ViewMode,
    open: bool,
    keys: &crate::config::Keys,
) -> Option<LayoutClick> {
    let (switch_w, pane_w) = (
        2.0 * LAYOUT_BUTTON + 2.0 * LAYOUT_PAD,
        LAYOUT_BUTTON + 2.0 * LAYOUT_PAD,
    );
    let height = LAYOUT_BUTTON + 2.0 * LAYOUT_PAD;
    let (whole, _) = ui.allocate_exact_size(
        Vec2::new(switch_w + space::TINY + pane_w, height),
        Sense::hover(),
    );
    // Laid out from the left, then mirrored for Arabic.
    let part = |x: f32, width: f32| {
        flip(
            whole,
            Rect::from_min_size(
                Pos2::new(whole.left() + x, whole.top()),
                Vec2::new(width, height),
            ),
        )
    };
    let (switch, pane) = (part(0.0, switch_w), part(switch_w + space::TINY, pane_w));
    for track in [switch, pane] {
        ui.painter().rect_filled(
            track,
            CornerRadius::same(radius::MEDIUM),
            ui.visuals().widgets.inactive.weak_bg_fill,
        );
    }
    let (lit_view, pane_lit) = layout_lit(view, open);
    let mut clicked = None;
    for (i, (mode, click)) in [
        (ViewMode::Grid, LayoutClick::Grid),
        (ViewMode::List, LayoutClick::List),
    ]
    .into_iter()
    .enumerate()
    {
        let at = Rect::from_min_size(
            switch.min + Vec2::new(LAYOUT_PAD + i as f32 * LAYOUT_BUTTON, LAYOUT_PAD),
            Vec2::splat(LAYOUT_BUTTON),
        );
        let button = LayoutButton {
            id: egui::Id::unique(("view", i)),
            // A part of a choice of one.
            role: egui::Role::RadioButton,
            name: hint(&mode.label(), &keys.view),
            icon: view_icon(mode),
            lit: lit_view == mode,
        };
        if button.show(ui, flip(switch, at)).clicked() && !button.lit {
            clicked = Some(click);
        }
    }
    let button = LayoutButton {
        id: egui::Id::unique("details-pane-button"),
        // A button that stays pressed while the pane is open.
        role: egui::Role::Button,
        name: hint(&t!("top-details-pane"), &keys.details),
        // Detail's icon, which `icons::paint` mirrors in Arabic.
        icon: Icon::Detail,
        lit: pane_lit,
    };
    if button.show(ui, pane.shrink(LAYOUT_PAD)).clicked() {
        clicked = Some(LayoutClick::Pane);
    }
    clicked
}

/// One button of the layout controls.
struct LayoutButton {
    id: egui::Id,
    /// What a screen reader calls it; it also hears whether it is lit.
    role: egui::Role,
    /// Its name and its tip.
    name: String,
    icon: Icon,
    lit: bool,
}

impl LayoutButton {
    /// Draws the button in `rect`: tinted while lit, lighter under the
    /// pointer, its icon in the middle.
    fn show(&self, ui: &Ui, rect: Rect) -> egui::Response {
        let resp = ui.interact(rect, self.id, Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(self.role, true, self.lit, &self.name));
        let v = ui.visuals();
        let round = CornerRadius::same(radius::MEDIUM);
        if self.lit {
            // Stronger on a dark track, so the lit button stands out.
            let tint = accent().gamma_multiply(if v.dark_mode { 0.34 } else { 0.18 });
            ui.painter().rect_filled(rect, round, tint);
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, round, v.widgets.hovered.weak_bg_fill);
        }
        let color = match (self.lit, resp.hovered()) {
            (true, _) => text_shade(accent(), v.dark_mode),
            (false, true) => v.strong_text_color(),
            (false, false) => v.text_color(),
        };
        let icon = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
        icons::paint(ui.painter(), icon, self.icon, color);
        tip(resp, &self.name)
    }
}

/// A flat button with an icon and its word.
fn word_button(ui: &mut Ui, icon: Icon, word: &str) -> egui::Response {
    let label = txt(word, size::CONTROL, None);
    let (galley, inner) = icons::icon_text_size(ui, &label, size::CONTROL, 16.0);
    let pad = 10.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(inner.x + 2.0 * pad, 34.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
    let v = ui.visuals();
    let p = ui.painter();
    // The view switcher's background, so it reads as a button.
    let fill = if resp.hovered() {
        v.widgets.hovered.weak_bg_fill
    } else {
        v.widgets.inactive.weak_bg_fill
    };
    p.rect_filled(rect, CornerRadius::same(radius::MEDIUM), fill);
    icons::paint_icon_text(
        p,
        rect.shrink2(Vec2::new(pad, 0.0)),
        icon,
        16.0,
        galley,
        v.text_color(),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Role, Toggled};

    #[test]
    fn the_switch_lights_grid_or_list_and_the_button_the_pane() {
        use ViewMode::*;
        // Grid, Grid with the pane, List, List with the pane Open details
        // put beside it, and Detail, the list with its pane.
        assert_eq!(layout_lit(Grid, false), (Grid, false));
        assert_eq!(layout_lit(Grid, true), (Grid, true));
        assert_eq!(layout_lit(List, false), (List, false));
        assert_eq!(layout_lit(List, true), (List, true));
        assert_eq!(layout_lit(Detail, true), (List, true));
    }

    #[test]
    fn the_pane_button_opens_and_closes_the_pane() {
        use ViewMode::*;
        assert_eq!(pane_clicked(Grid, false), (Grid, true));
        assert_eq!(pane_clicked(Grid, true), (Grid, false));
        assert_eq!(pane_clicked(List, false), (Detail, true));
        assert_eq!(pane_clicked(Detail, true), (List, false));
        // The pane Open details put beside the list closes.
        assert_eq!(pane_clicked(List, true), (List, false));
    }

    #[test]
    fn grid_or_list_keeps_the_pane_as_it_is() {
        use ViewMode::*;
        // Detail to Grid: the grid with the pane open.
        assert_eq!(layout_chosen(true, true), (Grid, true));
        // The grid with the pane to List: Detail.
        assert_eq!(layout_chosen(false, true), (Detail, true));
        assert_eq!(layout_chosen(false, false), (List, false));
        assert_eq!(layout_chosen(true, false), (Grid, false));
    }

    /// What one frame of the layout controls gave: the click, the screen
    /// reader's tree and the shapes drawn.
    type Frame = (
        Option<LayoutClick>,
        egui::accesskit::TreeUpdate,
        Vec<egui::Shape>,
    );

    /// One frame of the layout controls for `view` with the pane `open`.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, view: ViewMode, open: bool) -> Frame {
        let input = egui::RawInput {
            events,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 100.0))),
            ..Default::default()
        };
        let keys = crate::config::Keys::default();
        let mut clicked = None;
        let mut output = ctx.run_ui(input, |ui| {
            clicked = layout_controls(ui, view, open, &keys);
        });
        // The texture uploads no renderer takes here.
        output.textures_delta.clear();
        let tree = output.platform_output.accesskit_update.expect("a tree");
        let shapes = output.shapes.into_iter().map(|c| c.shape).collect();
        (clicked, tree, shapes)
    }

    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        icons::register(&ctx);
        ctx
    }

    /// The buttons a screen reader finds: role, name, whether it is
    /// pressed or chosen, and where it is.
    fn buttons(tree: &egui::accesskit::TreeUpdate) -> Vec<(Role, String, bool, Rect)> {
        tree.nodes
            .iter()
            .filter(|(_, node)| matches!(node.role(), Role::Button | Role::RadioButton))
            .map(|(_, node)| {
                let b = node.bounds().expect("a place");
                let rect = Rect::from_min_max(
                    Pos2::new(b.x0 as f32, b.y0 as f32),
                    Pos2::new(b.x1 as f32, b.y1 as f32),
                );
                let pressed = node.toggled() == Some(Toggled::True);
                let name = node.label().unwrap_or("").to_string();
                (node.role(), name, pressed, rect)
            })
            .collect()
    }

    /// Grid and List are a choice of one, the pane's button a button that
    /// stays pressed while the pane is open; a click on that button from
    /// each layout reaches it.
    #[test]
    fn the_layout_controls_say_their_state_and_take_a_click() {
        let keys = crate::config::Keys::default();
        let pane_name = hint(&t!("top-details-pane"), &keys.details);
        for (view, open) in [
            (ViewMode::Grid, false),
            (ViewMode::Grid, true),
            (ViewMode::List, false),
            (ViewMode::Detail, true),
        ] {
            let ctx = context();
            let (_, tree, _) = frame(&ctx, Vec::new(), view, open);
            let found = buttons(&tree);
            let (lit_view, pane_lit) = layout_lit(view, open);
            let case = format!("{view:?}, pane open {open}: {found:?}");
            for mode in [ViewMode::Grid, ViewMode::List] {
                let name = hint(&mode.label(), &keys.view);
                let part = found.iter().find(|(_, n, ..)| *n == name).expect(&case);
                assert_eq!(part.0, Role::RadioButton, "{case}");
                assert_eq!(part.2, mode == lit_view, "{case}");
            }
            let pane = found.iter().find(|(_, n, ..)| *n == pane_name);
            let pane = pane.expect(&case);
            assert_eq!(pane.0, Role::Button, "{case}");
            assert_eq!(pane.2, pane_lit, "{case}");
            // A click: the pointer comes, presses and lets go.
            let at = pane.3.center();
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, vec![egui::Event::PointerMoved(at)], view, open);
            frame(&ctx, vec![button(true)], view, open);
            let (clicked, ..) = frame(&ctx, vec![button(false)], view, open);
            assert_eq!(clicked, Some(LayoutClick::Pane), "{case}");
        }
    }

    /// From the reading start: Grid, List, then the pane's button, whose
    /// icon alone is mirrored in Arabic, by its texture's coordinates.
    fn check_pane_icon() {
        let ctx = context();
        // The icons load in the first frame.
        frame(&ctx, Vec::new(), ViewMode::Grid, false);
        let (_, _, shapes) = frame(&ctx, Vec::new(), ViewMode::Grid, false);
        let mut icons: Vec<(f32, bool)> = shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                    // The texture's side at the icon's left edge: its right
                    // side when mirrored.
                    let left = mesh
                        .vertices
                        .iter()
                        .min_by(|a, b| a.pos.x.total_cmp(&b.pos.x))
                        .expect("a corner");
                    Some((mesh.calc_bounds().center().x, left.uv.x > 0.5))
                }
                _ => None,
            })
            .collect();
        assert_eq!(icons.len(), 3, "{icons:?}");
        icons.sort_by(|a, b| a.0.total_cmp(&b.0));
        let rtl = i18n::rtl();
        // In reading order: Grid, List, the pane's button.
        if rtl {
            icons.reverse();
        }
        let mirrored: Vec<bool> = icons.iter().map(|(_, mirrored)| *mirrored).collect();
        assert_eq!(mirrored, [false, false, rtl], "{icons:?}");
    }

    #[test]
    fn the_pane_icon_is_drawn_as_it_is() {
        check_pane_icon();
    }

    #[test]
    fn the_pane_icon_is_mirrored_right_to_left() {
        crate::i18n::passes_in_arabic(module_path!(), "the_pane_icon_is_mirrored_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by the_pane_icon_is_mirrored_right_to_left"]
    fn the_pane_icon_is_mirrored_in_arabic() {
        if crate::i18n::arabic_run() {
            check_pane_icon();
        }
    }
}
