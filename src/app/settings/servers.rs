//! Settings, Search & AI: the saved servers, the editor of a server and its
//! models, and the menus that pick the models in use.

use super::*;
use crate::catalog;
use crate::config::{
    Model, ModelJob, ModelRef, ReadEngine, ReadSettings, SearchEngine, SearchSettings, Server,
    ServerKind,
};
use crate::embed::server::SearchTest;
use crate::i18n::log_server_error;

/// A server being added or edited: a copy, which Save puts in the settings.
pub(in crate::app) struct ServerEditor {
    server: Server,
    /// Not in the settings yet.
    is_new: bool,
    /// The model whose settings are open.
    open: Option<u32>,
    /// What the server offers, checked while the address and key are typed.
    found: ModelList,
    show_key: bool,
    tests: HashMap<u32, Test>,
    fills: HashMap<u32, Fill>,
    /// "Delete server" was clicked once.
    deleting: bool,
    /// Saving would start sending to servers outside this PC: the search
    /// server's and the reader's addresses, asked about first.
    ask_send: Option<(Option<String>, Option<String>)>,
}

impl ServerEditor {
    fn new(server: Server, is_new: bool) -> Self {
        Self {
            server,
            is_new,
            open: None,
            found: ModelList::default(),
            show_key: false,
            tests: HashMap::new(),
            fills: HashMap::new(),
            deleting: false,
            ask_send: None,
        }
    }
}

/// A change that would start sending to a server outside this PC, held back
/// until the user agrees: the settings with it, and the addresses.
pub(in crate::app) struct PendingSend {
    config: Config,
    /// The server the history would go to.
    search: Option<String>,
    /// The server the copied images would go to.
    reader: Option<String>,
}

/// A test request to a model: running, or what it found.
#[derive(Default)]
struct Test {
    running: Option<Receiver<Result<Tested, String>>>,
    result: Option<Result<Tested, String>>,
}

enum Tested {
    Search(SearchTest),
    Read { text: String, took: Duration },
}

/// Filling in a model's details: the lookup on models.dev, and what it said.
#[derive(Default)]
struct Fill {
    running: Option<Receiver<Result<Option<catalog::Entry>, String>>>,
    said: Vec<String>,
}

/// What a model is called in menus: the name given to it, its name on the
/// server, or "Server default".
pub(in crate::app) fn model_label(model: &Model) -> String {
    match (model.name.trim(), model.model.trim()) {
        ("", "") => t!("settings-server-default"),
        ("", model) => model.to_string(),
        (name, _) => name.to_string(),
    }
}

fn kind_name(kind: ServerKind) -> String {
    match kind {
        ServerKind::Other => t!("server-kind-other"),
        kind => kind.name().to_string(),
    }
}

fn is_search(model: &Model) -> bool {
    matches!(model.job, ModelJob::Search(_))
}

/// The models for one job, as references, in the order of the servers.
pub(super) fn models_for(d: &Config, search: bool) -> Vec<ModelRef> {
    d.servers
        .iter()
        .flat_map(|server| {
            server
                .models
                .iter()
                .filter(move |m| is_search(m) == search)
                .map(|m| ModelRef {
                    server: server.id,
                    model: m.id,
                })
        })
        .collect()
}

/// Whether `at` names a model for the job.
pub(super) fn is_model_for(d: &Config, at: Option<ModelRef>, search: bool) -> bool {
    at.and_then(|at| d.model(at))
        .is_some_and(|(_, m)| is_search(m) == search)
}

/// 12,345: numbers as the history size shows them.
fn thousands(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(",")
}

/// A new type fills in its usual address and name, unless the user typed
/// others: a server on another port keeps its port.
fn choose_kind(server: &mut Server, kind: ServerKind) {
    let was = server.kind;
    if kind == was {
        return;
    }
    let address = server.base_url.trim().trim_end_matches('/');
    if address.is_empty() || address == was.address() {
        server.base_url = kind.address().to_string();
    }
    let name = server.name.trim();
    if name.is_empty() || name == was.name() || name == kind_name(was) {
        server.name = kind.name().to_string();
    }
    server.kind = kind;
    if kind != ServerKind::LlamaCpp {
        server.launch = None;
    }
}

/// The type of server, as a row of tiles.
fn kind_tiles(ui: &mut Ui, server: &mut Server) {
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        for kind in ServerKind::ALL {
            let selected = server.kind == kind;
            if kind_tile(ui, kind, selected).clicked() {
                choose_kind(server, kind);
            }
        }
    });
}

/// A type's logo above its name.
fn kind_tile(ui: &mut Ui, kind: ServerKind, selected: bool) -> egui::Response {
    let name = kind_name(kind);
    // Seven fit in one row of the settings column.
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(86.0, 62.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, &name));
    let v = ui.visuals();
    let (fill, stroke, color) = if selected {
        let shade = text_shade(accent(), v.dark_mode);
        (
            accent().gamma_multiply(0.13),
            Stroke::new(1.5, shade),
            shade,
        )
    } else if resp.hovered() {
        (
            v.widgets.hovered.weak_bg_fill,
            Stroke::NONE,
            v.strong_text_color(),
        )
    } else {
        (
            v.widgets.inactive.weak_bg_fill,
            Stroke::NONE,
            v.text_color(),
        )
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(9),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    let logo = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + 21.0),
        Vec2::splat(24.0),
    );
    icons::paint_logo(p, logo, kind, color);
    paint_text(
        ui,
        p,
        Pos2::new(rect.center().x, rect.bottom() - 14.0),
        egui::Align2::CENTER_CENTER,
        &name,
        FontId::proportional(12.5),
        color,
    );
    resp
}

