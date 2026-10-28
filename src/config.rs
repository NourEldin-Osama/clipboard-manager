//! User settings, persisted as JSON in the data directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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

/// Which embedding backend is active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmbedBackend {
    /// No semantic search: keyword + OCR only.
    None,
    /// Built-in pure-Rust SigLIP (text <-> image), run in-process with candle.
    SigLip { repo: String },
    /// Any OpenAI-compatible `/v1/embeddings` server on this machine
    /// (llama.cpp `llama-server --embeddings`, Ollama, LM Studio, vLLM, ...).
    Server(Box<ServerConfig>),
}

/// Fields missing from an older settings file take the defaults, so adding a
/// field never makes saved server settings unreadable.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty. Encrypted in the
    /// settings file (see secure.rs).
    #[serde(default, with = "crate::secure::string")]
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

/// What reads the text in images.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TextReader {
    /// Windows' own reader, in the languages installed in Windows.
    Windows,
    /// The reader built into the app: Latin letters, after a download.
    BuiltIn,
    /// A vision model on a server, asked with a prompt.
    Server(ReaderServer),
}

impl TextReader {
    /// New settings use Windows' reader where there is one.
    fn for_new_settings() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::BuiltIn
        }
    }

    /// Settings from before the choice used the built-in reader, and keep it.
    fn for_older_settings() -> Self {
        Self::BuiltIn
    }
}

/// A server that reads text in images: any OpenAI-style chat server with a
/// vision model.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderServer {
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty. Encrypted in the
    /// settings file (see secure.rs).
    #[serde(default, with = "crate::secure::string")]
    pub api_key: String,
    /// Empty for the server's default model.
    pub model: String,
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
            base_url: "http://127.0.0.1:8080/v1".into(),
            api_key: String::new(),
            model: String::new(),
            prompt: Self::PROMPT.into(),
        }
    }
}

/// Written by hand so the API key never reaches a log.
impl std::fmt::Debug for ReaderServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = if self.api_key.is_empty() {
            ""
        } else {
            "<hidden>"
        };
        f.debug_struct("ReaderServer")
            .field("base_url", &self.base_url)
            .field("api_key", &key)
            .field("model", &self.model)
            .field("prompt", &self.prompt)
            .finish()
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
        let key = if self.api_key.is_empty() {
            ""
        } else {
            "<hidden>"
        };
        f.debug_struct("ServerConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &key)
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

    pub const GEMMA_QUERY: &'static str = "task: search result | query: ";
    pub const GEMMA_DOCUMENT: &'static str = "title: none | text: ";

    /// Defaults matching EmbeddingGemma's retrieval prompts.
    pub fn embeddinggemma() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080/v1".into(),
            api_key: String::new(),
            model: String::new(),
            query_prefix: Self::GEMMA_QUERY.into(),
            document_prefix: Self::GEMMA_DOCUMENT.into(),
            images: false,
            truncate_dims: 0,
            launch: None,
        }
    }
}

/// The three choices for meaning search in Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Off,
    BuiltIn,
    Server,
}

impl EmbedBackend {
    pub fn kind(&self) -> BackendKind {
        match self {
            Self::None => BackendKind::Off,
            Self::SigLip { .. } => BackendKind::BuiltIn,
            Self::Server(_) => BackendKind::Server,
        }
    }
}

/// The last settings used with each embedding server address, newest first,
/// so going back to a server restores its key, model and prompts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerProfiles(Vec<ServerConfig>);

impl ServerProfiles {
    /// More addresses than anyone switches between; keeps the file small.
    const MAX: usize = 20;

    /// Saves `server` as the newest settings for its address.
    pub fn remember(&mut self, server: &ServerConfig) {
        if server.base_url.trim().is_empty() {
            return;
        }
        let key = address_key(&server.base_url);
        self.0.retain(|p| address_key(&p.base_url) != key);
        self.0.insert(0, server.clone());
        self.0.truncate(Self::MAX);
    }

    pub fn get(&self, url: &str) -> Option<&ServerConfig> {
        let key = address_key(url);
        self.0.iter().find(|p| address_key(&p.base_url) == key)
    }

    pub fn latest(&self) -> Option<&ServerConfig> {
        self.0.first()
    }

