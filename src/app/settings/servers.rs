//! Settings, Search & AI: the saved servers, the editor of a server and its
//! models, and the menus that pick the models in use.

use super::*;
use crate::catalog;
use crate::config::{
    Model, ModelJob, ModelRef, ReadEngine, ReadSettings, SearchEngine, SearchSettings, Server,
    ServerKind,
};
use crate::embed::local::LocalServer;
use crate::embed::server::SearchTest;
use crate::i18n::log_server_error;
use crate::net::ServerError;

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

/// Whether two server addresses name the same place, with `localhost` as
/// 127.0.0.1.
fn same_place(a: &str, b: &str) -> bool {
    let place = |url: &str| {
        crate::net::host(url)
            .to_ascii_lowercase()
            .replace("localhost", "127.0.0.1")
    };
    place(a) == place(b)
}

/// A model `id` as a server's model for searching or reading, with the
/// settings it is known to need: the prompts of a known embedding model,
/// or the reader type and its prompt.
fn found_model(model_id: u32, id: &str, search: bool) -> Model {
    let job = if search {
        let (query, document) = catalog::known_prompts(id).unwrap_or(("", ""));
        ModelJob::Search(SearchSettings {
            query_prefix: query.into(),
            document_prefix: document.into(),
            ..Default::default()
        })
    } else {
        let kind = catalog::reader_kind(id).unwrap_or(ReaderModel::Other);
        ModelJob::Read(ReadSettings {
            kind,
            prompt: kind.prompt().to_string(),
        })
    };
    Model {
        id: model_id,
        model: id.to_string(),
        name: String::new(),
        job,
    }
}

/// A server found on this PC as a saved server, with the models it lists
/// that search or read.
fn found_server(d: &Config, found: &LocalServer) -> Server {
    let mut models = Vec::new();
    for id in &found.models {
        let search = catalog::is_search_model(id);
        if search || catalog::is_reading_model(id) {
            models.push(found_model(models.len() as u32 + 1, id, search));
        }
    }
    Server {
        id: d.next_server_id(),
        kind: found.kind,
        name: found.kind.name().to_string(),
        base_url: found.base_url.clone(),
        api_key: String::new(),
        models,
        launch: None,
    }
}

/// Use on a server found on this PC: saves it with the model `id` and
/// searches by meaning with it (D66).
pub(in crate::app) fn use_found(d: &mut Config, found: &LocalServer, id: &str) {
    let mut server = found_server(d, found);
    server.models = vec![found_model(1, id, true)];
    let at = ModelRef {
        server: server.id,
        model: 1,
    };
    d.servers.push(server);
    d.search_engine = SearchEngine::Server;
    d.search_model = Some(at);
}

/// Adds the model `id` a saved server lists to it, for searching or
/// reading (D67), and says where it is.
fn add_model(d: &mut Config, server: u32, id: &str, search: bool) -> Option<ModelRef> {
    let server = d.servers.iter_mut().find(|s| s.id == server)?;
    let model = found_model(server.next_model_id(), id, search);
    let at = ModelRef {
        server: server.id,
        model: model.id,
    };
    server.models.push(model);
    Some(at)
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
    let width = ui.available_width();
    let places = tile_places(width, ServerKind::ALL.len(), i18n::rtl());
    let height = places
        .iter()
        .map(|place| place.bottom())
        .fold(0.0, f32::max);
    let (area, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    for (index, (kind, place)) in ServerKind::ALL.into_iter().zip(places).enumerate() {
        let selected = server.kind == kind;
        let rect = place.translate(area.min.to_vec2());
        if kind_tile(ui, rect, index, kind, selected).clicked() {
            choose_kind(server, kind);
        }
    }
}

/// How narrow a type's tile can be: seven fit in one row of a card in the
/// widest settings column.
const KIND_TILE_LEAST_WIDTH: f32 = 78.0;

/// How tall a type's tile is.
const KIND_TILE_HEIGHT: f32 = 62.0;

/// Where `count` tiles go in a row `width` wide, from its top left corner:
/// as many to a line as fit, each as wide as the line's room shared among
/// them, so a full line's first tile starts and its last one ends on the
/// row's edges. They are placed from the reading start, from the right
/// when `rtl`.
fn tile_places(width: f32, count: usize, rtl: bool) -> Vec<Rect> {
    let gap = space::SMALL;
    let fit = ((width + gap) / (KIND_TILE_LEAST_WIDTH + gap)).floor() as usize;
    let per_line = fit.clamp(1, count.max(1));
    let tile = ((width - gap * (per_line - 1) as f32) / per_line as f32).max(0.0);
    (0..count)
        .map(|index| {
            let (line, place) = (index / per_line, index % per_line);
            let x = place as f32 * (tile + gap);
            let x = if rtl { width - x - tile } else { x };
            let y = line as f32 * (KIND_TILE_HEIGHT + gap);
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(tile, KIND_TILE_HEIGHT))
        })
        .collect()
}