/// The model's name on the server: typed, or picked from what it lists.
fn model_field(ui: &mut Ui, model: &mut String, listed: &[String], default: Option<&str>) {
    if !listed.is_empty() {
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
            // What is typed narrows the list, unless it is one of them.
            let typed = model.trim().to_lowercase();
            let exact = listed.contains(model);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    let shown = listed
                        .iter()
                        .filter(|m| exact || m.to_lowercase().contains(&typed))
                        .take(300);
                    for m in shown {
                        if ui.selectable_label(*m == *model, m.as_str()).clicked() {
                            *model = m.clone();
                        }
                    }
                });
        });
    }
    let hint = match default {
        Some(name) if !name.is_empty() => t!("settings-server-default-hint", model = name),
        _ => t!("settings-server-default"),
    };
    let field = ui.add(
        egui::TextEdit::singleline(model)
            .hint_text(txt(&hint, 14.0, None))
            .desired_width(280.0),
    );
    set_name(&field, &row_title(ui));
}

fn start_test(server: &Server, model: &Model, ctx: &egui::Context) -> Test {
    let (tx, rx) = channel();
    let c = ctx.clone();
    let failed = |e: anyhow::Error| log_server_error!("error-model-test", &e);
    if let Some(cfg) = server.search_config(model) {
        std::thread::spawn(move || {
            let r = crate::embed::server::test_search(cfg)
                .map(Tested::Search)
                .map_err(failed);
            let _ = tx.send(r);
            c.request_repaint();
        });
    } else if let Some(cfg) = server.read_config(model) {
        std::thread::spawn(move || {
            let r = crate::ocr::test_read(cfg)
                .map(|(text, took)| Tested::Read { text, took })
                .map_err(failed);
            let _ = tx.send(r);
            c.request_repaint();
        });
    }
    Test {
        running: Some(rx),
        result: None,
    }
}

/// Fills in what the app itself knows about the model at once, then looks
/// it up on models.dev in the background.
fn start_fill(model: &mut Model, name: &str, kind: ServerKind, ctx: &egui::Context) -> Fill {
    let mut said = Vec::new();
    match &mut model.job {
        ModelJob::Search(s) => {
            if let Some((query, document)) = catalog::known_prompts(name) {
                if s.query_prefix.is_empty() && s.document_prefix.is_empty() {
                    s.query_prefix = query.into();
                    s.document_prefix = document.into();
                    said.push(t!("settings-model-fill-prompts"));
                }
            }
        }
        ModelJob::Read(r) => {
            if let Some(known) = catalog::reader_kind(name) {
                if r.kind != known {
                    r.kind = known;
                    r.prompt = known.prompt().into();
                    said.push(t!("settings-model-fill-kind", model = known.name()));
                }
            }
        }
    }
    let provider = match kind {
        ServerKind::OpenRouter => Some("openrouter"),
        ServerKind::LmStudio => Some("lmstudio"),
        _ => None,
    };
    let (tx, rx) = channel();
    let (name, c) = (name.to_string(), ctx.clone());
    std::thread::spawn(move || {
        let r = catalog::look_up(&name, provider)
            .map_err(|e| log_error!("error-models-dev", error = format!("{e:#}")));
        let _ = tx.send(r);
        c.request_repaint();
    });
    Fill {
        running: Some(rx),
        said,
    }
}

/// What models.dev said, put into the model: its name, if it has none yet.
fn finish_fill(model: &mut Model, found: Result<Option<catalog::Entry>, String>) -> Vec<String> {
    let mut said = Vec::new();
    match found {
        Ok(Some(entry)) => {
            if model.name.trim().is_empty() && !entry.name.trim().is_empty() {
                model.name = entry.name.trim().to_string();
            }
            said.push(if entry.limit.context > 0 {
                t!(
                    "settings-model-fill-found",
                    name = entry.name.as_str(),
                    tokens = thousands(entry.limit.context)
                )
            } else {
                t!(
                    "settings-model-fill-found-no-limit",
                    name = entry.name.as_str()
                )
            });
            match (&model.job, entry.takes_images()) {
                (ModelJob::Search(_), true) => said.push(t!("settings-model-fill-images")),
                (ModelJob::Read(_), false) => said.push(t!("settings-model-fill-no-images")),
                _ => {}
            }
        }
        Ok(None) => said.push(t!("settings-model-fill-not-found")),
        Err(e) => said.push(e),
    }
    said
}