    /// Moves `current` to the server at `url`: its settings are remembered,
    /// then the saved settings for `url` are loaded, or empty ones. A key is
    /// never carried over to a different server.
    pub fn switch_to(&mut self, current: &mut ServerConfig, url: &str) {
        if address_key(&current.base_url) == address_key(url) {
            return;
        }
        self.remember(current);
        *current = self
            .get(url)
            .cloned()
            .unwrap_or_else(|| ServerConfig::plain(url));
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
    /// Pastes into the program the window opened over. Settings from before
    /// pasting called it `copy`.
    #[serde(alias = "copy")]
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
    pub ocr_images: bool,
    /// What reads the text in images.
    #[serde(default = "TextReader::for_older_settings")]
    pub text_reader: TextReader,
    pub search_mode: SearchMode,
    pub theme: Theme,
    /// Opens the quick popup. Settings from before the popup called it
    /// `show_hotkey`, the key that opened the window.
    #[serde(alias = "show_hotkey")]
    pub popup_hotkey: String,
    /// Opens the full window.
    pub window_hotkey: String,
    pub ignore_next_hotkey: String,
    pub minimize_after_copy: bool,
    /// Don't record text that looks like a password, API key or token.
    pub skip_secrets: bool,
    pub backend: EmbedBackend,
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
    /// Each embedding server's last settings, so leaving a server and coming
    /// back keeps its key, model and prompts.
    pub server_profiles: ServerProfiles,
    /// The built-in model last used, picked again when "Built-in" is chosen.
    pub builtin_repo: Option<String>,
    /// Servers outside this PC that the user agreed to send the history to,
    /// so applying one of them asks only once.
    pub allowed_servers: Vec<String>,
    /// Copies that aren't recorded (see `rules`).
    pub rules: Vec<crate::rules::Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_clipboard: true,
            max_items: 5000,
            delete_after_days: 0,
            dedupe: true,
            ocr_images: true,
            text_reader: TextReader::for_new_settings(),
            search_mode: SearchMode::Hybrid,
            theme: Theme::System,
            popup_hotkey: "Alt+V".into(),
            window_hotkey: "Alt+Shift+V".into(),
            ignore_next_hotkey: "Alt+C".into(),
            minimize_after_copy: true,
            skip_secrets: true,
            backend: EmbedBackend::None,
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
            server_profiles: ServerProfiles::default(),
            builtin_repo: None,
            allowed_servers: Vec::new(),
            rules: Vec::new(),
        }
    }
}

impl Config {
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

    /// Switches meaning search to `kind`. The settings of the backend being
    /// left are kept, so choosing it again restores them.
    pub fn choose_backend(&mut self, kind: BackendKind) {
        if self.backend.kind() == kind {
            return;
        }
        match &self.backend {
            EmbedBackend::Server(s) => self.server_profiles.remember(s),
            EmbedBackend::SigLip { repo } => self.builtin_repo = Some(repo.clone()),
            EmbedBackend::None => {}
        }
        self.backend = match kind {
            BackendKind::Off => EmbedBackend::None,
            BackendKind::BuiltIn => EmbedBackend::SigLip {
                repo: self
                    .builtin_repo
                    .clone()
                    .unwrap_or_else(|| crate::models::SIGLIP_MODELS[0].repo.into()),
            },
            BackendKind::Server => EmbedBackend::Server(Box::new(
                self.server_profiles
                    .latest()
                    .cloned()
                    .unwrap_or_else(ServerConfig::embeddinggemma),
            )),
        };
    }

