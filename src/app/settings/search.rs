//! Settings, Search: how search works and the meaning-search model.

use super::*;

pub(super) const STRICTNESS: [(f32, &str); 3] = [
    (0.0, "Show all"),
    (0.15, "Related only"),
    (0.3, "Close only"),
];

impl App {
    pub(super) fn search_page(&mut self, ui: &mut Ui, d: &mut Config, apply_model: &mut bool) {
        let def = Config::default();
        section(ui, "Search");
        setting_for(ui, "How search works", "Smart: finds items that contain your words and items that mean the same thing, merged into one list.\nMeaning: only by meaning (needs a search model).\nExact words: only items containing what you type.", &mut d.search_mode, def.search_mode, |ui, mode| {
            segmented(ui, mode, &[(SearchMode::Hybrid, "Smart"), (SearchMode::Semantic, "Meaning"), (SearchMode::Keyword, "Exact words")])
        });
        setting_for(ui, "Meaning matches to show", "How closely an item has to match before it's listed. \"Show all\" ranks everything by closeness; stricter options hide weak matches. Exact word matches are always shown.", &mut d.min_similarity, def.min_similarity, |ui, min_similarity| {
            let current = STRICTNESS.iter().find(|(v, _)| (*v - *min_similarity).abs() < 1e-4).map(|(_, l)| tr(l).to_string()).unwrap_or_else(|| trf("Custom ({v})", &[("v", &format!("{:.2}", min_similarity))]));
            egui::ComboBox::from_id_salt("strict").selected_text(txt(&current, 14.0, None)).width(150.0).show_ui(ui, |ui| {
                for (v, l) in STRICTNESS {
                    ui.selectable_value(min_similarity, v, txt(tr(l), 14.0, None));
                }
            });
        });

        section(ui, "Meaning search model");
        let (dot, text) = self.model_status();
        row(ui, |ui| {
            ui.add_space(10.0);
            let color = if dot == Color32::GRAY {
                ui.visuals().weak_text_color()
            } else {
                dot
            };
            icons::glyph(ui, Icon::Dot, 12.0, color);
            ui.label(txt(&text, 13.0, Some(color)));
        });
        ui.add_space(4.0);
        let mut kind = d.backend.kind();
        plain_setting(ui, "Model", "Off: search by words only (plus text found in images).\nBuilt-in: a model inside this app that understands text and images, no other software needed (one-time download).\nYour server: use a model you already run, such as llama.cpp, Unsloth, Sentence Transformers, Ollama or LM Studio.", |ui| {
            segmented(ui, &mut kind, &[(BackendKind::Off, "Off"), (BackendKind::BuiltIn, "Built-in"), (BackendKind::Server, "Your server")]);
        });
        d.choose_backend(kind);
        match &mut d.backend {
            EmbedBackend::None => note(ui, "Searching by words only. Turn on a model to also find items by meaning, e.g. “trip” finds “flight to Jeddah”."),
            EmbedBackend::SigLip { repo } => self.builtin_models(ui, repo),
            EmbedBackend::Server(s) => {
                server_settings(ui, s, &mut d.server_profiles, &mut self.model_list)
            }
        }
        if d.backend != self.cfg.backend {
            ui.add_space(8.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, "Apply model").clicked() {
                    *apply_model = true;
                }
                if ui.button(txt(tr("Discard changes"), 13.0, None)).clicked() {
                    d.backend = self.cfg.backend.clone();
                }
                lbl(
                    ui,
                    "Your items are indexed again in the background.",
                    12.5,
                    ui.visuals().weak_text_color(),
                );
            });
        }
    }

    pub(super) fn builtin_models(&mut self, ui: &mut Ui, repo: &mut String) {
        note(ui, "Runs on this PC's processor. Finds photos and screenshots from a description, like “receipt from a restaurant”.");
        let mut start: Option<String> = None;
        for m in &SIGLIP_MODELS {
            let ready = models::siglip_ready(&self.paths.models, m.repo);
            let selected = *repo == m.repo;
            let size = format!("{:.1}", m.size_mb as f32 / 1000.0);
            choice_setting(
                ui,
                m.label,
                &trf(
                    "{repo} · {size} GB download",
                    &[("repo", &m.repo), ("size", &size)],
                ),
                selected,
                |ui| {
                    if ready {
                        chip(ui, "✔ Downloaded", GREEN);
                    } else if let Some(dl) = self.download.as_ref().filter(|x| x.repo == m.repo) {
                        let p = &dl.progress;
                        if ui.small_button(txt(tr("Cancel"), 12.0, None)).clicked() {
                            p.cancel.store(true, Ordering::Relaxed);
                        }
                        let (done, total) = (
                            p.done.load(Ordering::Relaxed),
                            p.total.load(Ordering::Relaxed).max(1),
                        );
                        ui.add(
                            egui::ProgressBar::new(done as f32 / total as f32)
                                .desired_width(150.0)
                                .text(format!("{} / {} MB", done >> 20, total >> 20)),
                        );
                        ui.ctx().request_repaint_after(Duration::from_millis(250));
                    } else if self.download.is_none()
                        && ui
                            .button(txt(
                                &trf("Download {size} GB", &[("size", &size)]),
                                14.0,
                                None,
                            ))
                            .clicked()
                    {
                        start = Some(m.repo.to_string());
                    }
                    if !selected
                        && ui
                            .add(
                                egui::Button::new(txt(tr("Use"), 14.0, None))
                                    .frame_when_inactive(false),
                            )
                            .clicked()
                    {
                        *repo = m.repo.to_string();
                    }
                },
            );
        }
        if let Some(r) = start {
            *repo = r.clone();
            let progress = Arc::new(Progress::default());
            let (p, dir, rr, c) = (
                progress.clone(),
                self.paths.models.clone(),
                r.clone(),
                ui.ctx().clone(),
            );
            let handle = std::thread::spawn(move || {
                let out = models::download_siglip(&dir, &rr, &p);
                c.request_repaint();
                out
            });
            self.download = Some(Download {
                repo: r,
                progress,
                handle,
            });
        }
    }
}
