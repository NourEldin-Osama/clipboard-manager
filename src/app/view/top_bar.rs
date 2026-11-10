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
                        if icons::icon_button(ui, Icon::Settings, &t!("top-settings")).clicked() {
                            self.open_settings(SettingsPage::General);
                        }
                        let import = icons::icon_button(ui, Icon::Add, &t!("top-add-files-tip"));
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
                        ui.add_space(space::SMALL);
                        self.pause_button(ui);
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

    /// Grid, List and Detail side by side in reading order, the current one
    /// tinted. The view key still steps through them.
    fn view_switch(&mut self, ui: &mut Ui) {
        const BUTTON: f32 = 30.0;
        const PAD: f32 = 2.0;
        let count = ViewMode::ALL.len() as f32;
        let (track, _) = ui.allocate_exact_size(
            Vec2::new(count * BUTTON + 2.0 * PAD, BUTTON + 2.0 * PAD),
            Sense::hover(),
        );
        let v = ui.visuals();
        ui.painter().rect_filled(
            track,
            CornerRadius::same(radius::MEDIUM),
            v.widgets.inactive.weak_bg_fill,
        );
        let tint = accent();
        let shade = text_shade(tint, v.dark_mode);
        let (text, strong, hovered) = (
            v.text_color(),
            v.strong_text_color(),
            v.widgets.hovered.weak_bg_fill,
        );
        for (i, mode) in ViewMode::ALL.into_iter().enumerate() {
            let at = Rect::from_min_size(
                track.min + Vec2::new(PAD + i as f32 * BUTTON, PAD),
                Vec2::splat(BUTTON),
            );
            let rect = flip(track, at);
            let name = hint(&mode.label(), &self.cfg.keys.view);
            let current = self.cfg.view == mode;
            let resp = ui.interact(rect, egui::Id::unique(("view", i)), Sense::click());
            resp.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::RadioButton, true, current, &name)
            });
            let p = ui.painter();
            if current {
                p.rect_filled(
                    rect,
                    CornerRadius::same(radius::MEDIUM),
                    tint.gamma_multiply(0.18),
                );
            } else if resp.hovered() {
                p.rect_filled(rect, CornerRadius::same(radius::MEDIUM), hovered);
            }
            let color = match (current, resp.hovered()) {
                (true, _) => shade,
                (false, true) => strong,
                (false, false) => text,
            };
            icons::paint(
                p,
                Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
                view_icon(mode),
                color,
            );
            if tip(resp, &name).clicked() && !current {
                self.set_view(mode);
            }
        }
    }

    /// Pause, with its word; Resume, in amber, while recording is paused.
    fn pause_button(&mut self, ui: &mut Ui) {
        let recording = self.flags.enabled.load(Ordering::SeqCst);
        let (icon, word, name) = if recording {
            (Icon::Pause, t!("top-pause"), t!("top-pause-recording"))
        } else {
            (Icon::Play, t!("top-resume"), t!("top-resume-recording"))
        };
        let label = txt(&word, size::CONTROL, None);
        let (galley, inner) = icons::icon_text_size(ui, &label, size::CONTROL, 16.0);
        let pad = 10.0;
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(inner.x + 2.0 * pad, 34.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        let v = ui.visuals();
        let p = ui.painter();
        let color = if recording {
            if resp.hovered() {
                p.rect_filled(
                    rect,
                    CornerRadius::same(radius::MEDIUM),
                    v.widgets.hovered.weak_bg_fill,
                );
            }
            v.text_color()
        } else {
            p.rect_filled(rect, CornerRadius::same(radius::MEDIUM), button_fill(AMBER));
            if resp.hovered() {
                p.rect_filled(rect, CornerRadius::same(radius::MEDIUM), theme::lit(false));
            }
            Color32::WHITE
        };
        icons::paint_icon_text(
            p,
            rect.shrink2(Vec2::new(pad, 0.0)),
            icon,
            16.0,
            galley,
            color,
        );
        if tip(resp, &name).clicked() {
            self.set_recording(!recording);
        }
    }

    pub(super) fn search_field(&mut self, ui: &mut Ui, width: f32) {
        let focused_before = ui
            .memory(|m| m.focused())
            .is_some_and(|id| id == search_id());
        let bg = theme::search_fill(ui.visuals().dark_mode);
        let out = Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(radius::MEDIUM))
            .stroke(if focused_before {
                Stroke::new(1.5, accent())
            } else {
                Stroke::new(
                    1.0,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .bg_stroke
                        .color
                        .gamma_multiply(0.0),
                )
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
                    ui.label(txt(
                        &text,
                        size::SMALL,
                        Some(ui.visuals().weak_text_color()),
                    ));
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
            let shade = text_shade(c, ui.visuals().dark_mode);
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
