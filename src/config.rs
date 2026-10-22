//! User settings, persisted as JSON in the data directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    Server(ServerConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub base_url: String,
    /// Sent as `Authorization: Bearer <key>` when not empty.
    #[serde(default)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub llama_server: PathBuf,
    pub gguf: PathBuf,
    /// Multimodal projector, needed for image embeddings with some GGUF models.
    pub mmproj: Option<PathBuf>,
    pub port: u16,
    pub extra_args: String,
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
    pub copy: String,
    pub pin: String,
    pub delete: String,
    pub tag: String,
    pub view: String,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            copy: "Enter".into(),
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

    pub fn label(self) -> &'static str {
        match self {
            Self::BestMatch => "Best match",
            Self::Recent => "Recent",
            Self::Oldest => "Oldest",
            Self::AToZ => "A to Z",
            Self::MostUsed => "Most used",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::List => "List",
            Self::Detail => "Detail",
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
    pub dedupe: bool,
    pub ocr_images: bool,
    pub search_mode: SearchMode,
    pub theme: Theme,
    pub show_hotkey: String,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_clipboard: true,
            max_items: 5000,
            dedupe: true,
            ocr_images: true,
            search_mode: SearchMode::Hybrid,
            theme: Theme::System,
            show_hotkey: "Alt+V".into(),
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
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
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
            None => directories::ProjectDirs::from("", "", "clipboard-manager")
                .ok_or_else(|| anyhow::anyhow!("no home directory"))?
                .data_dir()
                .to_path_buf(),
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
