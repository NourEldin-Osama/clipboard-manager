//! The embedding server settings (Settings, Search, "Your server").

use super::*;

pub(super) const VECTOR_SIZES: [(usize, &str); 4] = [
    (0, "Full · best quality"),
    (512, "512 · smaller"),
    (256, "256 · compact"),
    (128, "128 · tiny, fastest"),
];

pub(super) fn server_settings(
    ui: &mut Ui,
    s: &mut ServerConfig,
    profiles: &mut ServerProfiles,
    list: &mut ModelList,
) {
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        lbl(ui, "Quick setup:", 12.5, ui.visuals().weak_text_color());
        for (name, url) in [
            ("llama.cpp", "http://127.0.0.1:8080/v1"),
            ("Unsloth", "http://127.0.0.1:8888/v1"),
            ("Sentence Transformers", "http://127.0.0.1:8001/v1"),
            ("Ollama", "http://127.0.0.1:11434/v1"),
            ("LM Studio", "http://127.0.0.1:1234/v1"),
        ] {
            let current = s.base_url.trim_end_matches('/') == url;
            let b = egui::Button::new(RichText::new(name).size(12.5).color(if current {
                accent()
            } else {
                ui.visuals().text_color()
            }))
            .frame_when_inactive(current)
            .fill(if current {
                accent().gamma_multiply(0.13)
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(CornerRadius::same(12));
            if ui.add(b).on_hover_text(url).clicked() {
                profiles.switch_to(s, url);
            }
        }
    });

    list.poll();
    let editable = s.launch.is_none();
    if editable {
        list.fetch_when_settled(s, ui.ctx());
    }
    plain_setting(
        ui,
        "Server address",
        "The address of the embeddings server, usually ending in /v1.",
        |ui| {
            if editable
                && icons::icon_button_ex(
                    ui,
                    Icon::Refresh,
                    "Check again",
                    Vec2::splat(28.0),
                    16.0,
                    None,
                    None,
                )
                .clicked()
            {
                list.fetch(s, ui.ctx());
            }
            ui.add_enabled(
                editable,
                egui::TextEdit::singleline(&mut s.base_url)
                    .hint_text("http://127.0.0.1:8080/v1")
                    .desired_width(300.0),
            );
        },
    );
    plain_setting(
        ui,
        "API key",
        "Only if your server asks for one. Sent as a Bearer token and stored only on this PC.",
        |ui| {
            let (eye, label) = if list.show_key {
                (Icon::EyeOff, "Hide key")
            } else {
                (Icon::Eye, "Show key")
            };
            if icons::icon_button_ex(ui, eye, label, Vec2::splat(28.0), 16.0, None, None).clicked()
            {
                list.show_key = !list.show_key;
            }
            ui.add(
                egui::TextEdit::singleline(&mut s.api_key)
                    .password(!list.show_key)
                    .hint_text("optional")
                    .desired_width(300.0),
            );
        },
    );
    let default_name = list.found.default.clone();
    let reset_model = setting(ui, "Model", "Leave empty to use the server's default embedding model. Pick from ▾ or type a name your server knows.", !s.model.trim().is_empty(), |ui| {
        let any = default_name.is_some() || !list.found.listed.is_empty();
        if editable && any {
            let b = icons::icon_button_ex(ui, Icon::Dropdown, "Models on this server", Vec2::splat(28.0), 11.0, None, None);
            egui::Popup::menu(&b).show(|ui| {
                ui.set_min_width(320.0);
                if let Some(dn) = &default_name {
                    let label = if dn.is_empty() { tr("Server default").to_string() } else { trf("Server default · {model}", &[("model", dn)]) };
                    if ui.selectable_label(s.model.is_empty(), txt(&label, 14.0, None)).clicked() {
                        s.model.clear();
                    }
                }
                if !list.found.listed.is_empty() {
                    lbl(ui, "LISTED BY THE SERVER (MAY INCLUDE CHAT MODELS)", 10.5, ui.visuals().weak_text_color());
                    for m in &list.found.listed {
                        if ui.selectable_label(s.model == *m, m).clicked() {
                            s.model = m.clone();
                        }
                    }
                }
            });
        }
        let hint = match default_name.as_deref() {
            Some(dn) if !dn.is_empty() => trf("default: {model}", &[("model", &dn)]),
            _ => tr("server default").to_string(),
        };
        ui.add_enabled(editable, egui::TextEdit::singleline(&mut s.model).hint_text(hint).desired_width(300.0));
    });
    if reset_model {
        s.model.clear();
    }
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        if list.pending.is_some() {
            ui.add(egui::Spinner::new().size(12.0));
            lbl(
                ui,
                "Checking the server…",
                12.5,
                ui.visuals().weak_text_color(),
            );
        } else if let Some(e) = &list.error {
            chip(ui, format!("⚠ {e}"), RED);
        } else if s.model.trim().is_empty() {
            match default_name.as_deref() {
                Some(dn) => {
                    let model = if dn.is_empty() {
                        tr("the default model")
                    } else {
                        dn
                    };
                    chip(
                        ui,
                        trf("✔ Connected · uses {model}", &[("model", &model)]),
                        GREEN,
                    );
                }
                None if !list.found.listed.is_empty() => {
                    chip(
                        ui,
                        "Connected, but no default embedding model: choose one",
                        AMBER,
                    );
                }
                None => {}
            }
        } else if default_name.is_some() || !list.found.listed.is_empty() {
            chip(ui, "✔ Connected", GREEN);
        }
        if !editable {
            lbl(
                ui,
                "The app starts llama-server itself; it connects after Apply.",
                12.5,
                ui.visuals().weak_text_color(),
            );
        }
    });

    let effective = if s.model.is_empty() {
        default_name.clone().unwrap_or_default()
    } else {
        s.model.clone()
    };
    let gemma = effective.to_lowercase().contains("embeddinggemma");
    let gemma_prompts = s.query_prefix == ServerConfig::GEMMA_QUERY
        && s.document_prefix == ServerConfig::GEMMA_DOCUMENT;
    if gemma && !gemma_prompts {
        row(ui, |ui| {
            ui.add_space(10.0);
            chip(
                ui,
                "💡 EmbeddingGemma searches better with its own prompts",
                AMBER,
            );
            if ui.button(txt(tr("Use them"), 14.0, None)).clicked() {
                s.query_prefix = ServerConfig::GEMMA_QUERY.into();
                s.document_prefix = ServerConfig::GEMMA_DOCUMENT.into();
            }
        });
    }

    ui.add_space(6.0);
    egui::CollapsingHeader::new(txt(tr("Advanced"), 13.5, None)).id_salt("advanced").show(ui, |ui| {
        if setting(ui, "Query prompt", "Text put before what you search for. Some models need it (EmbeddingGemma: “task: search result | query: ”).", !s.query_prefix.is_empty(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.query_prefix).hint_text("none").desired_width(300.0));
        }) {
            s.query_prefix.clear();
        }
        if setting(ui, "Item prompt", "Text put before each item when it's indexed (EmbeddingGemma: “title: none | text: ”).", !s.document_prefix.is_empty(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.document_prefix).hint_text("none").desired_width(300.0));
        }) {
            s.document_prefix.clear();
        }
        if setting(ui, "Vector size", "Smaller vectors use less memory and search faster, with slightly lower quality. Works with Matryoshka models such as EmbeddingGemma and UltraVBERT.", s.truncate_dims != 0, |ui| {
            let current = VECTOR_SIZES.iter().find(|(v, _)| *v == s.truncate_dims).map(|(_, l)| l.to_string()).unwrap_or_else(|| format!("{} dims", s.truncate_dims));
            egui::ComboBox::from_id_salt("dims").selected_text(current).width(170.0).show_ui(ui, |ui| {
                for (v, l) in VECTOR_SIZES {
                    ui.selectable_value(&mut s.truncate_dims, v, l);
                }
            });
        }) {
            s.truncate_dims = 0;
        }
        if setting(ui, "Search images with this model", "Send images to the server too. Only for models that understand images (CLIP, SigLIP, BiUltraVBERT…). Otherwise images are found by the text in them.", s.images, |ui| {
            toggle(ui, &mut s.images);
        }) {
            s.images = false;
        }
        let mut launch = s.launch.is_some();
        if setting(ui, "Start llama-server for me", "The app starts llama-server with your .gguf file in the background and stops it when you quit.", launch, |ui| {
            toggle(ui, &mut launch);
        }) {
            launch = false;
        }
        if launch && s.launch.is_none() {
            s.launch = Some(LaunchConfig {
                llama_server: PathBuf::from(if cfg!(windows) { "llama-server.exe" } else { "llama-server" }),
                gguf: PathBuf::new(),
                mmproj: None,
                port: 8765,
                extra_args: "--pooling mean -c 2048 -ub 2048".into(),
            });
        } else if !launch {
            s.launch = None;
        }
        if let Some(l) = &mut s.launch {
            plain_setting(ui, "llama-server program", "llama-server.exe from a llama.cpp release.", |ui| path_field(ui, &mut l.llama_server, &["exe"]));
            plain_setting(ui, "Model file (.gguf)", "", |ui| path_field(ui, &mut l.gguf, &["gguf"]));
            plain_setting(ui, "Image projector (.gguf)", "Optional mmproj file, for models that embed images.", |ui| {
                let mut p = l.mmproj.clone().unwrap_or_default();
                path_field(ui, &mut p, &["gguf"]);
                l.mmproj = if p.as_os_str().is_empty() { None } else { Some(p) };
            });
            plain_setting(ui, "Port", "", |ui| {
                ui.add(egui::DragValue::new(&mut l.port).range(1024..=65535));
            });
            plain_setting(ui, "Extra arguments", "Passed to llama-server as-is.", |ui| {
                ui.add(egui::TextEdit::singleline(&mut l.extra_args).desired_width(300.0));
            });
        }
    });
}
