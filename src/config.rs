//! User settings, persisted as JSON in the data directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::i18n::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Reciprocal-rank fusion of keyword and embedding results.
    Hybrid,
    Keyword,
    Semantic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// The meaning-search model in use, as the worker needs it.
#[derive(Clone, Debug, PartialEq)]
pub enum EmbedBackend {
    /// No semantic search: keyword + OCR only.
    None,
    /// Any OpenAI-compatible `/v1/embeddings` server (llama.cpp
    /// `llama-server --embeddings`, Ollama, LM Studio, vLLM, ...).
    Server(Box<ServerConfig>),
}

/// A search model with its server, put together from a saved server and
/// one of its models (see `Config::search_backend`).
#[derive(Clone, PartialEq)]
pub struct ServerConfig {
    /// What the server is: OpenRouter gets requests of its own.
    pub server_kind: ServerKind,
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty.
    pub api_key: String,
    pub model: String,
    pub query_prefix: String,
    pub document_prefix: String,
    /// Send images to the server (vLLM-style `messages` with a data-URI image).
    pub images: bool,
    /// Matryoshka truncation; 0 keeps the full vector.
    pub truncate_dims: usize,
    /// Optionally have clipboard-manager start `llama-server` itself.
    pub launch: Option<LaunchConfig>,
}

/// What reads the text in images, as the worker needs it.
#[derive(Clone, Debug, PartialEq)]
pub enum TextReader {
    /// Images are not read.
    Off,
    /// Windows' own reader, in the languages installed in Windows.
    Windows,
    /// A vision model on a server, asked with a prompt.
    Server(ReaderServer),
}

/// A vision model the app knows, for a text reader server: each gets the
/// prompt and request settings that suit it (see `ocr::server`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderModel {
    DeepSeekOcr2,
    UnlimitedOcr2,
    MiniCpmOcr,
    GlmOcr,
    /// Any other vision model, asked with the prompt as written.
    #[default]
    Other,
}

/// A reading model with its server, put together from a saved server and
/// one of its models (see `Config::text_reader`): any OpenAI-style chat
/// server with a vision model.
#[derive(Clone, PartialEq)]
pub struct ReaderServer {
    /// What the server is: OpenRouter gets requests of its own.
    pub server_kind: ServerKind,
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty.
    pub api_key: String,
    /// Empty for the server's default model.
    pub model: String,
    /// Which known model the server runs, if any.
    pub kind: ReaderModel,
    /// What the model is asked, with the image.
    pub prompt: String,
}

impl ReaderServer {
    pub const PROMPT: &'static str =
        "Read all the text in this image. Reply with the text only, line by line.";
}

impl Default for ReaderServer {
    fn default() -> Self {
        Self {
            server_kind: ServerKind::Other,
            base_url: "http://127.0.0.1:8080/v1".into(),
            api_key: String::new(),
            model: String::new(),
            kind: ReaderModel::Other,
            prompt: Self::PROMPT.into(),
        }
    }
}

/// Written by hand so the API key never reaches a log.
impl std::fmt::Debug for ReaderServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReaderServer")
            .field("server_kind", &self.server_kind)
            .field("base_url", &self.base_url)
            .field("api_key", &hidden(&self.api_key))
            .field("model", &self.model)
            .field("kind", &self.kind)
            .field("prompt", &self.prompt)
            .finish()
    }
}

/// How a key shows in a log: whether there is one, never what it is.
fn hidden(key: &str) -> &'static str {
    if key.is_empty() {
        ""
    } else {
        "<hidden>"
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchConfig {
    pub llama_server: PathBuf,
    pub gguf: PathBuf,
    /// Multimodal projector, needed for image embeddings with some GGUF models.
    pub mmproj: Option<PathBuf>,
    pub port: u16,
    pub extra_args: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::plain("")
    }
}

/// Written by hand so the API key never reaches a log.
impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerConfig")
            .field("server_kind", &self.server_kind)
            .field("base_url", &self.base_url)
            .field("api_key", &hidden(&self.api_key))
            .field("model", &self.model)
            .field("query_prefix", &self.query_prefix)
            .field("document_prefix", &self.document_prefix)
            .field("images", &self.images)
            .field("truncate_dims", &self.truncate_dims)
            .field("launch", &self.launch)
            .finish()
    }
}

impl ServerConfig {
    pub fn plain(base_url: &str) -> Self {
        Self {
            server_kind: ServerKind::Other,
            base_url: base_url.into(),
            api_key: String::new(),
            model: String::new(),
            query_prefix: String::new(),
            document_prefix: String::new(),
            images: false,
            truncate_dims: 0,
            launch: None,
        }
    }
}

/// What a saved server is: it fills in the usual address, picks the logo,
/// and tells where the server lists its embedding models.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerKind {
    LlamaCpp,
    Ollama,
    LmStudio,
    Vllm,
    Unsloth,
    OpenRouter,
    /// Any other server with an OpenAI-style API.
    #[default]
    Other,
}

impl ServerKind {
    /// In the order Settings offers them.
    pub const ALL: [ServerKind; 7] = [
        Self::LlamaCpp,
        Self::Ollama,
        Self::LmStudio,
        Self::Vllm,
        Self::Unsloth,
        Self::OpenRouter,
        Self::Other,
    ];