/// A type's logo above its name, in `rect`, the `index`th of the tiles.
fn kind_tile(
    ui: &mut Ui,
    rect: Rect,
    index: usize,
    kind: ServerKind,
    selected: bool,
) -> egui::Response {
    let name = kind_name(kind);
    let id = ui.scope_id().with(("server-kind", index));
    let resp = ui.interact(rect, id, Sense::click());
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
        CornerRadius::same(radius::MEDIUM),
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
        FontId::proportional(size::SMALL),
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
            .hint_text(txt(&hint, size::BODY, None))
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

/// What sending to servers outside this PC means, before the user agrees,
/// and the answers: a row of the card. `answer` is the main button's text;
/// the answer clicked comes back.
fn send_question_row(
    ui: &mut Ui,
    search: &Option<String>,
    reader: &Option<String>,
    answer: &str,
) -> Option<bool> {
    card_row(ui, false, |ui| {
        let strong = ui.visuals().strong_text_color();
        let font = FontId::proportional(size::CONTROL);
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
        row(ui, |ui| {
            let send = primary_button(ui, answer).clicked();
            let cancel = ui
                .button(txt(&t!("settings-server-send-cancel"), size::CONTROL, None))
                .clicked();
            if send {
                Some(true)
            } else if cancel {
                Some(false)
            } else {
                None
            }
        })
    })
}

/// A test's result, in a row under the model's.
fn test_line(ui: &mut Ui, test: &Test) {
    let Some(result) = &test.result else {
        return;
    };
    card_row(ui, false, |ui| test_result(ui, result));
}

fn test_result(ui: &mut Ui, result: &Result<Tested, String>) {
    let seconds = |took: &Duration| format!("{:.1}", took.as_secs_f32());
    super::view::row_wrapped(ui, |ui| {
        match result {
            Ok(Tested::Search(t)) => {
                let works = if t.vectors > 1 {
                    t!(
                        "settings-model-test-multi",
                        vectors = t.vectors,
                        size = t.size,
                        seconds = seconds(&t.took)
                    )
                } else {
                    t!(
                        "settings-model-test-search",
                        size = t.size,
                        seconds = seconds(&t.took)
                    )
                };
                icons::chip(ui, Icon::Check, &works, GREEN)
            }
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
    // Whether images work, on a line of its own.
    let Ok(Tested::Search(SearchTest {
        images: Some(images),
        ..
    })) = result
    else {
        return;
    };
    super::view::row_wrapped(ui, |ui| {
        match images {
            Ok(()) => icons::chip(ui, Icon::Check, &t!("settings-model-test-images"), GREEN),
            Err(e) => {
                let text = match e.downcast_ref::<ServerError>() {
                    Some(ServerError::NoImages { said, .. }) => {
                        t!("settings-model-test-no-images", said = said.as_str())
                    }
                    _ => t!(
                        "settings-model-test-images-failed",
                        error = crate::net::error_text(e)
                    ),
                };
                icons::chip(ui, Icon::Warning, &text, AMBER)
            }
        };
    });
}

impl App {
    /// The servers, below the engines: one row each, then "Add server".
    /// Settings, Servers: the ones found on this PC and the saved ones, or
    /// the editor of one (D68).
    pub(super) fn servers_page(&mut self, ui: &mut Ui, d: &mut Config) {
        if self.server_editor.is_some() {
            self.server_editor_page(ui, d);
            return;
        }
        self.local_servers.look(ui.ctx());
        self.found_section(ui, d);
        self.servers_section(ui, d);
    }

    /// The model servers found on this PC that aren't saved yet.
    fn unsaved_local(&self, d: &Config) -> Vec<LocalServer> {
        self.local_servers
            .found
            .iter()
            .filter(|f| {
                !d.servers
                    .iter()
                    .any(|s| same_place(&s.base_url, &f.base_url))
            })
            .cloned()
            .collect()
    }

    /// Each model server found on this PC with its search models, and Use
    /// beside each, which saves the server with that model and uses it (D66):
    /// a row of the card each.
    pub(super) fn found_rows(&mut self, ui: &mut Ui, d: &mut Config) {
        let found: Vec<LocalServer> = self
            .unsaved_local(d)
            .into_iter()
            .filter(|f| f.models.iter().any(|m| catalog::is_search_model(m)))
            .collect();
        let mut used = None;
        let v = ui.visuals().clone();
        for server in &found {
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    icons::logo(ui, server.kind, 20.0, v.strong_text_color());
                    let running = t!("settings-found-running", server = server.kind.name());
                    title_lbl(ui, &running, size::BODY, v.strong_text_color());
                    far(ui, |ui| {
                        let host = crate::net::host(&server.base_url);
                        lbl(ui, host, size::SMALL, v.weak_text_color());
                    });
                });
                let models = server.models.iter().filter(|m| catalog::is_search_model(m));
                for (i, model) in models.enumerate() {
                    row(ui, |ui| {
                        ui.set_min_height(34.0);
                        // Under the server's name, past its logo.
                        ui.add_space(space::HUGE + space::TINY);
                        lbl(ui, model, size::BODY, v.text_color());
                        chip(ui, t!("settings-found-search"), accent());
                        far(ui, |ui| {
                            let text = t!("settings-found-use");
                            let r = if i == 0 {
                                primary_button(ui, &text)
                            } else {
                                ui.button(txt(&text, size::CONTROL, None))
                            };
                            if r.clicked() {
                                used = Some((server.clone(), model.clone()));
                            }
                        });
                    });
                }
            });
        }
        if let Some((server, model)) = used {
            use_found(d, &server, &model);
        }
    }

    /// Settings, Servers: the model servers found on this PC, each with
    /// Add, which opens the editor filled in with it.
    fn found_section(&mut self, ui: &mut Ui, d: &Config) {
        let found = self.unsaved_local(d);
        if found.is_empty() {
            return;
        }
        let mut add = None;
        let weak = ui.visuals().weak_text_color();
        let strong = ui.visuals().strong_text_color();
        section(ui, &t!("settings-found-title"), |ui| {
            for server in &found {
                card_row(ui, false, |ui| {
                    row(ui, |ui| {
                        ui.set_min_height(30.0);
                        icons::logo(ui, server.kind, 20.0, strong);
                        ui.add_space(space::TINY);
                        lbl(ui, server.kind.name(), size::BODY, strong);
                        far(ui, |ui| {
                            if ui
                                .button(txt(&t!("settings-found-add"), size::CONTROL, None))
                                .clicked()
                            {
                                add = Some(server.clone());
                            }
                            let what = if server.needs_key {
                                t!("settings-found-needs-key")
                            } else {
                                t!("settings-server-models-count", count = server.models.len())
                            };
                            lbl(ui, &what, size::SMALL, weak);
                            lbl(ui, crate::net::host(&server.base_url), size::SMALL, weak);
                        });
                    });
                });
            }
        });
        if let Some(found) = add {
            let mut server = found_server(d, &found);
            // Its search models to start with; the editor adds the others.
            server.models.retain(is_search);
            self.server_editor = Some(ServerEditor::new(server, true));
            self.settings_page = SettingsPage::Servers;
        }
    }

    pub(super) fn servers_section(&mut self, ui: &mut Ui, d: &Config) {
        let mut edit = None;
        let mut discard = None;
        let weak = ui.visuals().weak_text_color();
        let strong = ui.visuals().strong_text_color();
        // A server added and left before its first Save shows too.
        let new: Vec<Server> = self
            .server_drafts
            .values()
            .filter(|e| !d.servers.iter().any(|s| s.id == e.server.id))
            .map(|e| e.server.clone())
            .collect();
        section(ui, &t!("settings-servers-section"), |ui| {
            if d.servers.is_empty() {
                note(ui, &t!("settings-servers-none"));
            }
            for server in d.servers.iter().chain(&new) {
                let drafted = self.server_drafts.contains_key(&server.id);
                ui.push_id(server.id, |ui| {
                    card_row(ui, false, |ui| {
                        row(ui, |ui| {
                            ui.set_min_height(30.0);
                            icons::logo(ui, server.kind, 20.0, strong);
                            ui.add_space(space::TINY);
                            let name = if server.label().trim().is_empty() {
                                t!("settings-server-add-title")
                            } else {
                                server.label().to_string()
                            };
                            lbl(ui, &name, size::BODY, strong);
                            far(ui, |ui| {
                                if ui
                                    .button(txt(&t!("settings-server-edit"), size::CONTROL, None))
                                    .clicked()
                                {
                                    edit = Some(server.clone());
                                }
                                // Left with changes: they wait here (D61).
                                if drafted {
                                    if ui
                                        .button(txt(
                                            &t!("settings-server-discard"),
                                            size::CONTROL,
                                            None,
                                        ))
                                        .clicked()
                                    {
                                        discard = Some(server.id);
                                    }
                                    chip(ui, t!("settings-server-not-saved"), AMBER);
                                }
                                let count = server.models.len();
                                lbl(
                                    ui,
                                    &t!("settings-server-models-count", count = count),
                                    size::SMALL,
                                    weak,
                                );
                                lbl(ui, crate::net::host(&server.base_url), size::SMALL, weak);
                            });
                        });
                    });
                });
            }
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    if ui
                        .button(txt(&t!("settings-server-add"), size::CONTROL, None))
                        .clicked()
                    {
                        self.add_server(d);
                    }
                });
            });
        });
        if let Some(id) = discard {
            self.server_drafts.remove(&id);
        }
        if let Some(server) = edit {
            let editor = self
                .server_drafts
                .remove(&server.id)
                .unwrap_or_else(|| ServerEditor::new(server, false));
            self.server_editor = Some(editor);
            self.settings_page = SettingsPage::Servers;
        }
    }

    /// Puts the open server editor away. Changes not saved stay as a draft,
    /// shown on the server's row until Save or Discard (D61). True if an
    /// editor was open.
    pub(in crate::app) fn stash_server_editor(&mut self) -> bool {
        let Some(editor) = self.server_editor.take() else {
            return false;
        };
        let config = self.draft.as_ref().unwrap_or(&self.cfg);
        let changed = match config.servers.iter().find(|s| s.id == editor.server.id) {
            Some(saved) => *saved != editor.server,
            None => {
                editor.server
                    != Server {
                        id: editor.server.id,
                        ..Default::default()
                    }
            }
        };
        if changed {
            self.server_drafts.insert(editor.server.id, editor);
        }
        true
    }

    pub(super) fn add_server(&mut self, d: &Config) {
        let server = Server {
            id: d.next_server_id(),
            ..Default::default()
        };
        self.server_editor = Some(ServerEditor::new(server, true));
        // The editor is part of the Servers page.
        self.settings_page = SettingsPage::Servers;
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
        let (line, details) = if search {
            (
                t!("settings-search-model-hint"),
                t!("settings-search-model-help"),
            )
        } else {
            (
                t!("settings-read-model-hint"),
                t!("settings-read-model-help"),
            )
        };
        let in_use = chosen
            .and_then(|at| d.server(at.server))
            .map(|server| server.kind);
        // Models a saved server has that aren't added to it yet (D67).
        let mut added: Option<(u32, String)> = None;
        let fits = |id: &str| {
            if search {
                catalog::is_search_model(id)
            } else {
                catalog::is_reading_model(id)
            }
        };
        let others: Vec<(u32, Vec<String>)> = d
            .servers
            .iter()
            .map(|server| {
                let more = self
                    .server_lists
                    .models(server)
                    .iter()
                    .filter(|id| fits(id))
                    .filter(|id| {
                        !server
                            .models
                            .iter()
                            .any(|m| m.model.eq_ignore_ascii_case(id))
                    })
                    .cloned()
                    .collect();
                (server.id, more)
            })
            .collect();
        plain_setting(ui, &t!("settings-model"), &line, &details, |ui| {
            let weak = ui.visuals().weak_text_color();
            let strong = ui.visuals().strong_text_color();
            // The server's logo inside the menu, before the model's name.
            let shown = txt(&current, size::BODY, None);
            field_look(ui);
            let menu =
                egui::ComboBox::from_id_salt(if search { "search-model" } else { "read-model" })
                    .icon(menu_arrow);
            let menu = match in_use {
                Some(kind) => {
                    let logo = egui::Atom::paint(Vec2::splat(18.0), move |ui, args| {
                        icons::paint_logo(ui.painter(), args.rect, kind, strong)
                    });
                    menu.selected_text((logo, shown))
                }
                None => menu.selected_text(shown),
            };
            let menu = menu.width(MENU_WIDTH).show_ui(ui, |ui| {
                for server in &d.servers {
                    let models: Vec<&Model> = server
                        .models
                        .iter()
                        .filter(|m| is_search(m) == search)
                        .collect();
                    let more = others
                        .iter()
                        .find(|(id, _)| *id == server.id)
                        .map_or(&[][..], |(_, more)| more.as_slice());
                    if models.is_empty() && more.is_empty() {
                        continue;
                    }
                    row(ui, |ui| {
                        icons::logo(ui, server.kind, 14.0, weak);
                        lbl(ui, &server.label().to_uppercase(), size::SMALL, weak);
                    });
                    for m in models {
                        let at = ModelRef {
                            server: server.id,
                            model: m.id,
                        };
                        let label = txt(&model_label(m), size::BODY, None);
                        if ui.selectable_label(chosen == Some(at), label).clicked() {
                            picked = Some(at);
                        }
                    }
                    // The ones it has that aren't added yet, marked so.
                    for id in more {
                        let note = (t!("settings-model-on-server-only"), weak);
                        let label = pill_job(id, size::BODY, strong, Some(note));
                        if ui.selectable_label(false, label).clicked() {
                            added = Some((server.id, id.clone()));
                        }
                    }
                }
                divider(ui);
                if ui
                    .selectable_label(
                        false,
                        txt(&t!("settings-model-add-server"), size::BODY, None),
                    )
                    .clicked()
                {
                    add = true;
                }
            });
            set_choice_name(&menu.response, &row_title(ui), &current);
        });
        if let Some((server, id)) = added {
            picked = add_model(d, server, &id, search);
        }
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
        let answer = send_question_row(
            ui,
            &pending.search,
            &pending.reader,
            &t!("settings-server-send"),
        );
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

    /// The editor of one server, drawn in place of the Servers page.
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
        ui.add_space(space::SMALL);
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
            title_lbl(ui, &title, size::LARGE, strong);
        });

        let can_save = section(ui, &t!("settings-server-kind"), |ui| {
            let ServerEditor {
                server,
                found,
                show_key,
                ..
            } = &mut editor;
            card_row(ui, false, |ui| kind_tiles(ui, server));
            plain_setting(
                ui,
                &t!("settings-server-name"),
                &t!("settings-server-name-hint"),
                "",
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
            // The address, and under it what the server said.
            card_row(ui, false, |ui| {
                let mut check = false;
                setting_content(
                    ui,
                    &t!("settings-server-address"),
                    &t!("settings-server-address-hint"),
                    &t!("settings-server-address-help"),
                    false,
                    false,
                    |ui| {
                        if !launching
                            && ui
                                .button(txt(
                                    &t!("settings-server-test-connection"),
                                    size::CONTROL,
                                    None,
                                ))
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
                found.poll();
                if check {
                    found.fetch(&server.base_url, &server.api_key, embedding_list, &ctx);
                } else if !launching {
                    found.fetch_when_settled(
                        &server.base_url,
                        &server.api_key,
                        embedding_list,
                        &ctx,
                    );
                }
                let said = launching
                    || found.pending.is_some()
                    || found.error.is_some()
                    || !found.url.is_empty();
                if said {
                    super::view::row_wrapped(ui, |ui| {
                        if launching {
                            lbl(ui, &t!("settings-server-launch-note"), size::SMALL, weak);
                        } else if found.pending.is_some() {
                            super::view::slow_spinner(ui, 12.0, None);
                            lbl(ui, &t!("settings-server-checking"), size::SMALL, weak);
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
                }
            });
            // Save waits for an address; this says why.
            let can_save = launching || !server.base_url.trim().is_empty();
            if !can_save {
                note(ui, &t!("settings-server-needs-address"));
            }
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
                warning(ui, &text);
            }
            plain_setting(
                ui,
                &t!("settings-server-key"),
                &t!("settings-server-key-hint"),
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
                            .hint_text(txt(&hint, size::BODY, None))
                            .desired_width(280.0),
                    );
                    set_name(&field, &row_title(ui));
                },
            );
            if server.kind == ServerKind::LlamaCpp {
                launch_settings(ui, server);
            }
            can_save
        });

        let (search_gone, read_gone) = section(ui, &t!("settings-server-models-section"), |ui| {
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
                    setting_row(ui, &model_label(model), "", "", open, false, |ui| {
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
                            ui.button(txt(&t!("settings-model-test"), size::CONTROL, None)),
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
                        if icons::icon_button_ex(
                            ui,
                            icon,
                            &label,
                            Vec2::splat(28.0),
                            15.0,
                            None,
                            None,
                        )
                        .clicked()
                        {
                            editor.open = if open { None } else { Some(id) };
                        }
                        chip(ui, job, accent());
                    });
                    // Its settings and its test's result belong to it.
                    nested(ui, |ui| {
                        if open {
                            // The server's default model is the one it embeds with.
                            let (listed, default) = if is_search(model) {
                                (found.for_search(), default_name.as_deref())
                            } else {
                                (found.listed.as_slice(), None)
                            };
                            model_settings(
                                ui,
                                model,
                                listed,
                                default,
                                editor.fills.get(&id),
                                &mut fill,
                            );
                        }
                        if let Some(t) = editor.tests.get(&id) {
                            test_line(ui, t);
                        }
                    });
                });
            }
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    for (search, label) in [
                        (true, t!("settings-model-add-search")),
                        (false, t!("settings-model-add-read")),
                    ] {
                        if ui.button(txt(&label, size::CONTROL, None)).clicked() {
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
                warning(ui, &t!("settings-server-in-use-removed"));
            }
            (search_gone, read_gone)
        });

        let mut agreed = false;
        if let Some((search, reader)) = &editor.ask_send {
            ui.add_space(space::HUGE);
            let answer = card(ui, |ui| {
                send_question_row(ui, search, reader, &t!("settings-server-send-and-save"))
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

        // Save and Cancel on the page under the cards, on their edges.
        ui.add_space(space::LARGE);
        row(ui, |ui| {
            if !editor.is_new {
                let red = text_shade(RED, ui.visuals().dark_mode);
                if editor.deleting {
                    let question = t!(
                        "settings-server-delete-question",
                        name = editor.server.label()
                    );
                    lbl(ui, &question, size::CONTROL, strong);
                    if ui
                        .button(txt(
                            &t!("settings-server-delete-confirm"),
                            size::CONTROL,
                            Some(red),
                        ))
                        .clicked()
                    {
                        delete = true;
                    }
                    if ui
                        .button(txt(&t!("common-cancel"), size::CONTROL, None))
                        .clicked()
                    {
                        editor.deleting = false;
                    }
                } else if ui
                    .add(
                        egui::Button::new(txt(
                            &t!("settings-server-delete"),
                            size::CONTROL,
                            Some(red),
                        ))
                        .frame_when_inactive(false),
                    )
                    .clicked()
                {
                    editor.deleting = true;
                }
            }
            far(ui, |ui| {
                if ui
                    .add_enabled(
                        can_save,
                        egui::Button::new(txt(
                            &t!("settings-server-save"),
                            size::BODY,
                            Some(Color32::WHITE),
                        ))
                        .fill(super::view::button_fill(accent()))
                        .min_size(Vec2::new(0.0, 32.0)),
                    )
                    .react(ui)
                    .clicked()
                {
                    save = true;
                }
                if ui
                    .button(txt(&t!("common-cancel"), size::BODY, None))
                    .clicked()
                {
                    close = true;
                }
            });
        });

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
        self.server_editor = Some(editor);
        if close {
            self.stash_server_editor();
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
    plain_setting(
        ui,
        &t!("settings-model-on-the-server"),
        &t!("settings-model-on-the-server-hint"),
        &t!("settings-model-on-the-server-help"),
        |ui| model_field(ui, &mut model.model, listed, default_name),
    );
    plain_setting(
        ui,
        &t!("settings-model-name"),
        &t!("settings-model-name-hint"),
        &t!("settings-model-name-help"),
        |ui| {
            let hint = match model.model.trim() {
                "" => t!("settings-server-default"),
                name => name.to_string(),
            };
            let field = ui.add(
                egui::TextEdit::singleline(&mut model.name)
                    .hint_text(txt(&hint, size::BODY, None))
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    card_row(ui, false, |ui| {
        row(ui, |ui| {
            let running = fill.is_some_and(|f| f.running.is_some());
            if tip(
                ui.add_enabled(
                    !running,
                    egui::Button::new(txt(&t!("settings-model-fill"), size::CONTROL, None)),
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
                    size::SMALL,
                    ui.visuals().weak_text_color(),
                );
            }
        });
    });
    for said in fill.map(|f| f.said.as_slice()).unwrap_or_default() {
        note(ui, said);
    }
    match &mut model.job {
        ModelJob::Search(s) => search_settings(ui, s, &lookup_name),
        ModelJob::Read(r) => read_settings(ui, r),
    }
}

fn search_settings(ui: &mut Ui, s: &mut SearchSettings, model: &str) {
    if let Some((query, document)) = catalog::known_prompts(model) {
        if s.query_prefix != query || s.document_prefix != document {
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    icons::chip(ui, Icon::Info, &t!("settings-model-prompts-tip"), AMBER);
                    if ui
                        .button(txt(&t!("settings-server-use-them"), size::BODY, None))
                        .clicked()
                    {
                        s.query_prefix = query.into();
                        s.document_prefix = document.into();
                    }
                });
            });
        }
    }
    setting_for(
        ui,
        &t!("settings-server-query-prompt"),
        &t!("settings-server-query-prompt-hint"),
        &t!("settings-server-query-prompt-help"),
        &mut s.query_prefix,
        String::new(),
        |ui, prefix| {
            let field = ui.add(
                egui::TextEdit::singleline(prefix)
                    .hint_text(txt(&t!("settings-server-none"), size::BODY, None))
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    setting_for(
        ui,
        &t!("settings-server-item-prompt"),
        &t!("settings-server-item-prompt-hint"),
        "",
        &mut s.document_prefix,
        String::new(),
        |ui, prefix| {
            let field = ui.add(
                egui::TextEdit::singleline(prefix)
                    .hint_text(txt(&t!("settings-server-none"), size::BODY, None))
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    setting_for(
        ui,
        &t!("settings-server-vector-size"),
        &t!("settings-server-vector-size-hint"),
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
        &t!("settings-server-images-hint"),
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
        &t!("settings-reader-kind-hint"),
        &t!("settings-reader-kind-help"),
        |ui| {
            field_look(ui);
            let menu = egui::ComboBox::from_id_salt("reader-kind")
                .icon(menu_arrow)
                .selected_text(txt(&reader_kind_label(r.kind), size::BODY, None))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for kind in ReaderModel::ALL {
                        let label = txt(&reader_kind_label(kind), size::BODY, None);
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
        &t!("settings-reader-prompt-hint"),
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
    // One llama-server runs one model.
    let has = |search: bool| {
        server
            .models
            .iter()
            .any(|m| matches!(m.job, ModelJob::Search(_)) == search)
    };
    let both = has(true) && has(false);
    let mut launch = server.launch.is_some();
    setting_for(
        ui,
        &t!("settings-server-launch"),
        &t!("settings-server-launch-hint"),
        "",
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
            hf: String::new(),
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
    if both {
        card_row(ui, false, |ui| {
            super::view::row_wrapped(ui, |ui| {
                icons::chip(ui, Icon::Warning, &t!("settings-server-one-model"), AMBER);
            });
        });
    }
    plain_setting(
        ui,
        &t!("settings-server-program"),
        &t!("settings-server-program-hint"),
        "",
        |ui| path_field(ui, &mut l.llama_server, &["exe"]),
    );
    plain_setting(ui, &t!("settings-server-model-file"), "", "", |ui| {
        path_field(ui, &mut l.gguf, &["gguf"])
    });
    plain_setting(
        ui,
        &t!("settings-server-hf"),
        &t!("settings-server-hf-hint"),
        "",
        |ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut l.hf)
                    .hint_text("ggml-org/GLM-OCR-GGUF")
                    .desired_width(280.0),
            );
            set_name(&field, &row_title(ui));
        },
    );
    plain_setting(
        ui,
        &t!("settings-server-projector"),
        &t!("settings-server-projector-hint"),
        "",
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
    plain_setting(ui, &t!("settings-server-port"), "", "", |ui| {
        ui.add(egui::DragValue::new(&mut l.port).range(1024..=65535));
    });
    plain_setting(
        ui,
        &t!("settings-server-extra-args"),
        &t!("settings-server-extra-args-hint"),
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

    /// In a card of the widest settings column, the seven types of server
    /// fit in one row of tiles, which reaches from the row's start to its
    /// end.
    #[test]
    fn the_server_types_fit_in_one_row() {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.enable_accesskit();
        // The fonts take effect from the next frame; the font atlas uploads
        // no renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut server = Server::default();
        let mut row = Rect::NOTHING;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let column = Rect::from_min_size(
                ui.max_rect().min,
                Vec2::new(super::super::COLUMN_WIDTH, 400.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                card(ui, |ui| {
                    row = card_row(ui, false, |ui| {
                        kind_tiles(ui, &mut server);
                        ui.max_rect()
                    });
                });
            });
        });
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.expect("a tree");
        let tops: Vec<f64> = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == egui::accesskit::Role::RadioButton)
            .filter_map(|(_, node)| node.bounds().map(|bounds| bounds.y0))
            .collect();
        assert_eq!(tops.len(), ServerKind::ALL.len());
        assert!(tops.iter().all(|top| *top == tops[0]), "{tops:?}");
        let tiles = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == egui::accesskit::Role::RadioButton)
            .filter_map(|(_, node)| node.bounds());
        let (start, end) = tiles.fold((f64::INFINITY, f64::NEG_INFINITY), |(start, end), b| {
            (start.min(b.x0), end.max(b.x1))
        });
        assert!((start as f32 - row.left()).abs() <= 0.5, "{start} {row:?}");
        assert!((end as f32 - row.right()).abs() <= 0.5, "{end} {row:?}");
    }

    #[test]
    fn type_tiles_share_their_lines_from_the_reading_start() {
        let lines = |places: &[Rect]| {
            let mut tops: Vec<f32> = places.iter().map(|place| place.top()).collect();
            tops.dedup();
            tops.len()
        };
        for rtl in [false, true] {
            let places = tile_places(606.0, 7, rtl);
            assert_eq!(lines(&places), 1);
            let (first, last) = (places[0], places[6]);
            let (start, end) = if rtl {
                (606.0 - first.right(), last.left())
            } else {
                (first.left(), 606.0 - last.right())
            };
            assert!(start.abs() < 0.01 && end.abs() < 0.01, "{places:?}");
            // In a narrow column, two to a line, the last alone.
            let places = tile_places(242.0, 7, rtl);
            assert_eq!(lines(&places), 4);
            assert!((places[1].width() - (242.0 - space::SMALL) / 2.0).abs() < 0.01);
        }
    }

    #[test]
    fn big_numbers_get_commas() {
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(32768), "32,768");
        assert_eq!(thousands(1_048_576), "1,048,576");
    }
}
