//! Embeddings from a local OpenAI-compatible server.
//!
//! This is how GGUF models such as EmbeddingGemma run: llama.cpp's
//! `llama-server --embeddings` (which clipboard-manager can launch for you), Ollama or
//! LM Studio. vLLM works too and also serves BiUltraVBERT / ColUltraVBERT;
//! multi-vector responses are detected and scored with MaxSim.

use std::io::Cursor;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use base64::Engine as _;
use image::DynamicImage;
use serde_json::{json, Value};

use super::{Embedder, Embedding, TextRole};
use crate::config::{ServerConfig, ServerKind};
use crate::llama::{self, Role};
use crate::net::ServerError;

pub struct ServerEmbedder {
    cfg: ServerConfig,
    agent: ureq::Agent,
    /// How the server takes an image: as its type is known to, until it
    /// refuses that and takes the other way.
    image_request: ImageRequest,
}

/// How a server takes an image to embed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ImageRequest {
    /// vLLM's way: a chat message with the image (`messages`).
    Messages,
    /// llama.cpp's and OpenRouter's way: an input with a content list.
    Content,
}

impl ImageRequest {
    fn body(self, image_url: &str) -> Value {
        let image = json!([{ "type": "image_url", "image_url": { "url": image_url } }]);
        match self {
            Self::Messages => json!({
                "encoding_format": "float",
                "messages": [{ "role": "user", "content": image }]
            }),
            Self::Content => json!({
                "encoding_format": "float",
                "input": [{ "content": image }]
            }),
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Messages => Self::Content,
            Self::Content => Self::Messages,
        }
    }
}