    /// Reads the settings; a missing file means defaults. A file that can't be
    /// read is copied to `config.json.bad` before the defaults are used, so
    /// the next save doesn't silently destroy it, and the problem is returned
    /// for the app to show.
    pub fn load(path: &Path) -> (Self, Option<LoadProblem>) {
        let parsed = match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(anyhow::Error::from),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Self::default(), None),
            Err(e) => Err(e.into()),
        };
        match parsed {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                let bad = path.with_extension("json.bad");
                let copy = std::fs::copy(path, &bad)
                    .map(|_| bad)
                    .map_err(|e| e.to_string());
                let problem = LoadProblem {
                    error: e.to_string(),
                    copy,
                };
                (Self::default(), Some(problem))
            }
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
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
    pub models: PathBuf,
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
            models: root.join("models"),
            root,
        };
        std::fs::create_dir_all(&p.images)?;
        std::fs::create_dir_all(&p.models)?;
        Ok(p)
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

    const UNSLOTH: &str = "http://127.0.0.1:8888/v1";
    const LLAMA: &str = "http://127.0.0.1:8080/v1";

    fn unsloth() -> ServerConfig {
        ServerConfig {
            api_key: "unsloth-key".into(),
            model: "embed-a".into(),
            ..ServerConfig::plain(UNSLOTH)
        }
    }

    #[test]
    fn quick_setup_switch_keeps_each_servers_settings() {
        let mut profiles = ServerProfiles::default();
        let mut current = unsloth();
        profiles.switch_to(&mut current, LLAMA);
        assert_eq!(current.base_url, LLAMA);
        assert!(
            current.api_key.is_empty(),
            "a key is never carried to another server"
        );
        profiles.switch_to(&mut current, &format!("{UNSLOTH}/"));
        assert_eq!(current, unsloth());
    }

    #[test]
    fn leaving_the_server_backend_and_coming_back_restores_it() {
        let mut cfg = Config {
            backend: EmbedBackend::Server(Box::new(unsloth())),
            ..Config::default()
        };
        cfg.choose_backend(BackendKind::BuiltIn);
        cfg.choose_backend(BackendKind::Off);
        cfg.choose_backend(BackendKind::Server);
        assert_eq!(cfg.backend, EmbedBackend::Server(Box::new(unsloth())));
    }

    #[test]
    fn the_built_in_model_choice_is_restored_too() {
        let mut cfg = Config {
            backend: EmbedBackend::SigLip {
                repo: "some/other-model".into(),
            },
            ..Config::default()
        };
        cfg.choose_backend(BackendKind::Off);
        cfg.choose_backend(BackendKind::BuiltIn);
        assert!(
            matches!(&cfg.backend, EmbedBackend::SigLip { repo } if repo == "some/other-model")
        );
    }

    #[test]
    fn profiles_survive_a_save_and_old_files_still_load() {
        let mut cfg = Config::default();
        cfg.server_profiles.remember(&unsloth());
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_profiles.get(UNSLOTH), Some(&unsloth()));

        let old = r#"{"max_items":100,"backend":"None"}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert_eq!(cfg.max_items, 100);
        assert!(cfg.server_profiles.latest().is_none());
    }

    #[test]
    fn older_settings_keep_the_built_in_reader() {
        let old: Config = serde_json::from_str(r#"{"ocr_images":true}"#).unwrap();
        assert_eq!(old.text_reader, TextReader::BuiltIn);
        assert_eq!(
            Config::default().text_reader,
            TextReader::for_new_settings()
        );
        let cfg = Config {
            text_reader: TextReader::Server(ReaderServer {
                api_key: "reader-key".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text_reader, cfg.text_reader);
        if cfg!(windows) {
            assert!(!json.contains("reader-key"), "the key is encrypted");
        }
    }

    #[test]
    fn the_old_window_shortcut_opens_the_popup() {
        let old = r#"{"show_hotkey":"Ctrl+Shift+Space"}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert_eq!(cfg.popup_hotkey, "Ctrl+Shift+Space");
        assert_eq!(cfg.window_hotkey, Config::default().window_hotkey);
    }

    #[test]
    fn the_old_copy_key_becomes_the_paste_key() {
        let old = r#"{"keys":{"copy":"Ctrl+Space","pin":"Ctrl+P"}}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert_eq!(cfg.keys.paste, "Ctrl+Space");
        assert_eq!(cfg.keys.copy_only, Keys::default().copy_only);
        let back: Config = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(back.keys, cfg.keys);
    }

    #[cfg(windows)]
    #[test]
    fn api_keys_are_encrypted_in_the_settings_file() {
        let mut cfg = Config {
            backend: EmbedBackend::Server(Box::new(unsloth())),
            ..Config::default()
        };
        cfg.server_profiles.remember(&unsloth());
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("unsloth-key"));
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn server_settings_missing_newer_fields_still_load() {
        let json = r#"{"base_url":"http://x/v1","model":"m"}"#;
        let s: ServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            (s.base_url.as_str(), s.model.as_str()),
            ("http://x/v1", "m")
        );
        assert_eq!(s.launch, None);
    }

    #[test]
    fn a_plain_key_from_an_older_version_still_loads() {
        let json = r#"{"base_url":"http://x/v1","api_key":"old-key","model":"","query_prefix":"","document_prefix":"","images":false,"truncate_dims":0,"launch":null}"#;
        let s: ServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(s.api_key, "old-key");
    }

    #[test]
    fn a_damaged_settings_file_is_kept_and_reported() {
        let dir = std::env::temp_dir().join(format!("cbm-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let (cfg, problem) = Config::load(&path);
        assert_eq!(cfg, Config::default());
        assert!(problem.is_none(), "a missing file is not a problem");

        std::fs::write(&path, r#"{"max_items": "not a number"}"#).unwrap();
        let (cfg, problem) = Config::load(&path);
        assert_eq!(cfg, Config::default());
        let problem = problem.expect("a damaged file is a problem");
        assert_eq!(problem.copy, Ok(dir.join("config.json.bad")));
        let kept = std::fs::read_to_string(dir.join("config.json.bad")).unwrap();
        assert_eq!(kept, r#"{"max_items": "not a number"}"#);

        std::fs::write(&path, r#"{"max_items": 7}"#).unwrap();
        let (cfg, problem) = Config::load(&path);
        assert_eq!((cfg.max_items, problem), (7, None));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn debug_output_hides_the_api_key() {
        assert!(!format!("{:?}", unsloth()).contains("unsloth-key"));
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