    /// Its name; empty for another server, which the app words itself.
    pub fn name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::Vllm => "vLLM",
            Self::Unsloth => "Unsloth",
            Self::OpenRouter => "OpenRouter",
            Self::Other => "",
        }
    }

    /// The address it serves at unless told otherwise; empty for another
    /// server.
    pub fn address(self) -> &'static str {
        match self {
            Self::LlamaCpp => "http://127.0.0.1:8080/v1",
            Self::Ollama => "http://127.0.0.1:11434/v1",
            Self::LmStudio => "http://127.0.0.1:1234/v1",
            Self::Vllm => "http://127.0.0.1:8000/v1",
            Self::Unsloth => "http://127.0.0.1:8888/v1",
            Self::OpenRouter => "https://openrouter.ai/api/v1",
            Self::Other => "",
        }
    }

    /// What a server saved before kinds existed most likely is, from its
    /// address: OpenRouter's, or the usual port of each server.
    fn from_address(url: &str) -> Self {
        let host = crate::net::host(url).to_ascii_lowercase();
        if host == "openrouter.ai" || host.ends_with(".openrouter.ai") {
            return Self::OpenRouter;
        }
        match port(&host) {
            Some(8080) => Self::LlamaCpp,
            Some(11434) => Self::Ollama,
            Some(1234) => Self::LmStudio,
            Some(8000) => Self::Vllm,
            Some(8888) => Self::Unsloth,
            _ => Self::Other,
        }
    }
}

/// The port of a `host:port`, as `net::host` gives it.
fn port(host: &str) -> Option<u16> {
    host.rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
}

/// A saved server: where it is, its key, and the models used on it.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Server {
    /// Stays the same when the server is renamed or moved.
    pub id: u32,
    pub kind: ServerKind,
    pub name: String,
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty. Encrypted in the
    /// settings file (see secure.rs).
    #[serde(default, with = "crate::secure::string")]
    pub api_key: String,
    pub models: Vec<Model>,
    /// The app starts `llama-server` itself with these (llama.cpp only).
    pub launch: Option<LaunchConfig>,
}

/// Written by hand so the API key never reaches a log.
impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("base_url", &self.base_url)
            .field("api_key", &hidden(&self.api_key))
            .field("models", &self.models)
            .field("launch", &self.launch)
            .finish()
    }
}

impl Server {
    /// What the server is called: its name, or else its address.
    pub fn label(&self) -> &str {
        match self.name.trim() {
            "" => crate::net::host(&self.base_url),
            name => name,
        }
    }

    pub fn model(&self, id: u32) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }

    /// The id a new model on this server gets.
    pub fn next_model_id(&self) -> u32 {
        self.models.iter().map(|m| m.id).max().unwrap_or(0) + 1
    }

    /// One of its search models with the server, as the worker needs it.
    pub fn search_config(&self, model: &Model) -> Option<ServerConfig> {
        let ModelJob::Search(s) = &model.job else {
            return None;
        };
        Some(ServerConfig {
            server_kind: self.kind,
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: model.model.clone(),
            query_prefix: s.query_prefix.clone(),
            document_prefix: s.document_prefix.clone(),
            images: s.images,
            truncate_dims: s.dimensions,
            launch: self.launch.clone(),
        })
    }

    /// One of its reading models with the server, as the worker needs it.
    pub fn read_config(&self, model: &Model) -> Option<ReaderServer> {
        let ModelJob::Read(r) = &model.job else {
            return None;
        };
        Some(ReaderServer {
            server_kind: self.kind,
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: model.model.clone(),
            kind: r.kind,
            prompt: r.prompt.clone(),
        })
    }
}

/// A model used on a server, for searching or for reading text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Model {
    /// Unique among the server's models.
    pub id: u32,
    /// The model's name on the server; empty for the server's default model.
    pub model: String,
    /// The name menus show; empty shows the model's own name.
    pub name: String,
    pub job: ModelJob,
}

/// What a model does, with its settings for that.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModelJob {
    /// Makes the vectors meaning search compares.
    Search(SearchSettings),
    /// Reads the text in images.
    Read(ReadSettings),
}

impl Default for ModelJob {
    fn default() -> Self {
        Self::Search(SearchSettings::default())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    /// Put before each search, as some models need (EmbeddingGemma).
    pub query_prefix: String,
    /// Put before each item's text.
    pub document_prefix: String,
    /// Send images to the server too.
    pub images: bool,
    /// How many numbers of each vector to keep (Matryoshka); 0 keeps them all.
    pub dimensions: usize,
}

impl SearchSettings {
    pub const GEMMA_QUERY: &'static str = "task: search result | query: ";
    pub const GEMMA_DOCUMENT: &'static str = "title: none | text: ";
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadSettings {
    /// Which known model it is, if any.
    pub kind: ReaderModel,
    /// What the model is asked, with the image.
    pub prompt: String,
}

impl Default for ReadSettings {
    fn default() -> Self {
        Self {
            kind: ReaderModel::Other,
            prompt: ReaderServer::PROMPT.into(),
        }
    }
}

/// A model on a saved server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    pub server: u32,
    pub model: u32,
}

/// Where meaning search runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchEngine {
    /// Search by words only.
    #[default]
    Off,
    /// A model on one of the saved servers.
    Server,
}

/// What reads the text in images.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadEngine {
    Off,
    /// Windows' own reader.
    Windows,
    /// A vision model on one of the saved servers.
    Server,
}

impl Default for ReadEngine {
    /// Only Windows has a reader of its own.
    fn default() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Off
        }
    }
}

/// Addresses that differ only in case or a trailing slash are the same server.
fn address_key(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Density {
    Comfortable,
    Compact,
}

/// Interface language. Arabic also lays the window out right to left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    Arabic,
}

/// How clock times are shown. Items are stored in UTC either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    /// 14:30
    H24,
    /// 2:30 PM
    H12,
}