/// What sending to servers outside this PC means, before the user agrees.
fn send_questions(ui: &mut Ui, search: &Option<String>, reader: &Option<String>) {
    Frame::new()
        .inner_margin(Margin::symmetric(10, 0))
        .show(ui, |ui| {
            let strong = ui.visuals().strong_text_color();
            let font = FontId::proportional(13.5);
            if let Some(url) = search {
                let host = crate::net::host(url);
                let question = t!("settings-server-send-question", host = host);
                super::bidi::paragraph(ui, &question, font.clone(), strong);
            }
            if let Some(url) = reader {
                let host = crate::net::host(url);
                let question = t!("settings-reader-send-question", host = host);
                super::bidi::paragraph(ui, &question, font, strong);
            }
        });
}

/// A test's result, under the model.
fn test_line(ui: &mut Ui, test: &Test) {
    let Some(result) = &test.result else {
        return;
    };
    let seconds = |took: &Duration| format!("{:.1}", took.as_secs_f32());
    super::view::row_wrapped(ui, |ui| {
        ui.add_space(10.0);
        match result {
            Ok(Tested::Search(t)) if t.vectors > 1 => icons::chip(
                ui,
                Icon::Check,
                &t!(
                    "settings-model-test-multi",
                    vectors = t.vectors,
                    size = t.size,
                    seconds = seconds(&t.took)
                ),
                GREEN,
            ),
            Ok(Tested::Search(t)) => icons::chip(
                ui,
                Icon::Check,
                &t!(
                    "settings-model-test-search",
                    size = t.size,
                    seconds = seconds(&t.took)
                ),
                GREEN,
            ),
            Ok(Tested::Read { text, took }) => {
                let shown: String = text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(70)
                    .collect();
                // The test image says "Clipboard Manager".
                let read = text.to_lowercase().contains("clipboard");
                icons::chip(
                    ui,
                    if read { Icon::Check } else { Icon::Warning },
                    &t!(
                        "settings-model-test-read",
                        text = shown,
                        seconds = seconds(took)
                    ),
                    if read { GREEN } else { AMBER },
                )
            }
            Err(e) => icons::chip(ui, Icon::Warning, e, RED),
        };
    });
}

