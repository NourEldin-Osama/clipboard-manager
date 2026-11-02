//! What is known about a model, for "Fill in details" in Settings: its name
//! and limits from models.dev, a public list of AI models, and the prompts
//! that popular embedding models are trained with. The list downloads only
//! when asked, and nothing from the history goes with the request.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::{ReaderModel, SearchSettings};

const URL: &str = "https://models.dev/api.json";

/// The list is several megabytes; this leaves room for it to grow.
const MAX_SIZE: u64 = 64 << 20;

#[derive(Deserialize)]
struct Provider {
    #[serde(default)]
    models: HashMap<String, Entry>,
}

/// What models.dev says about a model.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub modalities: Modalities,
    pub limit: Limit,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Modalities {
    pub input: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Limit {
    /// The most tokens the model takes in.
    pub context: u64,
}

impl Entry {
    pub fn takes_images(&self) -> bool {
        self.modalities.input.iter().any(|m| m == "image")
    }
}

type List = HashMap<String, Provider>;

/// The list, kept after the first download for the rest of the run.
static LIST: Mutex<Option<Arc<List>>> = Mutex::new(None);

fn list() -> Result<Arc<List>> {
    if let Some(list) = LIST.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Ok(list);
    }
    let mut response = crate::net::download(Duration::from_secs(120))
        .get(URL)
        .call()
        .context("cannot reach models.dev")?;
    let text = response
        .body_mut()
        .with_config()
        .limit(MAX_SIZE)
        .read_to_string()?;
    let list: List =
        serde_json::from_str(&text).context("models.dev sent a list this app can't read")?;
    let list = Arc::new(list);
    *LIST.lock().unwrap_or_else(|e| e.into_inner()) = Some(list.clone());
    Ok(list)
}

/// The model a server calls `id`, looked up among the models of `provider`
/// first: models.dev's name for the server, such as `openrouter`.
pub fn look_up(id: &str, provider: Option<&str>) -> Result<Option<Entry>> {
    let list = list()?;
    Ok(find(&list, id, provider))
}

fn find(list: &List, id: &str, provider: Option<&str>) -> Option<Entry> {
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    let same = |key: &str, entry: &Entry| {
        key.eq_ignore_ascii_case(id) || entry.id.eq_ignore_ascii_case(id)
    };
    if let Some(provider) = provider.and_then(|p| list.get(p)) {
        if let Some((_, entry)) = provider.models.iter().find(|(key, e)| same(key, e)) {
            return Some(entry.clone());
        }
    }
    // Every provider's models, in the same order each time.
    let mut models: Vec<(&String, &String, &Entry)> = list
        .iter()
        .flat_map(|(name, p)| p.models.iter().map(move |(key, e)| (name, key, e)))
        .collect();
    models.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    if let Some((_, _, entry)) = models.iter().find(|(_, key, e)| same(key, e)) {
        return Some((*entry).clone());
    }
    // The same model under another maker's name, tag, file name or quantization.
    let plain = plain_name(id);
    models
        .iter()
        .find(|(_, key, e)| plain_name(key) == plain || plain_name(&e.id) == plain)
        .map(|(_, _, entry)| (*entry).clone())
}

/// A model's name without what differs from server to server: who made it,
/// a tag such as `:free` or `:latest`, the file ending and the quantization.
/// "unsloth/embeddinggemma-300m-GGUF:Q8_0" is "embeddinggemma-300m".
pub fn plain_name(id: &str) -> String {
    let id = id.trim().to_ascii_lowercase();
    let id = id.rsplit('/').next().unwrap_or_default();
    let id = id.split(':').next().unwrap_or_default();
    let id = id.trim_end_matches(".gguf").trim_end_matches("-gguf");
    // LM Studio's names for embedding models start with this.
    let id = match id.strip_prefix("text-embedding-") {
        Some(rest) if rest.contains("embed") => rest,
        _ => id,
    };
    let mut parts: Vec<&str> = id.split('-').collect();
    while parts.len() > 1 && parts.last().is_some_and(|part| is_quantization(part)) {
        parts.pop();
    }
    parts.join("-")
}