/// In-app shortcuts, spelled like "Ctrl+P".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    /// Pastes into the program the window opened over.
    pub paste: String,
    /// Pastes the text without its formatting.
    pub paste_plain: String,
    /// Only copies.
    pub copy_only: String,
    pub pin: String,
    pub delete: String,
    pub tag: String,
    pub view: String,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            paste: "Enter".into(),
            paste_plain: "Shift+Enter".into(),
            copy_only: "Ctrl+Enter".into(),
            pin: "Ctrl+P".into(),
            delete: "Shift+Delete".into(),
            tag: "Ctrl+T".into(),
            view: "Ctrl+L".into(),
        }
    }
}

/// How the list is ordered. Searching and browsing each remember their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    /// Closest to the query; only while searching.
    BestMatch,
    /// Last copied or used first.
    Recent,
    Oldest,
    AToZ,
    /// Copied back from the list most often.
    MostUsed,
}

impl SortOrder {
    pub const ALL: [SortOrder; 5] = [
        Self::BestMatch,
        Self::Recent,
        Self::Oldest,
        Self::AToZ,
        Self::MostUsed,
    ];

    /// Its name in the interface language.
    pub fn label(self) -> String {
        match self {
            Self::BestMatch => t!("sort-best-match"),
            Self::Recent => t!("sort-recent"),
            Self::Oldest => t!("sort-oldest"),
            Self::AToZ => t!("sort-a-to-z"),
            Self::MostUsed => t!("sort-most-used"),
        }
    }
}

/// How the main window lays out the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    /// Cards across the whole window; details open on demand.
    Grid,
    /// Rows across the whole window; details open on demand.
    List,
    /// Rows on the left, the selected item always shown on the right.
    Detail,
}

impl ViewMode {
    pub const ALL: [ViewMode; 3] = [Self::Grid, Self::List, Self::Detail];

    /// Its name in the interface language.
    pub fn label(self) -> String {
        match self {
            Self::Grid => t!("view-grid"),
            Self::List => t!("view-list"),
            Self::Detail => t!("view-detail"),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Grid => Self::List,
            Self::List => Self::Detail,
            Self::Detail => Self::Grid,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub monitor_clipboard: bool,
    pub max_items: usize,
    /// Items older than this many days are deleted; 0 is off.
    pub delete_after_days: u32,
    pub dedupe: bool,
    /// What reads the text in images.
    pub read_engine: ReadEngine,
    /// The server model that reads text; kept while another reader is
    /// chosen, so choosing the server again picks it.
    pub read_model: Option<ModelRef>,
    pub search_mode: SearchMode,
    pub theme: Theme,
    /// Opens the quick popup.
    pub popup_hotkey: String,
    /// Opens the full window.
    pub window_hotkey: String,
    pub ignore_next_hotkey: String,
    pub minimize_after_copy: bool,
    /// Don't record text that looks like a password, API key or token.
    pub skip_secrets: bool,
    /// Where meaning search runs.
    pub search_engine: SearchEngine,
    /// The model meaning search uses; kept while search is off, so turning
    /// it on again picks it.
    pub search_model: Option<ModelRef>,
    /// Lowest cosine similarity a semantic-only hit needs to be listed.
    pub min_similarity: f32,
    pub density: Density,
    pub group_by_date: bool,
    /// Index into the accent palette.
    pub accent: usize,
    pub view: ViewMode,
    /// Show a tray icon; closing or minimising the window hides it there.
    pub tray: bool,
    /// Launch (hidden) when the user signs in.
    pub start_at_login: bool,
    pub keys: Keys,
    pub language: Language,
    pub time_format: TimeFormat,
    /// Order while browsing (never `BestMatch`).
    pub sort: SortOrder,
    /// Order while searching.
    pub search_sort: SortOrder,
    /// Pinned items above the rest while browsing (not while searching).
    pub pinned_first: bool,
    /// Look for a new release on start and once a day.
    pub update_check: bool,
    /// Download a new release as soon as it's found (otherwise ask first).
    pub update_download: bool,
    /// The release "Skip this version" was used on (`version+build`).
    pub skipped_update: String,
    /// The saved servers, each with the models used on it.
    pub servers: Vec<Server>,
    /// Servers outside this PC that the user agreed to send the history to,
    /// so applying one of them asks only once.
    pub allowed_servers: Vec<String>,
    /// Copies that aren't recorded (see `rules`).
    pub rules: Vec<crate::rules::Rule>,
    /// The version that ran last: the first start of a newer one says what
    /// is new (D47).
    pub last_version: String,
    /// The getting-started checklist for new installs (D53).
    pub checklist: Checklist,
}

/// What the getting-started checklist keeps; its other steps are read from
/// the history and the settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Checklist {
    /// An item was pasted into another program.
    pub pasted: bool,
    /// Closed, done, or skipped because the history had items when this
    /// version first started: it never shows again.
    pub closed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_clipboard: true,
            max_items: 5000,
            delete_after_days: 0,
            dedupe: true,
            read_engine: ReadEngine::default(),
            read_model: None,
            search_mode: SearchMode::Hybrid,
            theme: Theme::System,
            popup_hotkey: "Alt+V".into(),
            window_hotkey: "Alt+Shift+V".into(),
            ignore_next_hotkey: "Alt+C".into(),
            minimize_after_copy: true,
            skip_secrets: true,
            search_engine: SearchEngine::Off,
            search_model: None,
            min_similarity: 0.0,
            density: Density::Comfortable,
            group_by_date: true,
            accent: 0,
            view: ViewMode::Grid,
            tray: false,
            start_at_login: true,
            keys: Keys::default(),
            language: Language::English,
            time_format: TimeFormat::H12,
            sort: SortOrder::Recent,
            search_sort: SortOrder::BestMatch,
            pinned_first: true,
            update_check: true,
            update_download: true,
            skipped_update: String::new(),
            servers: Vec::new(),
            allowed_servers: Vec::new(),
            rules: Vec::new(),
            last_version: String::new(),
            checklist: Checklist::default(),
        }
    }
}