impl App {
    /// The servers, below the engines: one row each, then "Add server".
    pub(super) fn servers_section(&mut self, ui: &mut Ui, d: &Config) {
        section(ui, &t!("settings-servers-section"));
        if d.servers.is_empty() {
            note(ui, &t!("settings-servers-none"));
        }
        let mut edit = None;
        let weak = ui.visuals().weak_text_color();
        let strong = ui.visuals().strong_text_color();
        for server in &d.servers {
            ui.push_id(server.id, |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        row(ui, |ui| {
                            ui.set_min_height(30.0);
                            icons::logo(ui, server.kind, 20.0, strong);
                            ui.add_space(4.0);
                            lbl(ui, server.label(), 14.0, strong);
                            far(ui, |ui| {
                                if ui
                                    .button(txt(&t!("settings-server-edit"), 13.0, None))
                                    .clicked()
                                {
                                    edit = Some(server.clone());
                                }
                                let count = server.models.len();
                                lbl(
                                    ui,
                                    &t!("settings-server-models-count", count = count),
                                    12.5,
                                    weak,
                                );
                                lbl(ui, crate::net::host(&server.base_url), 12.5, weak);
                            });
                        });
                    });
            });
        }
        row(ui, |ui| {
            ui.add_space(10.0);
            if ui
                .button(txt(&t!("settings-server-add"), 13.0, None))
                .clicked()
            {
                self.add_server(d);
            }
        });
        if let Some(server) = edit {
            self.server_editor = Some(ServerEditor::new(server, false));
        }
    }

    pub(super) fn add_server(&mut self, d: &Config) {
        let server = Server {
            id: d.next_server_id(),
            ..Default::default()
        };
        self.server_editor = Some(ServerEditor::new(server, true));
    }

    /// The menu that picks the model in use for searching or for reading,
    /// grouped by server; picking one applies it.
    pub(super) fn model_menu(&mut self, ui: &mut Ui, d: &mut Config, search: bool) {
        let chosen = if search { d.search_model } else { d.read_model };
        let current = chosen
            .and_then(|at| d.model(at))
            .filter(|(_, m)| is_search(m) == search)
            .map(|(server, m)| {
                t!(
                    "settings-model-on-server",
                    model = model_label(m),
                    server = server.label()
                )
            })
            .unwrap_or_else(|| t!("settings-model-choose"));
        let mut picked = None;
        let mut add = false;
        let help = if search {
            t!("settings-search-model-help")
        } else {
            t!("settings-read-model-help")
        };
        let in_use = chosen
            .and_then(|at| d.server(at.server))
            .map(|server| server.kind);
        plain_setting(ui, &t!("settings-model"), &help, |ui| {
            let weak = ui.visuals().weak_text_color();
            let strong = ui.visuals().strong_text_color();
            let menu =
                egui::ComboBox::from_id_salt(if search { "search-model" } else { "read-model" })
                    .selected_text(txt(&current, 14.0, None))
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for server in &d.servers {
                            let models: Vec<&Model> = server
                                .models
                                .iter()
                                .filter(|m| is_search(m) == search)
                                .collect();
                            if models.is_empty() {
                                continue;
                            }
                            row(ui, |ui| {
                                icons::logo(ui, server.kind, 14.0, weak);
                                lbl(ui, &server.label().to_uppercase(), 11.0, weak);
                            });
                            for m in models {
                                let at = ModelRef {
                                    server: server.id,
                                    model: m.id,
                                };
                                let label = txt(&model_label(m), 14.0, None);
                                if ui.selectable_label(chosen == Some(at), label).clicked() {
                                    picked = Some(at);
                                }
                            }
                        }
                        ui.separator();
                        if ui
                            .selectable_label(
                                false,
                                txt(&t!("settings-model-add-server"), 14.0, None),
                            )
                            .clicked()
                        {
                            add = true;
                        }
                    });
            set_choice_name(&menu.response, &row_title(ui), &current);
            // The control area fills from its far end: this goes before the menu.
            if let Some(kind) = in_use {
                icons::logo(ui, kind, 18.0, strong);
            }
        });
        match picked {
            Some(at) if search => d.search_model = Some(at),
            Some(at) => d.read_model = Some(at),
            None => {}
        }
        if add {
            self.add_server(d);
        }
    }

    /// The question before anything goes to a server outside this PC.
    pub(super) fn send_question(&mut self, ui: &mut Ui, d: &mut Config) {
        let Some(pending) = &self.pending_send else {
            return;
        };
        ui.add_space(8.0);
        send_questions(ui, &pending.search, &pending.reader);
        ui.add_space(4.0);
        let mut answer = None;
        row(ui, |ui| {
            ui.add_space(10.0);
            if primary_button(ui, &t!("settings-server-send")).clicked() {
                answer = Some(true);
            }
            if ui
                .button(txt(&t!("settings-server-send-cancel"), 13.0, None))
                .clicked()
            {
                answer = Some(false);
            }
        });
        if let Some(agreed) = answer {
            let pending = self.pending_send.take().expect("a question is showing");
            if agreed {
                for url in pending.search.iter().chain(&pending.reader) {
                    d.allow_server(url);
                }
                d.search_engine = pending.config.search_engine;
                d.search_model = pending.config.search_model;
                d.read_engine = pending.config.read_engine;
                d.read_model = pending.config.read_model;
            }
        }
    }

    /// What applying `d` would start sending to servers outside this PC that
    /// the user hasn't agreed to yet: the history to the search server, and
    /// the copied images to the reader. Their addresses.
    pub(super) fn unagreed(&self, d: &Config) -> (Option<String>, Option<String>) {
        let outside = |url: &str| !crate::net::is_on_this_pc(url) && !d.is_server_allowed(url);
        let backend = d.search_backend();
        let search = match &backend {
            EmbedBackend::Server(s)
                if backend != self.cfg.search_backend() && outside(&s.base_url) =>
            {
                Some(s.base_url.clone())
            }
            _ => None,
        };
        let reader = d.text_reader();
        let reader = match &reader {
            TextReader::Server(r) if reader != self.cfg.text_reader() && outside(&r.base_url) => {
                Some(r.base_url.clone())
            }
            _ => None,
        };
        (search, reader)
    }

    /// Holds back a change that would start sending to a server outside
    /// this PC: `d` goes back to `before`, and the question shows.
    pub(super) fn hold_unagreed(&mut self, d: &mut Config, before: Config) {
        let (search, reader) = self.unagreed(d);
        if search.is_none() && reader.is_none() {
            return;
        }
        let config = std::mem::replace(d, before);
        self.pending_send = Some(PendingSend {
            config,
            search,
            reader,
        });
        self.settings_page = SettingsPage::Search;
    }

    /// The editor of one server, drawn in place of the Search & AI page.
    pub(super) fn server_editor_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let Some(mut editor) = self.server_editor.take() else {
            return;
        };
        let ctx = ui.ctx().clone();
        let mut close = false;
        let mut save = false;
        let mut delete = false;

        let strong = ui.visuals().strong_text_color();
        let weak = ui.visuals().weak_text_color();
        ui.add_space(8.0);
        row(ui, |ui| {
            if icons::icon_button_ex(
                ui,
                Icon::Back,
                &t!("settings-server-back"),
                Vec2::splat(30.0),
                18.0,
                None,
                None,
            )
            .clicked()
            {
                close = true;
            }
            let title = if editor.is_new {
                t!("settings-server-add-title")
            } else {
                t!("settings-server-edit-title")
            };
            lbl(ui, &title, 17.0, strong);
        });

        section(ui, &t!("settings-server-kind"));
        kind_tiles(ui, &mut editor.server);
        ui.add_space(6.0);

        let server = &mut editor.server;
        plain_setting(
            ui,
            &t!("settings-server-name"),
            &t!("settings-server-name-help"),
            |ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut server.name)
                        .hint_text(crate::net::host(&server.base_url))
                        .desired_width(280.0),
                );
                set_name(&field, &row_title(ui));
            },
        );
        // A server the app starts listens at its port.
        if let Some(launch) = &server.launch {
            server.base_url = format!("http://127.0.0.1:{}/v1", launch.port);
        }
        let launching = server.launch.is_some();
        let mut check = false;
        plain_setting(
            ui,
            &t!("settings-server-address"),
            &t!("settings-server-address-help"),
            |ui| {
                if !launching
                    && ui
                        .button(txt(&t!("settings-server-test-connection"), 13.0, None))
                        .clicked()
                {
                    check = true;
                }
                let usual = match server.kind.address() {
                    "" => "http://127.0.0.1:8080/v1",
                    usual => usual,
                };
                let field = ui.add_enabled(
                    !launching,
                    egui::TextEdit::singleline(&mut server.base_url)
                        .hint_text(usual)
                        .desired_width(280.0),
                );
                set_name(&field, &row_title(ui));
            },
        );
        let embedding_list = server.kind == ServerKind::OpenRouter;
        editor.found.poll();
        if check {
            editor
                .found
                .fetch(&server.base_url, &server.api_key, embedding_list, &ctx);
        } else if !launching {
            editor.found.fetch_when_settled(
                &server.base_url,
                &server.api_key,
                embedding_list,
                &ctx,
            );
        }
        super::view::row_wrapped(ui, |ui| {
            ui.add_space(10.0);
            let found = &editor.found;
            if launching {
                lbl(ui, &t!("settings-server-launch-note"), 12.5, weak);
            } else if found.pending.is_some() {
                super::view::slow_spinner(ui, 12.0, None);
                lbl(ui, &t!("settings-server-checking"), 12.5, weak);
            } else if let Some(e) = &found.error {
                icons::chip(ui, Icon::Warning, e, RED);
            } else if !found.url.is_empty() {
                let count = found
                    .found
                    .listed
                    .len()
                    .max(found.found.embedding.len())
                    .max(usize::from(found.found.default.is_some()));
                icons::chip(
                    ui,
                    Icon::Check,
                    &t!("settings-server-found", count = count),
                    GREEN,
                );
            }
        });
        if !server.base_url.trim().is_empty() && !crate::net::is_on_this_pc(&server.base_url) {
            let host = crate::net::host(&server.base_url);
            let text = if server
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
        let show_key = &mut editor.show_key;
        plain_setting(
            ui,
            &t!("settings-server-key"),
            &t!("settings-server-key-help"),
            |ui| {
                let (eye, label) = if *show_key {
                    (Icon::EyeOff, t!("settings-server-hide-key"))
                } else {
                    (Icon::Eye, t!("settings-server-show-key"))
                };
                if icons::icon_button_ex(ui, eye, &label, Vec2::splat(28.0), 16.0, None, None)
                    .clicked()
                {
                    *show_key = !*show_key;
                }
                let hint = if server.kind == ServerKind::OpenRouter {
                    t!("settings-server-needed")
                } else {
                    t!("settings-server-optional")
                };
                let field = ui.add(
                    egui::TextEdit::singleline(&mut server.api_key)
                        .password(!*show_key)
                        .hint_text(txt(&hint, 14.0, None))
                        .desired_width(280.0),
                );
                set_name(&field, &row_title(ui));
            },
        );
        if server.kind == ServerKind::LlamaCpp {
            launch_settings(ui, server);
        }

        section(ui, &t!("settings-server-models-section"));
        if editor.server.models.is_empty() {
            note(ui, &t!("settings-server-no-models"));
        }
        let mut remove = None;
        let mut test = None;
        let mut fill = None;
        let found = &editor.found.found;
        let default_name = found.default.clone().filter(|name| !name.is_empty());
        for model in editor.server.models.iter_mut() {
            let id = model.id;
            let open = editor.open == Some(id);
            ui.push_id(id, |ui| {
                let job = if is_search(model) {
                    t!("settings-model-job-search")
                } else {
                    t!("settings-model-job-read")
                };
                setting_row(ui, &model_label(model), "", open, false, |ui| {
                    if icons::icon_button_ex(
                        ui,
                        Icon::Delete,
                        &t!("settings-model-remove"),
                        Vec2::splat(28.0),
                        15.0,
                        None,
                        None,
                    )
                    .clicked()
                    {
                        remove = Some(id);
                    }
                    let testing = editor.tests.get(&id).is_some_and(|t| t.running.is_some());
                    if testing {
                        super::view::slow_spinner(ui, 14.0, None);
                    } else if tip(
                        ui.button(txt(&t!("settings-model-test"), 13.0, None)),
                        &t!("settings-model-test-help"),
                    )
                    .clicked()
                    {
                        test = Some(id);
                    }
                    let (icon, label) = if open {
                        (Icon::Check, t!("settings-model-close"))
                    } else {
                        (Icon::Edit, t!("settings-model-open"))
                    };
                    if icons::icon_button_ex(ui, icon, &label, Vec2::splat(28.0), 15.0, None, None)
                        .clicked()
                    {
                        editor.open = if open { None } else { Some(id) };
                    }
                    chip(ui, job, accent());
                });
                if open {
                    // The server's default model is the one it embeds with.
                    let (listed, default) = if is_search(model) {
                        (found.for_search(), default_name.as_deref())
                    } else {
                        (found.listed.as_slice(), None)
                    };
                    model_settings(ui, model, listed, default, editor.fills.get(&id), &mut fill);
                }
                if let Some(t) = editor.tests.get(&id) {
                    test_line(ui, t);
                }
            });
        }
        row(ui, |ui| {
            ui.add_space(10.0);
            for (search, label) in [
                (true, t!("settings-model-add-search")),
                (false, t!("settings-model-add-read")),
            ] {
                if ui.button(txt(&label, 13.0, None)).clicked() {
                    let job = if search {
                        ModelJob::Search(SearchSettings::default())
                    } else {
                        ModelJob::Read(ReadSettings::default())
                    };
                    let id = editor.server.next_model_id();
                    editor.server.models.push(Model {
                        id,
                        job,
                        ..Default::default()
                    });
                    editor.open = Some(id);
                }
            }
        });

        // What the clicks above asked for.
        if let Some(id) = remove {
            editor.server.models.retain(|m| m.id != id);
            editor.tests.remove(&id);
            editor.fills.remove(&id);
        }
        if let Some(id) = test {
            if let Some(model) = editor.server.model(id) {
                editor
                    .tests
                    .insert(id, start_test(&editor.server, model, &ctx));
            }
        }
        if let Some(id) = fill {
            let kind = editor.server.kind;
            if let Some(model) = editor.server.models.iter_mut().find(|m| m.id == id) {
                let name = match model.model.trim() {
                    "" if is_search(model) => default_name.clone().unwrap_or_default(),
                    "" => String::new(),
                    name => name.to_string(),
                };
                let started = if name.is_empty() {
                    Fill {
                        running: None,
                        said: vec![t!("settings-model-fill-needs-name")],
                    }
                } else {
                    start_fill(model, &name, kind, &ctx)
                };
                editor.fills.insert(id, started);
            }
        }
        for t in editor.tests.values_mut() {
            if let Some(result) = t.running.as_ref().and_then(|rx| rx.try_recv().ok()) {
                t.running = None;
                t.result = Some(result);
            }
        }
        let ServerEditor { server, fills, .. } = &mut editor;
        for (id, f) in fills.iter_mut() {
            if let Some(found) = f.running.as_ref().and_then(|rx| rx.try_recv().ok()) {
                f.running = None;
                if let Some(model) = server.models.iter_mut().find(|m| m.id == *id) {
                    f.said.extend(finish_fill(model, found));
                }
            }
        }

        // A model in use that is gone turns its job off at Save.
        let gone = |at: Option<ModelRef>, search: bool| {
            at.is_some_and(|at| {
                at.server == editor.server.id
                    && !editor
                        .server
                        .model(at.model)
                        .is_some_and(|m| is_search(m) == search)
            })
        };
        let search_gone = d.search_engine == SearchEngine::Server && gone(d.search_model, true);
        let read_gone = d.read_engine == ReadEngine::Server && gone(d.read_model, false);
        if !editor.is_new && (search_gone || read_gone) {
            note(ui, &format!("⚠ {}", t!("settings-server-in-use-removed")));
        }

        let mut agreed = false;
        if let Some((search, reader)) = &editor.ask_send {
            ui.add_space(8.0);
            send_questions(ui, search, reader);
            let mut answer = None;
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, &t!("settings-server-send-and-save")).clicked() {
                    answer = Some(true);
                }
                if ui
                    .button(txt(&t!("settings-server-send-cancel"), 13.0, None))
                    .clicked()
                {
                    answer = Some(false);
                }
            });
            match answer {
                Some(true) => {
                    agreed = true;
                    save = true;
                }
                Some(false) => editor.ask_send = None,
                None => {}
            }
        }

        ui.add_space(14.0);
        let can_save = launching || !editor.server.base_url.trim().is_empty();
        row(ui, |ui| {
            ui.add_space(10.0);
            if !editor.is_new {
                let red = text_shade(RED, ui.visuals().dark_mode);
                if editor.deleting {
                    let question = t!(
                        "settings-server-delete-question",
                        name = editor.server.label()
                    );
                    lbl(ui, &question, 13.5, strong);
                    if ui
                        .button(txt(&t!("settings-server-delete-confirm"), 13.0, Some(red)))
                        .clicked()
                    {
                        delete = true;
                    }
                    if ui.button(txt(&t!("common-cancel"), 13.0, None)).clicked() {
                        editor.deleting = false;
                    }
                } else if ui
                    .add(
                        egui::Button::new(txt(&t!("settings-server-delete"), 13.0, Some(red)))
                            .frame_when_inactive(false),
                    )
                    .clicked()
                {
                    editor.deleting = true;
                }
            }
            far(ui, |ui| {
                ui.add_space(10.0);
                if ui
                    .add_enabled(
                        can_save,
                        egui::Button::new(txt(
                            &t!("settings-server-save"),
                            14.0,
                            Some(Color32::WHITE),
                        ))
                        .fill(super::view::button_fill(accent()))
                        .min_size(Vec2::new(0.0, 32.0)),
                    )
                    .clicked()
                {
                    save = true;
                }
                if ui.button(txt(&t!("common-cancel"), 14.0, None)).clicked() {
                    close = true;
                }
            });
        });
        if !can_save {
            note(ui, &t!("settings-server-needs-address"));
        }

        if delete {
            let id = editor.server.id;
            d.servers.retain(|s| s.id != id);
            if d.search_model.is_some_and(|at| at.server == id) {
                d.search_engine = SearchEngine::Off;
                d.search_model = None;
            }
            if d.read_model.is_some_and(|at| at.server == id) {
                if d.read_engine == ReadEngine::Server {
                    d.read_engine = ReadEngine::Off;
                }
                d.read_model = None;
            }
            return;
        }
        if save && can_save {
            let mut candidate = d.clone();
            match candidate
                .servers
                .iter_mut()
                .find(|s| s.id == editor.server.id)
            {
                Some(s) => *s = editor.server.clone(),
                None => candidate.servers.push(editor.server.clone()),
            }
            if search_gone {
                candidate.search_engine = SearchEngine::Off;
                candidate.search_model = None;
            }
            if read_gone {
                candidate.read_engine = ReadEngine::Off;
                candidate.read_model = None;
            }
            // With no model chosen yet, the server's first one for the job
            // is: turning its engine on then uses it.
            let first = |search: bool| {
                editor
                    .server
                    .models
                    .iter()
                    .find(|m| is_search(m) == search)
                    .map(|m| ModelRef {
                        server: editor.server.id,
                        model: m.id,
                    })
            };
            if !is_model_for(&candidate, candidate.search_model, true) {
                candidate.search_model = first(true);
            }
            if !is_model_for(&candidate, candidate.read_model, false) {
                candidate.read_model = first(false);
            }
            let (search, reader) = self.unagreed(&candidate);
            if agreed {
                for url in search.iter().chain(&reader) {
                    candidate.allow_server(url);
                }
            } else if search.is_some() || reader.is_some() {
                editor.ask_send = Some((search, reader));
                self.server_editor = Some(editor);
                return;
            }
            *d = candidate;
            return;
        }
        if !close {
            self.server_editor = Some(editor);
        }
    }
}

