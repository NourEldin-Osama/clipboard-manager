//! The embedding server settings (Settings, Search, "Your server").

use super::*;

/// The vector sizes offered: numbers to keep, 0 for all.
fn vector_sizes() -> [(usize, String); 4] {
    [
        (0, t!("settings-vector-full")),
        (512, t!("settings-vector-512")),
        (256, t!("settings-vector-256")),
        (128, t!("settings-vector-128")),
    ]
}

pub(super) fn server_settings(
    ui: &mut Ui,
    s: &mut ServerConfig,
    profiles: &mut ServerProfiles,
    list: &mut ModelList,
) {
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        lbl(
            ui,
            &t!("settings-server-quick-setup"),
            12.5,
            ui.visuals().weak_text_color(),
        );
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
        &t!("settings-server-address"),
        &t!("settings-server-address-help"),
        |ui| {
            if editable
                && icons::icon_button_ex(
                    ui,
                    Icon::Refresh,
                    &t!("settings-server-check-again"),
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
    if !s.base_url.trim().is_empty() && !crate::net::is_on_this_pc(&s.base_url) {
        let host = crate::net::host(&s.base_url);
        let text = if s
            .base_url
            .trim()
            .to_ascii_lowercase()
            .starts_with("http://")
        {
            t!("settings-server-remote-http", host = host)
        } else {
            t!("settings-server-remote", host = host)
        };
        note(ui, &format!("⚠ {text}"));
    }
    plain_setting(
        ui,
        &t!("settings-server-key"),
        &t!("settings-server-key-help"),
        |ui| {
            let (eye, label) = if list.show_key {
                (Icon::EyeOff, t!("settings-server-hide-key"))
            } else {
                (Icon::Eye, t!("settings-server-show-key"))
            };
            if icons::icon_button_ex(ui, eye, &label, Vec2::splat(28.0), 16.0, None, None).clicked()
            {
                list.show_key = !list.show_key;
            }
            ui.add(
                egui::TextEdit::singleline(&mut s.api_key)
                    .password(!list.show_key)
                    .hint_text(t!("settings-server-optional"))
                    .desired_width(300.0),
            );
        },
    );
    let default_name = list.found.default.clone();
    let reset_model = setting(
        ui,
        &t!("settings-server-model"),
        &t!("settings-server-model-help"),
        !s.model.trim().is_empty(),
        |ui| {
            let any = default_name.is_some() || !list.found.listed.is_empty();
            if editable && any {
                let b = icons::icon_button_ex(
                    ui,
                    Icon::Dropdown,
                    &t!("settings-server-models"),
                    Vec2::splat(28.0),
                    11.0,
                    None,
                    None,
                );
                egui::Popup::menu(&b).show(|ui| {
                    ui.set_min_width(320.0);
                    if let Some(dn) = &default_name {
                        let label = if dn.is_empty() {
                            t!("settings-server-default")
                        } else {
                            t!("settings-server-default-named", model = dn.as_str())
                        };
                        if ui
                            .selectable_label(s.model.is_empty(), txt(&label, 14.0, None))
                            .clicked()
                        {
                            s.model.clear();
                        }
                    }
                    if !list.found.listed.is_empty() {
                        lbl(
                            ui,
                            &t!("settings-server-listed"),
                            10.5,
                            ui.visuals().weak_text_color(),
                        );
                        for m in &list.found.listed {
                            if ui.selectable_label(s.model == *m, m).clicked() {
                                s.model = m.clone();
                            }
                        }
                    }
                });
            }
            let hint = match default_name.as_deref() {
                Some(dn) if !dn.is_empty() => t!("settings-server-default-hint", model = dn),
                _ => t!("settings-server-default-lower"),
            };
            ui.add_enabled(
                editable,
                egui::TextEdit::singleline(&mut s.model)
                    .hint_text(hint)
                    .desired_width(300.0),
            );
        },
    );
    if reset_model {
        s.model.clear();
    }
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        if list.pending.is_some() {
            ui.add(egui::Spinner::new().size(12.0));
            lbl(
                ui,
                &t!("settings-server-checking"),
                12.5,
                ui.visuals().weak_text_color(),
            );
        } else if let Some(e) = &list.error {
            icons::chip(ui, Icon::Warning, e, RED);
        } else if s.model.trim().is_empty() {
            match default_name.as_deref() {
                Some(dn) => {
                    let model = if dn.is_empty() {
                        t!("settings-server-the-default-model")
                    } else {
                        dn.to_string()
                    };
                    icons::chip(
                        ui,
                        Icon::Check,
                        &t!("settings-server-connected-uses", model = model),
                        GREEN,
                    );
                }
                None if !list.found.listed.is_empty() => {
                    chip(ui, t!("settings-server-no-default"), AMBER);
                }
                None => {}
            }
        } else if default_name.is_some() || !list.found.listed.is_empty() {
            icons::chip(ui, Icon::Check, &t!("settings-server-connected"), GREEN);
        }
        if !editable {
            lbl(
                ui,
                &t!("settings-server-launch-note"),
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
            icons::chip(ui, Icon::Info, &t!("settings-server-gemma-tip"), AMBER);
            if ui
                .button(txt(&t!("settings-server-use-them"), 14.0, None))
                .clicked()
            {
                s.query_prefix = ServerConfig::GEMMA_QUERY.into();
                s.document_prefix = ServerConfig::GEMMA_DOCUMENT.into();
            }
        });
    }

    ui.add_space(6.0);
    egui::CollapsingHeader::new(txt(&t!("settings-server-advanced"), 13.5, None))
        .id_salt("advanced")
        .show(ui, |ui| {
            setting_for(
                ui,
                &t!("settings-server-query-prompt"),
                &t!("settings-server-query-prompt-help"),
                &mut s.query_prefix,
                String::new(),
                |ui, prefix| {
                    ui.add(
                        egui::TextEdit::singleline(prefix)
                            .hint_text(t!("settings-server-none"))
                            .desired_width(300.0),
                    )
                },
            );
            setting_for(
                ui,
                &t!("settings-server-item-prompt"),
                &t!("settings-server-item-prompt-help"),
                &mut s.document_prefix,
                String::new(),
                |ui, prefix| {
                    ui.add(
                        egui::TextEdit::singleline(prefix)
                            .hint_text(t!("settings-server-none"))
                            .desired_width(300.0),
                    )
                },
            );
            setting_for(
                ui,
                &t!("settings-server-vector-size"),
                &t!("settings-server-vector-size-help"),
                &mut s.truncate_dims,
                0,
                |ui, dims| {
                    let sizes = vector_sizes();
                    let current = sizes
                        .iter()
                        .find(|(v, _)| v == dims)
                        .map(|(_, l)| l.clone())
                        .unwrap_or_else(|| t!("settings-vector-custom", count = *dims));
                    egui::ComboBox::from_id_salt("dims")
                        .selected_text(current)
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for (v, l) in sizes {
                                ui.selectable_value(dims, v, l);
                            }
                        });
                },
            );
            setting_for(
                ui,
                &t!("settings-server-images"),
                &t!("settings-server-images-help"),
                &mut s.images,
                false,
                toggle,
            );
            let mut launch = s.launch.is_some();
            setting_for(
                ui,
                &t!("settings-server-launch"),
                &t!("settings-server-launch-help"),
                &mut launch,
                false,
                toggle,
            );
            if launch && s.launch.is_none() {
                s.launch = Some(LaunchConfig {
                    llama_server: PathBuf::from(if cfg!(windows) {
                        "llama-server.exe"
                    } else {
                        "llama-server"
                    }),
                    gguf: PathBuf::new(),
                    mmproj: None,
                    port: 8765,
                    extra_args: "--pooling mean -c 2048 -ub 2048".into(),
                });
            } else if !launch {
                s.launch = None;
            }
            if let Some(l) = &mut s.launch {
                plain_setting(
                    ui,
                    &t!("settings-server-program"),
                    &t!("settings-server-program-help"),
                    |ui| path_field(ui, &mut l.llama_server, &["exe"]),
                );
                plain_setting(ui, &t!("settings-server-model-file"), "", |ui| {
                    path_field(ui, &mut l.gguf, &["gguf"])
                });
                plain_setting(
                    ui,
                    &t!("settings-server-projector"),
                    &t!("settings-server-projector-help"),
                    |ui| {
                        let mut p = l.mmproj.clone().unwrap_or_default();
                        path_field(ui, &mut p, &["gguf"]);
                        l.mmproj = if p.as_os_str().is_empty() {
                            None
                        } else {
                            Some(p)
                        };
                    },
                );
                plain_setting(ui, &t!("settings-server-port"), "", |ui| {
                    ui.add(egui::DragValue::new(&mut l.port).range(1024..=65535));
                });
                plain_setting(
                    ui,
                    &t!("settings-server-extra-args"),
                    &t!("settings-server-extra-args-help"),
                    |ui| {
                        ui.add(egui::TextEdit::singleline(&mut l.extra_args).desired_width(300.0));
                    },
                );
            }
        });
}