impl Config {
    /// What "Reset all settings" leaves: every setting at its default, but
    /// the saved servers (with their keys) and the rules per app kept, as
    /// they are things the user made. Agreements to send the history to a
    /// server outside this PC go, so the app asks again. The version that
    /// ran last and the checklist aren't settings and stay.
    pub fn reset(&self) -> Config {
        Config {
            servers: self.servers.clone(),
            rules: self.rules.clone(),
            last_version: self.last_version.clone(),
            checklist: self.checklist.clone(),
            ..Config::default()
        }
    }

    /// Whether the user agreed to send the history to the server at `url`.
    pub fn is_server_allowed(&self, url: &str) -> bool {
        self.allowed_servers.contains(&address_key(url))
    }

    pub fn allow_server(&mut self, url: &str) {
        let key = address_key(url);
        if !self.allowed_servers.contains(&key) {
            self.allowed_servers.push(key);
        }
    }

    pub fn server(&self, id: u32) -> Option<&Server> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn model(&self, at: ModelRef) -> Option<(&Server, &Model)> {
        let server = self.server(at.server)?;
        Some((server, server.model(at.model)?))
    }

    /// The id a new server gets.
    pub fn next_server_id(&self) -> u32 {
        self.servers.iter().map(|s| s.id).max().unwrap_or(0) + 1
    }

    /// The settings of the search model chosen, to change them.
    pub fn search_settings_mut(&mut self) -> Option<&mut SearchSettings> {
        let at = self.search_model?;
        let server = self.servers.iter_mut().find(|s| s.id == at.server)?;
        let model = server.models.iter_mut().find(|m| m.id == at.model)?;
        match &mut model.job {
            ModelJob::Search(search) => Some(search),
            ModelJob::Read(_) => None,
        }
    }

    /// The meaning-search model in use, put together for the worker: none
    /// while search is off or its model is gone.
    pub fn search_backend(&self) -> EmbedBackend {
        if self.search_engine == SearchEngine::Off {
            return EmbedBackend::None;
        }
        self.search_model
            .and_then(|at| self.model(at))
            .and_then(|(server, model)| server.search_config(model))
            .map_or(EmbedBackend::None, |c| EmbedBackend::Server(Box::new(c)))
    }

    /// What reads the text in images, put together for the worker: off
    /// while its model is gone.
    pub fn text_reader(&self) -> TextReader {
        match self.read_engine {
            ReadEngine::Off => TextReader::Off,
            ReadEngine::Windows if cfg!(windows) => TextReader::Windows,
            ReadEngine::Windows => TextReader::Off,
            ReadEngine::Server => self
                .read_model
                .and_then(|at| self.model(at))
                .and_then(|(server, model)| server.read_config(model))
                .map_or(TextReader::Off, TextReader::Server),
        }
    }

    /// Whether copied images are read.
    pub fn reads_images(&self) -> bool {
        self.text_reader() != TextReader::Off
    }

    /// Reads the settings; a missing file means defaults. A file that can't be
    /// read is copied to `config.json.bad` before the defaults are used, so
    /// the next save doesn't silently destroy it, and the problem is returned
    /// for the app to show. Settings of an earlier version are updated, with
    /// what the user should hear about.
    pub fn load(path: &Path) -> (Self, Option<LoadProblem>, Option<Vec<UpdateNote>>) {
        let parsed = match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Self::default(), None, None)
            }
            Err(e) => Err(e.into()),
        };
        match parsed {
            Ok((cfg, notes)) => (cfg, None, notes),
            Err(e) => {
                let bad = path.with_extension("json.bad");
                let copy = std::fs::copy(path, &bad)
                    .map(|_| bad)
                    .map_err(|e| e.to_string());
                let problem = LoadProblem {
                    error: e.to_string(),
                    copy,
                };
                (Self::default(), Some(problem), None)
            }
        }
    }

    /// Settings as saved by this version or an earlier one; for an earlier
    /// one, also what its update changed that the user should hear about.
    fn parse(text: &str) -> anyhow::Result<(Self, Option<Vec<UpdateNote>>)> {
        let mut value: Value = serde_json::from_str(text)?;
        let notes = update_from_0_4(&mut value);
        Ok((serde_json::from_value(value)?, notes))
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

/// What the update of earlier settings changed that the user should know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateNote {
    /// Meaning search used the built-in model, which is gone: it is off.
    BuiltInSearchRemoved,
    /// The built-in text reader is gone: Windows reads the text in images,
    /// or reading is off where there is no Windows.
    BuiltInReaderRemoved,
}