impl ServerEmbedder {
    pub fn new(cfg: ServerConfig) -> Result<Self> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(300)))
            .http_status_as_error(false)
            .build()
            .into();
        let image_request = match cfg.server_kind {
            ServerKind::LlamaCpp | ServerKind::OpenRouter | ServerKind::Unsloth => {
                ImageRequest::Content
            }
            _ => ImageRequest::Messages,
        };
        let mut s = Self {
            cfg,
            agent,
            image_request,
        };
        if let Some(launch) = s.cfg.launch.clone() {
            llama::start(Role::Search, &launch)?;
            s.cfg.base_url = format!("http://127.0.0.1:{}/v1", launch.port);
            s.wait_until_ready(Duration::from_secs(180))?;
        }
        Ok(s)
    }

    fn wait_until_ready(&mut self, limit: Duration) -> Result<()> {
        let start = Instant::now();
        let health = self.cfg.base_url.trim_end_matches("/v1").to_string() + "/health";
        loop {
            if let Some(status) = llama::exited(Role::Search)? {
                bail!("llama-server exited during start-up ({status}); check the model paths");
            }
            if let Ok(r) = self.agent.get(&health).call() {
                if r.status().is_success() {
                    return Ok(());
                }
            }
            if start.elapsed() > limit {
                bail!("llama-server did not become ready within {limit:?}");
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.cfg.base_url.trim_end_matches('/'))
    }

    fn post(&self, body: Value, role: &str) -> Result<Value, ServerError> {
        let url = self.embeddings_url();
        let mut req = self
            .agent
            .post(&url)
            // Other servers ignore this; the bundled Sentence Transformers server
            // uses it to apply the model's own query/document prompts.
            .header("X-Embedding-Role", role);
        if !self.cfg.api_key.trim().is_empty() {
            req = req.header(
                "Authorization",
                format!("Bearer {}", self.cfg.api_key.trim()),
            );
        }
        let mut resp = req
            .send_json(&body)
            .map_err(|e| ServerError::unreachable(&url, &e))?;
        let status = resp.status();
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| ServerError::unreachable(&url, &e))?;
        if !status.is_success() {
            return Err(ServerError::from_answer(&url, status, &text));
        }
        serde_json::from_str(&text).map_err(|e| ServerError::bad_answer(&url, e))
    }

    /// Sends an image the given way. A server that can't take it refuses
    /// it, or fails as llama.cpp does without its image projector; an image
    /// too big, too soon, or a server busy or restarting is another error.
    fn post_image(&self, way: ImageRequest, image_url: &str) -> Result<Value, ServerError> {
        let body = self.request_body(way.body(image_url), TextRole::Document);
        self.post(body, "document").map_err(|e| match e {
            ServerError::Refused { url, status, said }
            | ServerError::Failed { url, status, said }
                if !matches!(status.as_u16(), 413 | 429 | 502 | 503 | 504) =>
            {
                ServerError::NoImages { url, said }
            }
            other => other,
        })
    }

    /// What every request carries besides its input: the model's name (an
    /// empty name means "the server's default", so it is left out), and for
    /// OpenRouter, whether the text is a search or an item, and that only
    /// providers that don't collect user data may answer, unless the user
    /// let the others answer too.
    fn request_body(&self, mut body: Value, role: TextRole) -> Value {
        if !self.cfg.model.trim().is_empty() {
            body["model"] = json!(self.cfg.model.trim());
        }
        if self.cfg.server_kind == ServerKind::OpenRouter {
            body["input_type"] = json!(match role {
                TextRole::Query => "search_query",
                TextRole::Document => "search_document",
            });
            if !self.cfg.allows_data_collection {
                body["provider"] = json!({ "data_collection": "deny" });
            }
        }
        body
    }

    fn parse(&self, v: &Value) -> Result<Vec<Embedding>> {
        let data = v["data"]
            .as_array()
            .ok_or_else(|| anyhow!("response has no `data`"))?;
        let mut rows: Vec<(usize, Embedding)> = Vec::new();
        for (i, d) in data.iter().enumerate() {
            let idx = d["index"].as_u64().map(|x| x as usize).unwrap_or(i);
            let e = &d["embedding"];
            let arr = e
                .as_array()
                .ok_or_else(|| anyhow!("embedding is not an array"))?;
            let emb = if arr.first().map(|x| x.is_array()).unwrap_or(false) {
                let vs = arr.iter().map(to_f32s).collect::<Result<Vec<_>>>()?;
                Embedding::multi(vs, self.cfg.truncate_dims)
            } else {
                Embedding::dense(to_f32s(e)?, self.cfg.truncate_dims)
            };
            rows.push((idx, emb));
        }
        rows.sort_by_key(|(i, _)| *i);
        Ok(rows.into_iter().map(|(_, e)| e).collect())
    }
}

/// What a server offers for embeddings.
#[derive(Clone, Debug, Default)]
pub struct ServerModels {
    /// From `GET /models` (llama.cpp, vLLM, LM Studio, the bundled Sentence
    /// Transformers server) or Ollama's `/api/tags`. Some servers list only
    /// chat models here.
    pub listed: Vec<String>,
    /// The embedding models of a server that lists them apart, as OpenRouter
    /// does at `/embeddings/models`.
    pub embedding: Vec<String>,
    /// The model the server embeds with when no model is named, found by a
    /// small test request. `Some("")` means it works but didn't say its name.
    pub default: Option<String>,
    /// The names a server gives its models besides their ids, by id, such
    /// as OpenRouter's "NVIDIA: Nemotron 3.5 Embed 8B (free)".
    pub names: std::collections::HashMap<String, String>,
}

impl ServerModels {
    /// The models to offer for searching.
    pub fn for_search(&self) -> &[String] {
        if self.embedding.is_empty() {
            &self.listed
        } else {
            &self.embedding
        }
    }
}