/// A model's settings, under its row.
fn model_settings(
    ui: &mut Ui,
    model: &mut Model,
    listed: &[String],
    default_name: Option<&str>,
    fill: Option<&Fill>,
    fill_clicked: &mut Option<u32>,
) {
    let id = model.id;
    let lookup_name = match model.model.trim() {
        "" => default_name.unwrap_or_default().to_string(),
        name => name.to_string(),
    };
    Frame::new()
        .inner_margin(Margin {
            left: 18,
            right: 0,
            top: 0,
            bottom: 6,
        })
        .show(ui, |ui| {
            plain_setting(
                ui,
                &t!("settings-model-on-the-server"),
                &t!("settings-model-on-the-server-help"),
                |ui| model_field(ui, &mut model.model, listed, default_name),
            );
            plain_setting(
                ui,
                &t!("settings-model-name"),
                &t!("settings-model-name-help"),
                |ui| {
                    let hint = match model.model.trim() {
                        "" => t!("settings-server-default"),
                        name => name.to_string(),
                    };
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut model.name)
                            .hint_text(txt(&hint, 14.0, None))
                            .desired_width(280.0),
                    );
                    set_name(&field, &row_title(ui));
                },
            );
            row(ui, |ui| {
                ui.add_space(10.0);
                let running = fill.is_some_and(|f| f.running.is_some());
                if tip(
                    ui.add_enabled(
                        !running,
                        egui::Button::new(txt(&t!("settings-model-fill"), 13.0, None)),
                    ),
                    &t!("settings-model-fill-help"),
                )
                .clicked()
                {
                    *fill_clicked = Some(id);
                }
                if running {
                    super::view::slow_spinner(ui, 12.0, None);
                    lbl(
                        ui,
                        &t!("settings-model-fill-looking"),
                        12.5,
                        ui.visuals().weak_text_color(),
                    );
                }
            });
            for said in fill.map(|f| f.said.as_slice()).unwrap_or_default() {
                note(ui, said);
            }
            match &mut model.job {
                ModelJob::Search(s) => search_settings(ui, s, &lookup_name),
                ModelJob::Read(r) => read_settings(ui, r),
            }
        });
}