/// Settings from 0.4 and earlier keep meaning search in `backend` and
/// `server_profiles`, and the text reader in `text_reader` and `ocr_images`.
/// Each address becomes a saved server, with the models used on it as they
/// were, so the vectors of the model in use stay valid. The built-in search
/// model and text reader are gone: search turns off, and Windows reads the
/// text (elsewhere reading turns off). None for settings that need no
/// update. This update ships in 0.5 and goes in the version after it
/// (migration rule D33).
fn update_from_0_4(value: &mut Value) -> Option<Vec<UpdateNote>> {
    let settings = value.as_object_mut()?;
    const OLD: [&str; 5] = [
        "backend",
        "server_profiles",
        "text_reader",
        "ocr_images",
        "builtin_repo",
    ];
    if !OLD.iter().any(|key| settings.contains_key(*key)) {
        return None;
    }
    let mut notes = Vec::new();
    let backend = settings.remove("backend").unwrap_or_default();
    let profiles = settings.remove("server_profiles").unwrap_or_default();
    let reader = settings.remove("text_reader");
    let reading = settings
        .remove("ocr_images")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    settings.remove("builtin_repo");

    // Meaning search: the server in use first, then the remembered ones.
    let mut saved = Saved::default();
    let mut search_model = None;
    if backend.get("SigLip").is_some() {
        notes.push(UpdateNote::BuiltInSearchRemoved);
    } else if let Some(server) = backend.get("Server") {
        search_model = saved.add_search_model(server);
    }
    for profile in profiles.as_array().into_iter().flatten() {
        saved.add_search_model(profile);
    }

    // The text reader. Settings without one read with the built-in reader
    // where Windows' reader isn't there.
    let built_in = match &reader {
        Some(name) => name == "BuiltIn",
        None => !cfg!(windows),
    };
    if built_in && reading {
        notes.push(UpdateNote::BuiltInReaderRemoved);
    }
    let read_model = reader
        .as_ref()
        .and_then(|r| r.get("Server"))
        .and_then(|server| saved.add_read_model(server));
    let read_engine = match (reading, read_model) {
        (false, _) => ReadEngine::Off,
        (true, Some(_)) => ReadEngine::Server,
        (true, None) => ReadEngine::default(),
    };

    let search_engine = match search_model {
        Some(_) => SearchEngine::Server,
        None => SearchEngine::Off,
    };
    settings.insert("servers".into(), Value::Array(saved.servers));
    settings.insert("search_engine".into(), json!(search_engine));
    settings.insert("search_model".into(), json!(search_model));
    settings.insert("read_engine".into(), json!(read_engine));
    settings.insert("read_model".into(), json!(read_model));
    Some(notes)
}

/// The servers the update makes from settings of 0.4 and earlier, as JSON.
#[derive(Default)]
struct Saved {
    servers: Vec<Value>,
}

impl Saved {
    /// A search server's settings as 0.4 saved them, as a model on its server.
    fn add_search_model(&mut self, old: &Value) -> Option<ModelRef> {
        let launch = old.get("launch").filter(|l| !l.is_null());
        let url = match launch {
            // A server the app starts listens on this PC, at its port.
            Some(l) => format!("http://127.0.0.1:{}/v1", l["port"].as_u64().unwrap_or(0)),
            None => old["base_url"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_string(),
        };
        if url.is_empty() {
            return None;
        }
        let at = self.server(&url, old["api_key"].as_str().unwrap_or_default(), launch);
        let mut settings = Map::new();
        for (from, to) in [
            ("query_prefix", "query_prefix"),
            ("document_prefix", "document_prefix"),
            ("images", "images"),
            ("truncate_dims", "dimensions"),
        ] {
            if let Some(v) = old.get(from) {
                settings.insert(to.into(), v.clone());
            }
        }
        let model = old["model"].as_str().unwrap_or_default().trim();
        Some(self.add_model(at, model, json!({ "Search": settings })))
    }

    /// A text reader server's settings as 0.4 saved them, as a model on its
    /// server.
    fn add_read_model(&mut self, old: &Value) -> Option<ModelRef> {
        let url = old["base_url"].as_str().unwrap_or_default().trim();
        if url.is_empty() {
            return None;
        }
        let at = self.server(url, old["api_key"].as_str().unwrap_or_default(), None);
        let mut settings = Map::new();
        for field in ["kind", "prompt"] {
            if let Some(v) = old.get(field) {
                settings.insert(field.into(), v.clone());
            }
        }
        let model = old["model"].as_str().unwrap_or_default().trim();
        Some(self.add_model(at, model, json!({ "Read": settings })))
    }

    /// The index of the server at `url` with the key `stored` (as the settings
    /// file keeps it), made if there is none yet. A server saved without a
    /// key takes this one.
    fn server(&mut self, url: &str, stored: &str, launch: Option<&Value>) -> usize {
        let key = crate::secure::open(stored);
        let found = self.servers.iter().position(|s| {
            let other = crate::secure::open(s["api_key"].as_str().unwrap_or_default());
            address_key(s["base_url"].as_str().unwrap_or_default()) == address_key(url)
                && (other == key || other.is_empty() || key.is_empty())
        });
        if let Some(at) = found {
            if self.servers[at]["api_key"] == "" && !stored.is_empty() {
                self.servers[at]["api_key"] = json!(stored);
            }
            return at;
        }
        let kind = match launch {
            Some(_) => ServerKind::LlamaCpp,
            None => ServerKind::from_address(url),
        };
        let name = match kind {
            // The port of the Sentence Transformers server in tools/.
            ServerKind::Other if port(crate::net::host(url)) == Some(8001) => {
                "Sentence Transformers".to_string()
            }
            ServerKind::Other => crate::net::host(url).to_string(),
            kind => kind.name().to_string(),
        };
        self.servers.push(json!({
            "id": self.servers.len() + 1,
            "kind": kind,
            "name": name,
            "base_url": url,
            "api_key": stored,
            "models": [],
            "launch": launch,
        }));
        self.servers.len() - 1
    }

    /// Adds a model to the server at `at`; the same model with the same
    /// settings is kept once.
    fn add_model(&mut self, at: usize, model: &str, job: Value) -> ModelRef {
        let server = &mut self.servers[at];
        let id = server["id"].as_u64().unwrap_or_default() as u32;
        let models = server["models"].as_array_mut().expect("models is a list");
        if let Some(same) = models
            .iter()
            .find(|m| m["model"] == model && m["job"] == job)
        {
            return ModelRef {
                server: id,
                model: same["id"].as_u64().unwrap_or_default() as u32,
            };
        }
        let model_id = models.len() as u32 + 1;
        models.push(json!({ "id": model_id, "model": model, "name": "", "job": job }));
        ModelRef {
            server: id,
            model: model_id,
        }
    }
}

/// Why the settings file could not be read. The app words it, because the
/// interface language is only known once the settings are read.
#[derive(Debug, PartialEq)]
pub struct LoadProblem {
    pub error: String,
    /// Where the unreadable file was copied, or why the copy failed.
    pub copy: Result<PathBuf, String>,
}

/// Where everything lives: `%APPDATA%\clipboard-manager\data` on Windows.
#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub images: PathBuf,
}

