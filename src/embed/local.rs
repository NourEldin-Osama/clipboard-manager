//! Model servers on this PC and the models a server lists (D66, D67). Only
//! `GET /models` is asked, with nothing from the history, and the servers
//! found by themselves are only ever looked for on 127.0.0.1.

use std::time::Duration;

use serde_json::Value;

use crate::config::ServerKind;

/// A model server that answered on this PC at its type's usual address.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalServer {
    pub kind: ServerKind,
    pub base_url: String,
    /// The models it lists; empty when it wants an API key.
    pub models: Vec<String>,
    /// It answered that it needs an API key.
    pub needs_key: bool,
}

/// The server types that run on this PC, each looked for at its usual
/// address.
const LOCAL_KINDS: [ServerKind; 5] = [
    ServerKind::LlamaCpp,
    ServerKind::Ollama,
    ServerKind::LmStudio,
    ServerKind::Vllm,
    ServerKind::Unsloth,
];

/// Tests move every usual port by this much, so a stand-in server can take
/// a real one's place without the real ones on this PC being asked.
const PORT_OFFSET: &str = "CLIPBOARD_MANAGER_LOCAL_PORT_OFFSET";

/// Asks each usual address on this PC which models it has, all at once. A
/// port with nothing on it answers at once, and each gets at most about a
/// second and a half.
pub fn find() -> Vec<LocalServer> {
    let offset = std::env::var(PORT_OFFSET)
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(0);
    std::thread::scope(|scope| {
        let asks: Vec<_> = LOCAL_KINDS
            .iter()
            .map(|&kind| scope.spawn(move || ask(kind, offset)))
            .collect();
        asks.into_iter()
            .filter_map(|ask| ask.join().ok().flatten())
            .collect()
    })
}

/// A type's usual address with its port moved by `offset`.
fn usual_address(kind: ServerKind, offset: u16) -> Option<String> {
    let usual = kind.address();
    let (head, rest) = usual.rsplit_once(':')?;
    let (port, path) = rest.split_once('/').unwrap_or((rest, ""));
    let port = port.parse::<u16>().ok()?.checked_add(offset)?;
    Some(format!("{head}:{port}/{path}"))
}

fn ask(kind: ServerKind, offset: u16) -> Option<LocalServer> {
    let base_url = usual_address(kind, offset)?;
    match models_at(&base_url, "", kind) {
        Listed::Models(models) if !models.is_empty() => Some(LocalServer {
            kind,
            base_url,
            models,
            needs_key: false,
        }),
        Listed::NeedsKey => Some(LocalServer {
            kind,
            base_url,
            models: Vec::new(),
            needs_key: true,
        }),
        _ => None,
    }
}

/// What a server answered when asked for its models.
#[derive(Debug, PartialEq)]
pub enum Listed {
    Models(Vec<String>),
    NeedsKey,
    /// Nothing answered, or not as a model server does.
    Nothing,
}

/// The models the server at `base_url` lists, asked quickly: `/models`,
/// Ollama's `/api/tags` when that has none, and OpenRouter's embedding
/// models, which it lists apart.
pub fn models_at(base_url: &str, api_key: &str, kind: ServerKind) -> Listed {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_millis(500)))
        .timeout_global(Some(Duration::from_millis(1500)))
        .http_status_as_error(false)
        .build()
        .into();
    let base = base_url.trim().trim_end_matches('/');
    let key = api_key.trim();
    let get = |url: &str| {
        let mut request = agent.get(url);
        if !key.is_empty() {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        let mut response = request.call().ok()?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .ok()
            .unwrap_or_default();
        Some((status, text))
    };
    let lists = if kind == ServerKind::OpenRouter {
        vec![format!("{base}/embeddings/models")]
    } else {
        vec![
            format!("{base}/models"),
            format!("{}/api/tags", base.trim_end_matches("/v1")),
        ]
    };
    for (i, url) in lists.iter().enumerate() {
        match get(url) {
            None => return Listed::Nothing,
            // Only the list every model server has says whether it wants a key.
            Some((401 | 403, _)) if i == 0 => return Listed::NeedsKey,
            Some((status, text)) if (200..300).contains(&status) => {
                let models = model_ids(&text);
                if !models.is_empty() {
                    return Listed::Models(models);
                }
            }
            Some(_) => {}
        }
    }
    Listed::Nothing
}

/// The model names in a server's list of models: OpenAI's `data`, or
/// Ollama's `models`, by `id`, `name` or `model`.
pub fn model_ids(text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = Vec::new();
    for k in ["data", "models"] {
        for m in v[k].as_array().into_iter().flatten() {
            if let Some(id) = m["id"]
                .as_str()
                .or(m["name"].as_str())
                .or(m["model"].as_str())
            {
                if !ids.iter().any(|i| i == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usual_addresses_move_by_the_offset() {
        assert_eq!(
            usual_address(ServerKind::LlamaCpp, 0).as_deref(),
            Some("http://127.0.0.1:8080/v1")
        );
        assert_eq!(
            usual_address(ServerKind::Ollama, 10000).as_deref(),
            Some("http://127.0.0.1:21434/v1")
        );
        assert_eq!(usual_address(ServerKind::Other, 0), None);
    }

    #[test]
    fn model_lists_of_every_kind_are_read() {
        let openai = r#"{"data":[{"id":"embeddinggemma-300m"},{"id":"glm-ocr"}]}"#;
        assert_eq!(model_ids(openai), ["embeddinggemma-300m", "glm-ocr"]);
        let ollama =
            r#"{"models":[{"name":"nomic-embed-text:latest"},{"name":"nomic-embed-text:latest"}]}"#;
        assert_eq!(model_ids(ollama), ["nomic-embed-text:latest"]);
        assert!(model_ids("not json").is_empty());
    }

    #[test]
    fn nothing_on_a_port_is_nothing() {
        // A port nothing listens on refuses at once.
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);
        let url = format!("http://127.0.0.1:{port}/v1");
        assert_eq!(models_at(&url, "", ServerKind::LlamaCpp), Listed::Nothing);
    }
}