/// What a server offers. With `embedding_list`, the embedding models are
/// also asked for apart (OpenRouter).
pub fn list_models(base_url: &str, api_key: &str, embedding_list: bool) -> Result<ServerModels> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(4)))
        .http_status_as_error(false)
        .build()
        .into();
    let base = base_url.trim().trim_end_matches('/');
    let key = api_key.trim();
    let get = |u: &str| {
        let mut r = agent.get(u);
        if !key.is_empty() {
            r = r.header("Authorization", format!("Bearer {key}"));
        }
        r.call()
    };
    let url = format!("{base}/models");
    let mut listed: Vec<String> = Vec::new();
    let mut names = std::collections::HashMap::new();
    let mut list_error: Option<ServerError> = None;
    match get(&url) {
        Ok(mut resp) => {
            let status = resp.status();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ServerError::from_answer(&url, status, &text).into());
            }
            if status.is_success() {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    for k in ["data", "models"] {
                        for m in v[k].as_array().into_iter().flatten() {
                            if let Some(id) = m["id"]
                                .as_str()
                                .or(m["name"].as_str())
                                .or(m["model"].as_str())
                            {
                                listed.push(id.to_string());
                            }
                            note_name(&mut names, m);
                        }
                    }
                }
            } else {
                // Ollama without the /v1 suffix.
                let tags = format!("{}/api/tags", base.trim_end_matches("/v1"));
                if let Ok(mut r) = get(&tags) {
                    if r.status().is_success() {
                        let v: Value = serde_json::from_str(&r.body_mut().read_to_string()?)
                            .unwrap_or_default();
                        for m in v["models"].as_array().into_iter().flatten() {
                            if let Some(n) = m["name"].as_str() {
                                listed.push(n.to_string());
                            }
                        }
                    }
                }
                if listed.is_empty() {
                    list_error = Some(ServerError::from_answer(&url, status, &text));
                }
            }
        }
        Err(e) => return Err(ServerError::unreachable(&url, &e).into()),
    }
    listed.dedup();
    let mut embedding: Vec<String> = Vec::new();
    if embedding_list {
        if let Ok(mut r) = get(&format!("{base}/embeddings/models")) {
            if r.status().is_success() {
                let v: Value =
                    serde_json::from_str(&r.body_mut().read_to_string()?).unwrap_or_default();
                for m in v["data"].as_array().into_iter().flatten() {
                    if let Some(id) = m["id"].as_str() {
                        embedding.push(id.to_string());
                    }
                    note_name(&mut names, m);
                }
            }
        }
    }

    // Ask for one embedding without naming a model, to find the server's default.
    let probe: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut req = probe.post(&format!("{base}/embeddings"));
    if !key.is_empty() {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let default = match req.send_json(json!({ "input": ["ping"], "encoding_format": "float" })) {
        Ok(mut r) if r.status().is_success() => {
            let v: Value =
                serde_json::from_str(&r.body_mut().read_to_string()?).unwrap_or_default();
            if v["data"][0]["embedding"].is_array() {
                Some(v["model"].as_str().unwrap_or("").to_string())
            } else {
                None
            }
        }
        _ => None,
    };
    if listed.is_empty() && embedding.is_empty() && default.is_none() {
        let url = base.to_string();
        return Err(list_error.unwrap_or(ServerError::NoModels { url }).into());
    }
    Ok(ServerModels {
        listed,
        embedding,
        default,
        names,
    })
}

/// Notes a listed model's name, when the server gives it one besides its id.
fn note_name(names: &mut std::collections::HashMap<String, String>, model: &Value) {
    names.extend(super::local::name_of(model));
}

/// What a test request to a search model found.
#[derive(Debug)]
pub struct SearchTest {
    /// Numbers in each vector the model gives, before any shortening.
    pub size: usize,
    /// Vectors for each text: 1, or more for a multi-vector model.
    pub vectors: usize,
    pub took: Duration,
    /// With Send images on: whether the server took the test image.
    pub images: Option<Result<()>>,
}