impl Paths {
    pub fn new() -> anyhow::Result<Self> {
        let root = match std::env::var_os("CLIPBOARD_MANAGER_HOME") {
            Some(p) => PathBuf::from(p),
            None => default_root().ok_or_else(|| anyhow::anyhow!("no home directory"))?,
        };
        Self::at(root)
    }

    pub fn at(root: PathBuf) -> anyhow::Result<Self> {
        let p = Self {
            config: root.join("config.json"),
            db: root.join("history.redb"),
            images: root.join("images"),
            root,
        };
        std::fs::create_dir_all(&p.images)?;
        Ok(p)
    }

    /// Deletes what the built-in search model and text reader downloaded in
    /// versions before 0.5: their folders in `models`, and `models` itself
    /// once it is empty. Anything else in it stays.
    pub fn remove_built_in_models(&self) {
        let models = self.root.join("models");
        let Ok(entries) = std::fs::read_dir(&models) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "ocrs" || name.starts_with("google--siglip") {
                if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                    log::warn!("removing the old model folder {name}: {e}");
                }
            }
        }
        // Fails while something else is in the folder, which is meant.
        let _ = std::fs::remove_dir(&models);
    }
}

/// The data folder when CLIPBOARD_MANAGER_HOME is not set.
fn default_root() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "clipboard-manager").map(|d| d.data_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window's size and place are kept in the data folder. Without
    /// CLIPBOARD_MANAGER_HOME that is the folder eframe used before, so an
    /// installed app keeps its window.
    #[test]
    fn the_default_data_folder_is_where_eframe_kept_the_window() {
        assert_eq!(default_root(), eframe::storage_dir("clipboard-manager"));
    }

    fn unsloth() -> Server {
        Server {
            id: 1,
            kind: ServerKind::Unsloth,
            name: "Unsloth".into(),
            base_url: "http://127.0.0.1:8888/v1".into(),
            api_key: "unsloth-key".into(),
            models: vec![
                Model {
                    id: 1,
                    model: "embed-a".into(),
                    name: "Embed A".into(),
                    job: ModelJob::Search(SearchSettings {
                        query_prefix: "q: ".into(),
                        dimensions: 256,
                        ..Default::default()
                    }),
                },
                Model {
                    id: 2,
                    model: "vision".into(),
                    job: ModelJob::Read(ReadSettings {
                        kind: ReaderModel::MiniCpmOcr,
                        prompt: "Text Recognition:".into(),
                    }),
                    ..Default::default()
                },
            ],
            launch: None,
        }
    }

    fn with_unsloth() -> Config {
        Config {
            servers: vec![unsloth()],
            search_engine: SearchEngine::Server,
            search_model: Some(ModelRef {
                server: 1,
                model: 1,
            }),
            read_engine: ReadEngine::Server,
            read_model: Some(ModelRef {
                server: 1,
                model: 2,
            }),
            ..Config::default()
        }
    }

    #[test]
    fn the_models_in_use_are_put_together_with_their_server() {
        let cfg = with_unsloth();
        assert_eq!(
            cfg.search_backend(),
            EmbedBackend::Server(Box::new(ServerConfig {
                server_kind: ServerKind::Unsloth,
                api_key: "unsloth-key".into(),
                model: "embed-a".into(),
                query_prefix: "q: ".into(),
                truncate_dims: 256,
                ..ServerConfig::plain("http://127.0.0.1:8888/v1")
            }))
        );
        assert_eq!(
            cfg.text_reader(),
            TextReader::Server(ReaderServer {
                server_kind: ServerKind::Unsloth,
                base_url: "http://127.0.0.1:8888/v1".into(),
                api_key: "unsloth-key".into(),
                model: "vision".into(),
                kind: ReaderModel::MiniCpmOcr,
                prompt: "Text Recognition:".into(),
            })
        );
        assert!(cfg.reads_images());
    }

    #[test]
    fn search_is_off_without_its_model() {
        let mut cfg = with_unsloth();
        cfg.search_engine = SearchEngine::Off;
        assert_eq!(cfg.search_backend(), EmbedBackend::None);
        assert!(cfg.search_model.is_some(), "the choice is kept for later");

        let mut cfg = with_unsloth();
        cfg.servers[0].models.remove(0);
        assert_eq!(cfg.search_backend(), EmbedBackend::None, "a model gone");

        // A reading model can't search, and a search model can't read.
        let mut cfg = with_unsloth();
        cfg.search_model = cfg.read_model;
        cfg.read_model = Some(ModelRef {
            server: 1,
            model: 1,
        });
        assert_eq!(cfg.search_backend(), EmbedBackend::None);
        assert_eq!(cfg.text_reader(), TextReader::Off);

        let mut cfg = with_unsloth();
        cfg.read_engine = ReadEngine::Windows;
        let windows = if cfg!(windows) {
            TextReader::Windows
        } else {
            TextReader::Off
        };
        assert_eq!(cfg.text_reader(), windows);
    }

    #[test]
    fn a_reset_keeps_servers_and_rules() {
        let mut cfg = with_unsloth();
        cfg.rules = vec![crate::rules::Rule {
            app: "keepass.exe".into(),
            ..Default::default()
        }];
        cfg.allow_server("http://192.0.2.1:8080/v1");
        cfg.max_items = 100;
        cfg.last_version = "0.6.0".into();
        cfg.checklist.closed = true;
        let reset = cfg.reset();
        assert_eq!(reset.servers, cfg.servers);
        assert_eq!(reset.rules, cfg.rules);
        // Not settings: a reset neither says what is new again nor brings
        // back a closed checklist.
        assert_eq!(reset.last_version, "0.6.0");
        assert!(reset.checklist.closed);
        assert!(!reset.is_server_allowed("http://192.0.2.1:8080/v1"));
        assert_eq!(reset.max_items, Config::default().max_items);
        assert_eq!(reset.search_engine, SearchEngine::Off);
        // Reset again, nothing changes.
        assert!(reset.reset() == reset);
    }

    #[test]
    fn servers_survive_a_save() {
        let cfg = with_unsloth();
        let json = serde_json::to_string(&cfg).unwrap();
        let (back, notes) = Config::parse(&json).unwrap();
        assert_eq!(back, cfg);
        assert_eq!(notes, None, "settings of this version need no update");
        if cfg!(windows) {
            assert!(!json.contains("unsloth-key"), "the key is encrypted");
        }
        // Fields a later version adds are missing from older files.
        let (cfg, _) = Config::parse(r#"{"servers":[{"id":4,"base_url":"http://x/v1"}]}"#).unwrap();
        assert_eq!(cfg.servers[0].kind, ServerKind::Other);
        assert!(cfg.servers[0].models.is_empty());
    }

    /// Settings as 0.4.3 saved them, with the parts this update reads.
    const SETTINGS_0_4: &str = r#"{
        "max_items": 100,
        "ocr_images": true,
        "text_reader": {"Server": {"base_url": "http://127.0.0.1:8080/v1", "api_key": "",
            "model": "minicpm-ocr", "kind": "MiniCpmOcr", "prompt": "Text Recognition:"}},
        "backend": {"Server": {"base_url": "http://127.0.0.1:8888/v1", "api_key": "unsloth-key",
            "model": "", "query_prefix": "task: search result | query: ",
            "document_prefix": "title: none | text: ", "images": false, "truncate_dims": 0,
            "launch": null}},
        "server_profiles": [
            {"base_url": "http://127.0.0.1:8888/v1", "api_key": "unsloth-key", "model": "",
             "query_prefix": "task: search result | query: ",
             "document_prefix": "title: none | text: ", "images": false, "truncate_dims": 0,
             "launch": null},
            {"base_url": "http://127.0.0.1:1234/v1", "api_key": "", "model": "nomic-embed",
             "query_prefix": "", "document_prefix": "", "images": false, "truncate_dims": 512,
             "launch": null},
            {"base_url": "http://127.0.0.1:8001/v1", "api_key": "", "model": "",
             "query_prefix": "", "document_prefix": "", "images": true, "truncate_dims": 0,
             "launch": null}
        ],
        "builtin_repo": "google/siglip-base-patch16-224",
        "allowed_servers": []
    }"#;

    #[test]
    fn settings_from_0_4_become_saved_servers_with_their_models() {
        let (cfg, notes) = Config::parse(SETTINGS_0_4).unwrap();
        assert_eq!(notes, Some(Vec::new()), "nothing to tell");
        assert_eq!(cfg.max_items, 100, "the other settings stay");
        let names: Vec<(ServerKind, &str)> = cfg
            .servers
            .iter()
            .map(|s| (s.kind, s.name.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                (ServerKind::Unsloth, "Unsloth"),
                (ServerKind::LmStudio, "LM Studio"),
                (ServerKind::Other, "Sentence Transformers"),
                (ServerKind::LlamaCpp, "llama.cpp"),
            ]
        );
        assert_eq!(
            cfg.servers[0].models.len(),
            1,
            "the server in use and its remembered settings are one model"
        );
        // The model in use comes out as it was, so its vectors still count.
        assert_eq!(
            cfg.search_backend(),
            EmbedBackend::Server(Box::new(ServerConfig {
                server_kind: ServerKind::Unsloth,
                api_key: "unsloth-key".into(),
                query_prefix: SearchSettings::GEMMA_QUERY.into(),
                document_prefix: SearchSettings::GEMMA_DOCUMENT.into(),
                ..ServerConfig::plain("http://127.0.0.1:8888/v1")
            }))
        );
        let lm_studio = &cfg.servers[1].models[0];
        assert_eq!(lm_studio.model, "nomic-embed");
        assert!(matches!(&lm_studio.job, ModelJob::Search(s) if s.dimensions == 512));
        assert!(matches!(&cfg.servers[2].models[0].job, ModelJob::Search(s) if s.images));
        assert_eq!(
            cfg.text_reader(),
            TextReader::Server(ReaderServer {
                server_kind: ServerKind::LlamaCpp,
                base_url: "http://127.0.0.1:8080/v1".into(),
                api_key: String::new(),
                model: "minicpm-ocr".into(),
                kind: ReaderModel::MiniCpmOcr,
                prompt: "Text Recognition:".into(),
            })
        );
        // Saved again, it is settings of this version.
        let saved = serde_json::to_string(&cfg).unwrap();
        assert!(!saved.contains("server_profiles") && !saved.contains("builtin_repo"));
        assert_eq!(Config::parse(&saved).unwrap(), (cfg, None));
    }

    #[test]
    fn a_reader_on_the_search_server_is_one_more_model_there() {
        let old = r#"{
            "backend": {"Server": {"base_url": "http://127.0.0.1:8000/v1", "model": "embed"}},
            "text_reader": {"Server": {"base_url": "http://127.0.0.1:8000/v1/", "model": "ocr"}}
        }"#;
        let (cfg, _) = Config::parse(old).unwrap();
        assert_eq!(cfg.servers.len(), 1);
        assert_eq!(cfg.servers[0].kind, ServerKind::Vllm);
        assert_eq!(cfg.servers[0].models.len(), 2);
        // A reader saved before model types gets the usual prompt.
        assert!(matches!(&cfg.text_reader(), TextReader::Server(r)
            if r.model == "ocr" && r.kind == ReaderModel::Other && r.prompt == ReaderServer::PROMPT));
    }

    #[test]
    fn the_built_in_engines_become_off_and_windows() {
        let old = r#"{"backend":{"SigLip":{"repo":"google/siglip-base-patch16-224"}},
            "builtin_repo":"google/siglip-base-patch16-224","text_reader":"BuiltIn",
            "ocr_images":true}"#;
        let (cfg, notes) = Config::parse(old).unwrap();
        assert_eq!(cfg.search_engine, SearchEngine::Off);
        assert_eq!(cfg.read_engine, ReadEngine::default());
        assert_eq!(
            notes,
            Some(vec![
                UpdateNote::BuiltInSearchRemoved,
                UpdateNote::BuiltInReaderRemoved
            ])
        );
        // Reading was off: it stays off, and there is nothing to say.
        let (cfg, notes) =
            Config::parse(r#"{"text_reader":"BuiltIn","ocr_images":false}"#).unwrap();
        assert_eq!(
            (cfg.read_engine, notes),
            (ReadEngine::Off, Some(Vec::new()))
        );
        let (cfg, _) = Config::parse(r#"{"text_reader":"Windows","ocr_images":true}"#).unwrap();
        assert_eq!(cfg.read_engine, ReadEngine::default());
    }

    #[test]
    fn kinds_come_from_the_usual_addresses() {
        for (url, kind) in [
            ("http://127.0.0.1:8080/v1", ServerKind::LlamaCpp),
            ("http://localhost:11434/v1", ServerKind::Ollama),
            ("http://127.0.0.1:1234/v1", ServerKind::LmStudio),
            ("http://[::1]:8000/v1", ServerKind::Vllm),
            ("http://127.0.0.1:8888/v1", ServerKind::Unsloth),
            ("https://openrouter.ai/api/v1", ServerKind::OpenRouter),
            ("http://127.0.0.1:3333/v1", ServerKind::Other),
            ("http://192.168.1.5/v1", ServerKind::Other),
        ] {
            assert_eq!(ServerKind::from_address(url), kind, "{url}");
        }
        for kind in ServerKind::ALL {
            let usual = kind.address();
            assert!(usual.is_empty() || ServerKind::from_address(usual) == kind);
        }
    }

    #[test]
    fn a_damaged_settings_file_is_kept_and_reported() {
        let dir = std::env::temp_dir().join(format!("cbm-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let (cfg, problem, _) = Config::load(&path);
        assert_eq!(cfg, Config::default());
        assert!(problem.is_none(), "a missing file is not a problem");

        std::fs::write(&path, r#"{"max_items": "not a number"}"#).unwrap();
        let (cfg, problem, _) = Config::load(&path);
        assert_eq!(cfg, Config::default());
        let problem = problem.expect("a damaged file is a problem");
        assert_eq!(problem.copy, Ok(dir.join("config.json.bad")));
        let kept = std::fs::read_to_string(dir.join("config.json.bad")).unwrap();
        assert_eq!(kept, r#"{"max_items": "not a number"}"#);

        std::fs::write(&path, r#"{"max_items": 7}"#).unwrap();
        let (cfg, problem, _) = Config::load(&path);
        assert_eq!((cfg.max_items, problem), (7, None));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn debug_output_hides_the_api_key() {
        let cfg = with_unsloth();
        assert!(!format!("{:?}", cfg.servers[0]).contains("unsloth-key"));
        assert!(!format!("{:?}", cfg.search_backend()).contains("unsloth-key"));
        assert!(!format!("{:?}", cfg.text_reader()).contains("unsloth-key"));
    }

    #[test]
    fn an_agreed_server_is_remembered_by_its_address() {
        let mut cfg = Config::default();
        assert!(!cfg.is_server_allowed("https://api.example.com/v1"));
        cfg.allow_server("https://API.example.com/v1/");
        assert!(cfg.is_server_allowed("https://api.example.com/v1"));
        assert!(!cfg.is_server_allowed("https://other.example.com/v1"));
        cfg.allow_server("https://api.example.com/v1");
        assert_eq!(cfg.allowed_servers.len(), 1);
    }
}
