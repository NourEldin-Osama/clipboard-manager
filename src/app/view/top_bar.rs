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
                        right: 12,
                        top: 12,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    let actions_w = 4.0 * 40.0;
                    let w = (ui.available_width() - actions_w).max(200.0);
                    self.search_field(ui, w);
                    far(ui, |ui| {
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
                        let view = self.cfg.view;
                        let tip = t!(
                            "top-view-tip",
                            view = view.label(),
                            next = view.next().label()
                        );
                        if icons::icon_button(ui, view_icon(view), &tip).clicked() {
                            self.set_view(view.next());
                        }
                        let rec = self.flags.enabled.load(Ordering::SeqCst);
                        let (icon, tip) = if rec {
                            (Icon::Pause, t!("top-pause-recording"))
                        } else {
                            (Icon::Play, t!("top-resume-recording"))
                        };
                        let (fill, fg) = if rec {
                            (None, None)
                        } else {
                            (Some(AMBER), Some(Color32::WHITE))
                        };
                        if icon_button_ex(ui, icon, &tip, Vec2::splat(34.0), 16.0, fill, fg)
                            .clicked()
                        {
                            self.set_recording(!rec);
                        }
                    });
                });
                ui.add_space(4.0);
                self.filter_chips(ui);
            });
    }

    pub(super) fn search_field(&mut self, ui: &mut Ui, width: f32) {
        let focused_before = ui
            .memory(|m| m.focused())
            .is_some_and(|id| id == search_id());
        let bg = if ui.visuals().dark_mode {
            Color32::from_gray(22)
        } else {
            Color32::WHITE
        };
        let out = Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(10))
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
                    // Arabic is shown in reading order (the cursor still follows typing order).
                    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let color = ui.visuals().text_color();
                        let job = if bidi::has_rtl(text.as_str()) {
                            bidi::job(text.as_str(), FontId::proportional(16.0), color)
                        } else {
                            LayoutJob::simple_singleline(
                                text.as_str().to_string(),
                                FontId::proportional(16.0),
                                color,
                            )
                        };
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let te = egui::TextEdit::singleline(&mut self.query)
                        .id(search_id())
                        .frame(Frame::NONE)
                        .hint_text(txt(&hint, 16.0, Some(ui.visuals().weak_text_color())))
                        .font(FontId::proportional(16.0))
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
                    let in_popup = ui.ctx().viewport_id() != egui::ViewportId::ROOT;
                    if (in_popup || !self.win.is_popup_open())
                        && self.focus_search.swap(false, Ordering::SeqCst)
                    {
                        resp.request_focus();
                    }
                    if resp.changed() {
                        self.query_changed = Some(Instant::now());
                        self.dirty = true;
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
        row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for f in Filter::ALL {
                let n = self.library.items().iter().filter(|i| f.accepts(i)).count();
                let selected = self.filter == f;
                // Empty categories add noise; keep them out unless selected.
                if n == 0 && f != Filter::All && !selected {
                    continue;
                }
                let (fg, weak) = if selected {
                    (accent(), accent().gamma_multiply(0.75))
                } else {
                    (ui.visuals().text_color(), ui.visuals().weak_text_color())
                };
                let job = pill_job(&f.label(), 13.5, fg, Some((n.to_string(), weak)));
                if icons::pill(ui, filter_icon(f), job, accent(), selected).clicked() {
                    self.filter = f;
                    self.dirty = true;
                }
            }
            self.tag_chips(ui);
            far(ui, |ui| {
                ui.add_space(4.0);
                self.sort_menu(ui);
                if !self.query.trim().is_empty() {
                    let text = t!("top-results", count = self.results.len());
                    ui.label(txt(&text, 12.5, Some(ui.visuals().weak_text_color())));
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
                ui.add_space(10.0);
                lbl(ui, &t!("top-sort-by-heading"), 11.0, weak);
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
        let tags = self.library.all_tags();
        if tags.is_empty() {
            return;
        }
        ui.add_space(6.0);
        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
        ui.painter().rect_filled(
            sep_rect,
            0.0,
            ui.visuals().weak_text_color().gamma_multiply(0.3),
        );
        ui.add_space(6.0);
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
        for (t, n) in &shown {
            let sel = is_active(t);
            let c = tag_color(t);
            let job = pill_job(
                t,
                13.0,
                if sel { c } else { ui.visuals().text_color() },
                Some((
                    n.to_string(),
                    if sel {
                        c.gamma_multiply(0.75)
                    } else {
                        ui.visuals().weak_text_color()
                    },
                )),
            );
            let r = tip(
                icons::pill(ui, Icon::Tag, job, c, sel),
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
                        &t!("tags-remove-everywhere", tag = t.as_str(), count = *n),
                        14.0,
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