fn search_settings(ui: &mut Ui, s: &mut SearchSettings, model: &str) {
    if let Some((query, document)) = catalog::known_prompts(model) {
        if s.query_prefix != query || s.document_prefix != document {
            row(ui, |ui| {
                ui.add_space(10.0);
                icons::chip(ui, Icon::Info, &t!("settings-model-prompts-tip"), AMBER);
                if ui
                    .button(txt(&t!("settings-server-use-them"), 14.0, None))
                    .clicked()
                {
                    s.query_prefix = query.into();
                    s.document_prefix = document.into();
                }
            });
        }
    }
    setting_for(
        ui,
        &t!("settings-server-query-prompt"),
        &t!("settings-server-query-prompt-help"),
        &mut s.query_prefix,
        String::new(),
        |ui, prefix| {
            let field = ui.add(
                egui::TextEdit::singleline(prefix)
                    .hint_text(txt(&t!("settings-server-none"), 14.0, None))
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    setting_for(
        ui,
        &t!("settings-server-item-prompt"),
        &t!("settings-server-item-prompt-help"),
        &mut s.document_prefix,
        String::new(),
        |ui, prefix| {
            let field = ui.add(
                egui::TextEdit::singleline(prefix)
                    .hint_text(txt(&t!("settings-server-none"), 14.0, None))
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    setting_for(
        ui,
        &t!("settings-server-vector-size"),
        &t!("settings-server-vector-size-help"),
        &mut s.dimensions,
        0,
        |ui, dimensions| {
            let all = t!("settings-vector-all");
            let field = ui.add(
                egui::DragValue::new(dimensions)
                    .range(0..=16384)
                    .speed(8.0)
                    .custom_formatter(move |n, _| {
                        if n == 0.0 {
                            all.clone()
                        } else {
                            format!("{n:.0}")
                        }
                    })
                    .custom_parser(|text| match text.trim() {
                        "" => Some(0.0),
                        number => number.parse::<f64>().ok(),
                    }),
            );
            set_name(&field, &row_title(ui));
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
}

fn read_settings(ui: &mut Ui, r: &mut ReadSettings) {
    plain_setting(
        ui,
        &t!("settings-reader-kind"),
        &t!("settings-reader-kind-help"),
        |ui| {
            let menu = egui::ComboBox::from_id_salt("reader-kind")
                .selected_text(txt(&reader_kind_label(r.kind), 14.0, None))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for kind in ReaderModel::ALL {
                        let label = txt(&reader_kind_label(kind), 14.0, None);
                        if ui.selectable_label(r.kind == kind, label).clicked() && r.kind != kind {
                            // Its usual prompt; it stays editable.
                            r.kind = kind;
                            r.prompt = kind.prompt().to_string();
                        }
                    }
                });
            set_choice_name(&menu.response, &row_title(ui), &reader_kind_label(r.kind));
        },
    );
    plain_setting(
        ui,
        &t!("settings-reader-prompt"),
        &t!("settings-reader-prompt-help"),
        |ui| {
            let field = ui.add(
                egui::TextEdit::multiline(&mut r.prompt)
                    .desired_rows(2)
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
}

fn reader_kind_label(kind: ReaderModel) -> String {
    match kind {
        ReaderModel::Other => t!("reader-kind-other"),
        known => known.name().to_string(),
    }
}

/// llama.cpp only: the app can start llama-server itself.
fn launch_settings(ui: &mut Ui, server: &mut Server) {
    let mut launch = server.launch.is_some();
    setting_for(
        ui,
        &t!("settings-server-launch"),
        &t!("settings-server-launch-help"),
        &mut launch,
        false,
        toggle,
    );
    if launch && server.launch.is_none() {
        server.launch = Some(LaunchConfig {
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
        server.launch = None;
    }
    let Some(l) = &mut server.launch else {
        return;
    };
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
            let field = ui.add(egui::TextEdit::singleline(&mut l.extra_args).desired_width(280.0));
            set_name(&field, &row_title(ui));
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_type_keeps_an_address_the_user_typed() {
        let mut server = Server::default();
        choose_kind(&mut server, ServerKind::Unsloth);
        assert_eq!(
            (server.base_url.as_str(), server.name.as_str()),
            ("http://127.0.0.1:8888/v1", "Unsloth")
        );
        // Unsloth on another port, then another type: the port stays.
        server.base_url = "http://127.0.0.1:3333/v1".into();
        choose_kind(&mut server, ServerKind::Vllm);
        assert_eq!(server.base_url, "http://127.0.0.1:3333/v1");
        assert_eq!(server.name, "vLLM", "the usual name follows the type");
        server.name = "Work PC".into();
        choose_kind(&mut server, ServerKind::Ollama);
        assert_eq!(server.name, "Work PC", "a name the user gave stays");
    }

    #[test]
    fn models_are_called_by_their_name_then_their_id() {
        let mut model = Model::default();
        assert_eq!(model_label(&model), t!("settings-server-default"));
        model.model = "qwen3-embedding-0.6b".into();
        assert_eq!(model_label(&model), "qwen3-embedding-0.6b");
        model.name = "Qwen3 Embedding".into();
        assert_eq!(model_label(&model), "Qwen3 Embedding");
    }

    #[test]
    fn filling_in_takes_the_name_and_keeps_what_was_given() {
        let entry = catalog::Entry {
            id: "nomic-embed-text-v1.5".into(),
            name: "Nomic Embed v1.5".into(),
            ..Default::default()
        };
        let mut model = Model::default();
        finish_fill(&mut model, Ok(Some(entry.clone())));
        assert_eq!(model.name, "Nomic Embed v1.5");
        model.name = "Mine".into();
        finish_fill(&mut model, Ok(Some(entry)));
        assert_eq!(model.name, "Mine");
        assert_eq!(finish_fill(&mut model, Ok(None)).len(), 1);
    }

    #[test]
    fn big_numbers_get_commas() {
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(32768), "32,768");
        assert_eq!(thousands(1_048_576), "1,048,576");
    }
}