/// Embeds one short text, to check that the model answers and to see how
/// big its vectors are, and the test image with Send images on. It never
/// starts or stops llama-server: one the app started is asked as it runs.
pub fn test_search(mut cfg: ServerConfig) -> Result<SearchTest> {
    if let Some(launch) = cfg.launch.take() {
        cfg.base_url = format!("http://127.0.0.1:{}/v1", launch.port);
    }
    cfg.truncate_dims = 0;
    let mut embedder = ServerEmbedder::new(cfg)?;
    let start = Instant::now();
    let mut out = embedder.embed_texts(&["Clipboard Manager test".into()], TextRole::Query)?;
    let took = start.elapsed();
    let (size, vectors) = match out.pop().ok_or_else(|| anyhow!("no embedding returned"))? {
        Embedding::Dense(v) => (v.len(), 1),
        Embedding::Multi(vs) => (vs.first().map_or(0, Vec::len), vs.len()),
    };
    let images = embedder.cfg.images.then(|| {
        let image = image::load_from_memory(crate::ocr::TEST_IMAGE)?;
        embedder.embed_image(&image).map(|_| ()).inspect_err(|e| {
            log::warn!("testing images: {e:#}");
        })
    });
    Ok(SearchTest {
        size,
        vectors,
        took,
        images,
    })
}

fn to_f32s(v: &Value) -> Result<Vec<f32>> {
    v.as_array()
        .ok_or_else(|| anyhow!("expected a number array"))?
        .iter()
        .map(|x| {
            x.as_f64()
                .map(|f| f as f32)
                .ok_or_else(|| anyhow!("non-numeric value"))
        })
        .collect()
}

impl Embedder for ServerEmbedder {
    fn key(&self) -> String {
        let model = match &self.cfg.launch {
            Some(l) => l
                .gguf
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            // The server's default model: tie the index to the server address.
            None if self.cfg.model.trim().is_empty() => {
                format!("default@{}", self.cfg.base_url.trim().trim_end_matches('/'))
            }
            None => self.cfg.model.trim().to_string(),
        };
        let mut settings = format!(
            "{}|{}|{}",
            self.cfg.query_prefix, self.cfg.document_prefix, self.cfg.truncate_dims
        );
        // OpenRouter is told whether a text is a search or an item, which
        // changes its vectors.
        if self.cfg.server_kind == ServerKind::OpenRouter {
            settings.push_str("|input_type");
        }
        let salt = crate::store::fnv1a(settings.as_bytes());
        format!("server:{model}:{salt:x}")
    }

    fn supports_images(&self) -> bool {
        self.cfg.images
    }

    fn embed_texts(&mut self, texts: &[String], role: TextRole) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let prefix = match role {
            TextRole::Query => &self.cfg.query_prefix,
            TextRole::Document => &self.cfg.document_prefix,
        };
        let input: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
        let body = self.request_body(json!({ "input": input, "encoding_format": "float" }), role);
        let role = if role == TextRole::Query {
            "query"
        } else {
            "document"
        };
        let v = self.post(body, role)?;
        let url = self.embeddings_url();
        let out = self
            .parse(&v)
            .map_err(|e| ServerError::bad_answer(&url, e))?;
        if out.len() != texts.len() {
            let count = format!("{} embeddings for {} inputs", out.len(), texts.len());
            return Err(ServerError::bad_answer(&url, count).into());
        }
        Ok(out)
    }

    fn embed_image(&mut self, img: &DynamicImage) -> Result<Embedding> {
        // Keep requests small; embedding models downscale anyway.
        let img = if img.width().max(img.height()) > 1536 {
            img.thumbnail(1536, 1536)
        } else {
            img.clone()
        };
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        let url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        // A server that refuses one way may take the other; it then gets
        // that way from now on. Refused both ways, the first answer says why.
        let first = self.image_request;
        let v = match self.post_image(first, &url) {
            Err(refused @ ServerError::NoImages { .. }) => {
                match self.post_image(first.other(), &url) {
                    Ok(v) => {
                        self.image_request = first.other();
                        v
                    }
                    Err(ServerError::NoImages { .. }) => return Err(refused.into()),
                    Err(other) => return Err(other.into()),
                }
            }
            answer => answer?,
        };
        let url = self.embeddings_url();
        let embedding = self
            .parse(&v)
            .map_err(|e| ServerError::bad_answer(&url, e))?
            .into_iter()
            .next();
        embedding.ok_or_else(|| ServerError::bad_answer(&url, "no embedding").into())
    }
}