/// A part of a file name that names how the weights are stored, as `q8_0`,
/// `iq4_xs` or `f16`.
fn is_quantization(part: &str) -> bool {
    let digit_after = |prefix: &str| {
        part.strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    matches!(
        part,
        "f16" | "bf16" | "f32" | "fp16" | "fp8" | "int4" | "int8" | "awq" | "gptq" | "mlx"
    ) || digit_after("q")
        || digit_after("iq")
}

/// The prompts a popular embedding model is trained with, from its model
/// card: what goes before each search, and before each item.
pub fn known_prompts(id: &str) -> Option<(&'static str, &'static str)> {
    let name = plain_name(id);
    if name.contains("embeddinggemma") {
        Some((SearchSettings::GEMMA_QUERY, SearchSettings::GEMMA_DOCUMENT))
    } else if name.contains("nomic-embed-text") {
        Some(("search_query: ", "search_document: "))
    } else if name.contains("multilingual-e5") && !name.contains("instruct") {
        Some(("query: ", "passage: "))
    } else if (name.starts_with("bge-") && name.ends_with("-en-v1.5"))
        || name.contains("mxbai-embed-large")
    {
        Some((
            "Represent this sentence for searching relevant passages: ",
            "",
        ))
    } else {
        None
    }
}

/// The reading model the app has settings for that `id` names, if any.
pub fn reader_kind(id: &str) -> Option<ReaderModel> {
    let name = plain_name(id);
    ReaderModel::ALL
        .into_iter()
        .filter(|kind| *kind != ReaderModel::Other)
        .find(|kind| name.contains(&plain_name(kind.served_name())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, name: &str) -> Entry {
        Entry {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    fn list() -> List {
        let provider = |models: &[Entry]| Provider {
            models: models.iter().map(|e| (e.id.clone(), e.clone())).collect(),
        };
        HashMap::from([
            (
                "openrouter".to_string(),
                provider(&[entry("qwen/qwen3-embedding-8b", "Qwen: Qwen3 Embedding 8B")]),
            ),
            (
                "scaleway".to_string(),
                provider(&[entry("qwen3-embedding-8b", "Qwen3 Embedding 8B")]),
            ),
        ])
    }

    #[test]
    fn a_model_is_found_by_its_name_on_the_server() {
        let list = list();
        let name = |id: &str, provider| find(&list, id, provider).map(|e| e.name);
        // The server's own entry first.
        assert_eq!(
            name("qwen/qwen3-embedding-8b", Some("openrouter")).as_deref(),
            Some("Qwen: Qwen3 Embedding 8B")
        );
        // Another server's name for the same model.
        assert_eq!(
            name("Qwen/Qwen3-Embedding-8B-GGUF:Q8_0", None).as_deref(),
            Some("Qwen: Qwen3 Embedding 8B")
        );
        assert_eq!(
            name("qwen3-embedding-8b", None).as_deref(),
            Some("Qwen3 Embedding 8B")
        );
        assert_eq!(name("some-other-model", None), None);
        assert_eq!(name("", None), None);
    }

    #[test]
    fn plain_names_leave_out_what_differs_between_servers() {
        for (id, plain) in [
            (
                "unsloth/embeddinggemma-300m-GGUF:Q8_0",
                "embeddinggemma-300m",
            ),
            ("nomic-embed-text:latest", "nomic-embed-text"),
            (
                "text-embedding-nomic-embed-text-v1.5",
                "nomic-embed-text-v1.5",
            ),
            ("text-embedding-3-small", "text-embedding-3-small"),
            ("Qwen3-Embedding-0.6B-Q4_K_M.gguf", "qwen3-embedding-0.6b"),
            ("MiniCPM-OCR-1_0-IQ4_XS", "minicpm-ocr-1_0"),
            ("nvidia/nemotron-3.5-embed-1b:free", "nemotron-3.5-embed-1b"),
            ("qwen2.5-vl-7b-instruct", "qwen2.5-vl-7b-instruct"),
        ] {
            assert_eq!(plain_name(id), plain, "{id}");
        }
    }

    #[test]
    fn popular_models_get_the_prompts_they_are_trained_with() {
        assert_eq!(
            known_prompts("unsloth/embeddinggemma-300m-GGUF"),
            Some((SearchSettings::GEMMA_QUERY, SearchSettings::GEMMA_DOCUMENT))
        );
        assert_eq!(
            known_prompts("text-embedding-nomic-embed-text-v1.5"),
            Some(("search_query: ", "search_document: "))
        );
        assert_eq!(
            known_prompts("intfloat/multilingual-e5-large"),
            Some(("query: ", "passage: "))
        );
        assert!(known_prompts("BAAI/bge-small-en-v1.5").is_some());
        assert_eq!(known_prompts("BAAI/bge-m3"), None);
        assert_eq!(known_prompts("text-embedding-3-small"), None);
    }

    #[test]
    fn known_reading_models_are_named_by_their_files_too() {
        assert_eq!(
            reader_kind("deepseek-ai/DeepSeek-OCR-2"),
            Some(ReaderModel::DeepSeekOcr2)
        );
        assert_eq!(
            reader_kind("MiniCPM-OCR-1_0-Q8_0.gguf"),
            Some(ReaderModel::MiniCpmOcr)
        );
        assert_eq!(
            reader_kind("ggml-org/GLM-OCR-GGUF:Q8_0"),
            Some(ReaderModel::GlmOcr)
        );
        assert_eq!(reader_kind("qwen2.5-vl-7b-instruct"), None);
    }

    /// Downloads the real list (about 6 MB):
    /// cargo test -- --ignored models_dev_lists_models
    #[test]
    #[ignore]
    fn models_dev_lists_models() {
        let found = look_up("text-embedding-3-small", Some("openai")).unwrap();
        println!("{found:?}");
        assert!(found.is_some_and(|e| e.limit.context > 0));
    }
}