impl Drop for ServerEmbedder {
    fn drop(&mut self) {
        if self.cfg.launch.is_some() {
            llama::stop(Role::Search);
        }
    }
}

/// A stand-in embedding server for tests.
#[cfg(test)]
mod stand_in {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    use serde_json::{json, Value};

    use super::ImageRequest;

    /// Starts a server that answers each text with a small vector, and
    /// refuses images as a server that takes only text does, unless they
    /// come the way it `takes`. Its address ends with `/v1`; it runs until
    /// the tests end.
    pub fn serve(takes: Option<ImageRequest>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || answer(stream, takes));
            }
        });
        url
    }

    fn answer(mut stream: TcpStream, takes: Option<ImageRequest>) {
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];
        let body_start = loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
            if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                break at + 4;
            }
        };
        let head = String::from_utf8_lossy(&request[..body_start]).to_lowercase();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0);
        while request.len() < body_start + length {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        let asked: Value = serde_json::from_slice(&request[body_start..]).unwrap_or_default();
        let inputs = asked["input"].as_array().cloned().unwrap_or_default();
        let image = if asked["messages"].is_array() {
            Some(ImageRequest::Messages)
        } else if inputs.iter().any(|input| input.get("content").is_some()) {
            Some(ImageRequest::Content)
        } else {
            None
        };
        let count = match image {
            None => inputs.len(),
            Some(way) if takes == Some(way) => 1,
            Some(_) => 0,
        };
        let (status, body) = if count == 0 {
            (
                "400 Bad Request",
                json!({ "detail": "'input' is required for embeddings" }),
            )
        } else {
            let data: Vec<Value> = (0..count)
                .map(|i| json!({ "index": i, "embedding": [1.0, 0.5, i as f64] }))
                .collect();
            ("200 OK", json!({ "data": data, "model": "stand-in" }))
        };
        let body = body.to_string();
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_says_whether_the_server_takes_images() {
        let text_only = stand_in::serve(None);
        let found = test_search(ServerConfig {
            images: true,
            ..ServerConfig::plain(&text_only)
        })
        .unwrap();
        assert_eq!((found.size, found.vectors), (3, 1));
        let refused = found.images.unwrap().unwrap_err();
        assert!(
            matches!(
                refused.downcast_ref::<ServerError>(),
                Some(ServerError::NoImages { said, .. }) if said == "'input' is required for embeddings"
            ),
            "{refused:#}"
        );
        // Without Send images, no image is sent.
        assert!(test_search(ServerConfig::plain(&text_only))
            .unwrap()
            .images
            .is_none());
        let with_images = stand_in::serve(Some(ImageRequest::Messages));
        let found = test_search(ServerConfig {
            images: true,
            ..ServerConfig::plain(&with_images)
        })
        .unwrap();
        assert!(found.images.unwrap().is_ok());
    }

    #[test]
    fn images_go_the_way_the_server_takes_them() {
        let image = DynamicImage::new_rgb8(8, 8);
        let embedder = |server_kind, url: &str| {
            ServerEmbedder::new(ServerConfig {
                server_kind,
                images: true,
                ..ServerConfig::plain(url)
            })
            .unwrap()
        };
        // llama.cpp takes them its own way at once.
        let llama_cpp = stand_in::serve(Some(ImageRequest::Content));
        let mut e = embedder(ServerKind::LlamaCpp, &llama_cpp);
        assert!(e.embed_image(&image).is_ok());
        assert_eq!(e.image_request, ImageRequest::Content);
        // A server of another type that takes only that way gets it after
        // one refusal, and from then on.
        let mut e = embedder(ServerKind::Other, &llama_cpp);
        assert_eq!(e.image_request, ImageRequest::Messages);
        assert!(e.embed_image(&image).is_ok());
        assert_eq!(e.image_request, ImageRequest::Content);
        // vLLM's way works for a llama.cpp type that is vLLM after all.
        let vllm = stand_in::serve(Some(ImageRequest::Messages));
        let mut e = embedder(ServerKind::LlamaCpp, &vllm);
        assert!(e.embed_image(&image).is_ok());
        assert_eq!(e.image_request, ImageRequest::Messages);
    }

    #[test]
    fn openrouter_hears_what_each_text_is_for_and_to_keep_no_data() {
        let embedder = |server_kind| {
            ServerEmbedder::new(ServerConfig {
                server_kind,
                model: "m".into(),
                ..ServerConfig::plain("https://openrouter.ai/api/v1")
            })
            .unwrap()
        };
        let openrouter = embedder(ServerKind::OpenRouter);
        let search = openrouter.request_body(json!({ "input": ["a"] }), TextRole::Query);
        assert_eq!(search["input_type"], "search_query");
        assert_eq!(search["provider"]["data_collection"], "deny");
        let item = openrouter.request_body(json!({ "input": ["a"] }), TextRole::Document);
        assert_eq!(item["input_type"], "search_document");
        // Unless the user lets every provider answer.
        let every = ServerEmbedder::new(ServerConfig {
            server_kind: ServerKind::OpenRouter,
            allows_data_collection: true,
            ..ServerConfig::plain("https://openrouter.ai/api/v1")
        })
        .unwrap();
        let body = every.request_body(json!({ "input": ["a"] }), TextRole::Query);
        assert!(body.get("provider").is_none(), "{body}");
        // Other servers get neither, and keep the index they have.
        let other = embedder(ServerKind::Other);
        let body = other.request_body(json!({ "input": ["a"] }), TextRole::Query);
        assert!(body.get("input_type").is_none() && body.get("provider").is_none());
        assert_eq!(body["model"], "m");
        let before = format!("server:m:{:x}", crate::store::fnv1a(b"||0"));
        assert_eq!(other.key(), before);
        assert_ne!(openrouter.key(), before, "OpenRouter's index is made again");
    }

    /// Needs a running server, e.g. `python tools/st_server.py --model sentence-transformers/clip-ViT-B-32`:
    /// EMBED_URL=http://127.0.0.1:8001/v1 SIGLIP_IMAGES=<dir> cargo test -- --ignored server
    #[test]
    #[ignore]
    fn server_lists_models_and_embeds() {
        let url = std::env::var("EMBED_URL").expect("EMBED_URL");
        let key = std::env::var("EMBED_KEY").unwrap_or_default();
        let found = list_models(&url, &key, false).unwrap();
        println!("models: {found:?}");
        let clip = found
            .listed
            .iter()
            .find(|m| m.contains("clip"))
            .expect("a CLIP model")
            .clone();
        let mut e = ServerEmbedder::new(ServerConfig {
            model: clip,
            images: true,
            api_key: key,
            ..ServerConfig::plain(&url)
        })
        .unwrap();
        let imgs = std::path::PathBuf::from(std::env::var("SIGLIP_IMAGES").unwrap());
        let cat = e
            .embed_image(&image::open(imgs.join("cat.jpg")).unwrap())
            .unwrap();
        let car = e
            .embed_image(&image::open(imgs.join("car.jpg")).unwrap())
            .unwrap();
        let q = e
            .embed_texts(
                &["a photo of a cat".into(), "a photo of a car".into()],
                TextRole::Query,
            )
            .unwrap();
        println!(
            "cat-q: cat {:.3} car {:.3} | car-q: cat {:.3} car {:.3}",
            q[0].score(&cat),
            q[0].score(&car),
            q[1].score(&cat),
            q[1].score(&car)
        );
        assert!(q[0].score(&cat) > q[0].score(&car));
        assert!(q[1].score(&car) > q[1].score(&cat));
    }
}
