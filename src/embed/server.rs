//! Embeddings from a local OpenAI-compatible server.
//!
//! This is how GGUF models such as EmbeddingGemma run: llama.cpp's
//! `llama-server --embeddings` (which clipboard-manager can launch for you), Ollama or
//! LM Studio. vLLM works too and also serves BiUltraVBERT / ColUltraVBERT;
//! multi-vector responses are detected and scored with MaxSim.

use std::io::Cursor;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use image::DynamicImage;
use serde_json::{json, Value};

use super::{Embedder, Embedding, TextRole};
use crate::config::{LaunchConfig, ServerConfig};

/// The llama-server we started, kept globally so it can be stopped on exit
/// even though the worker thread never unwinds.
static CHILD: Mutex<Option<Child>> = Mutex::new(None);

pub fn stop_launched_server() {
    if let Some(mut c) = CHILD.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

pub struct ServerEmbedder {
    cfg: ServerConfig,
    agent: ureq::Agent,
}

impl ServerEmbedder {
    pub fn new(cfg: ServerConfig) -> Result<Self> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(300)))
            .http_status_as_error(false)
            .build()
            .into();
        let mut s = Self { cfg, agent };
        if let Some(launch) = s.cfg.launch.clone() {
            stop_launched_server();
            *CHILD.lock().unwrap() = Some(spawn_llama_server(&launch)?);
            s.cfg.base_url = format!("http://127.0.0.1:{}/v1", launch.port);
            s.wait_until_ready(Duration::from_secs(180))?;
        }
        Ok(s)
    }

    fn wait_until_ready(&mut self, limit: Duration) -> Result<()> {
        let start = Instant::now();
        let health = self.cfg.base_url.trim_end_matches("/v1").to_string() + "/health";
        loop {
            if let Some(child) = CHILD.lock().unwrap().as_mut() {
                if let Some(status) = child.try_wait()? {
                    bail!("llama-server exited during start-up ({status}); check the model paths");
                }
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

    fn post(&self, body: Value, role: &str) -> Result<Value> {
        let url = format!("{}/embeddings", self.cfg.base_url.trim_end_matches('/'));
        let mut req = self
            .agent
            .post(&url)
            // Other servers ignore this; the bundled Sentence Transformers server
            // uses it to apply the model's own query/document prompts.
            .header("X-Embedding-Role", role);
        if !self.cfg.api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.cfg.api_key.trim()));
        }
        let mut resp = req
            .send_json(&body)
            .with_context(|| format!("cannot reach {url}"))?;
        let status = resp.status();
        let text = resp.body_mut().read_to_string()?;
        if status.as_u16() == 401 || status.as_u16() == 403 {
            bail!("{url} rejected the request ({status}): set the server's API key in Settings");
        }
        if !status.is_success() {
            bail!("{url} returned {status}: {}", text.chars().take(400).collect::<String>());
        }
        Ok(serde_json::from_str(&text)?)
    }

    /// An empty model name means "the server's default", so the field is left out.
    fn with_model(&self, mut body: Value) -> Value {
        if !self.cfg.model.trim().is_empty() {
            body["model"] = json!(self.cfg.model.trim());
        }
        body
    }

    fn parse(&self, v: &Value) -> Result<Vec<Embedding>> {
        let data = v["data"].as_array().ok_or_else(|| anyhow!("response has no `data`"))?;
        let mut rows: Vec<(usize, Embedding)> = Vec::new();
        for (i, d) in data.iter().enumerate() {
            let idx = d["index"].as_u64().map(|x| x as usize).unwrap_or(i);
            let e = &d["embedding"];
            let arr = e.as_array().ok_or_else(|| anyhow!("embedding is not an array"))?;
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
    /// The model the server embeds with when no model is named, found by a
    /// small test request. `Some("")` means it works but didn't say its name.
    pub default: Option<String>,
}

pub fn list_models(base_url: &str, api_key: &str) -> Result<ServerModels> {
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
    let mut list_error: Option<anyhow::Error> = None;
    match get(&url) {
        Ok(mut resp) => {
            let status = resp.status();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                bail!("the server needs an API key (got {status}); check the API key field");
            }
            if status.is_success() {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    for k in ["data", "models"] {
                        for m in v[k].as_array().into_iter().flatten() {
                            if let Some(id) = m["id"].as_str().or(m["name"].as_str()).or(m["model"].as_str()) {
                                listed.push(id.to_string());
                            }
                        }
                    }
                }
            } else {
                // Ollama without the /v1 suffix.
                let tags = format!("{}/api/tags", base.trim_end_matches("/v1"));
                if let Ok(mut r) = get(&tags) {
                    if r.status().is_success() {
                        let v: Value = serde_json::from_str(&r.body_mut().read_to_string()?).unwrap_or_default();
                        for m in v["models"].as_array().into_iter().flatten() {
                            if let Some(n) = m["name"].as_str() {
                                listed.push(n.to_string());
                            }
                        }
                    }
                }
                if listed.is_empty() {
                    list_error = Some(anyhow!("{url} returned {status}"));
                }
            }
        }
        Err(e) => return Err(anyhow::Error::new(e).context(format!("cannot reach {url}"))),
    }
    listed.dedup();

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
    let default = match req.send_json(&json!({ "input": ["ping"], "encoding_format": "float" })) {
        Ok(mut r) if r.status().is_success() => {
            let v: Value = serde_json::from_str(&r.body_mut().read_to_string()?).unwrap_or_default();
            if v["data"][0]["embedding"].is_array() {
                Some(v["model"].as_str().unwrap_or("").to_string())
            } else {
                None
            }
        }
        _ => None,
    };
    if listed.is_empty() && default.is_none() {
        return Err(list_error.unwrap_or_else(|| anyhow!("the server offered no embedding models")));
    }
    Ok(ServerModels { listed, default })
}

fn to_f32s(v: &Value) -> Result<Vec<f32>> {
    v.as_array()
        .ok_or_else(|| anyhow!("expected a number array"))?
        .iter()
        .map(|x| x.as_f64().map(|f| f as f32).ok_or_else(|| anyhow!("non-numeric value")))
        .collect()
}

impl Embedder for ServerEmbedder {
    fn key(&self) -> String {
        let model = match &self.cfg.launch {
            Some(l) => l.gguf.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
            // The server's default model: tie the index to the server address.
            None if self.cfg.model.trim().is_empty() => format!("default@{}", self.cfg.base_url.trim().trim_end_matches('/')),
            None => self.cfg.model.trim().to_string(),
        };
        let salt = crate::store::fnv1a(
            format!("{}|{}|{}", self.cfg.query_prefix, self.cfg.document_prefix, self.cfg.truncate_dims).as_bytes(),
        );
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
        let role = if role == TextRole::Query { "query" } else { "document" };
        let v = self.post(self.with_model(json!({ "input": input, "encoding_format": "float" })), role)?;
        let out = self.parse(&v)?;
        if out.len() != texts.len() {
            bail!("server returned {} embeddings for {} inputs", out.len(), texts.len());
        }
        Ok(out)
    }

    fn embed_image(&mut self, img: &DynamicImage) -> Result<Embedding> {
        // Keep requests small; embedding models downscale anyway.
        let img = if img.width().max(img.height()) > 1536 { img.thumbnail(1536, 1536) } else { img.clone() };
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        let url = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png));
        // vLLM's multimodal embeddings request shape.
        let v = self.post(self.with_model(json!({
            "encoding_format": "float",
            "messages": [{ "role": "user", "content": [
                { "type": "image_url", "image_url": { "url": url } }
            ]}]
        })), "document")?;
        self.parse(&v)?.into_iter().next().ok_or_else(|| anyhow!("no embedding returned"))
    }
}

impl Drop for ServerEmbedder {
    fn drop(&mut self) {
        if self.cfg.launch.is_some() {
            stop_launched_server();
        }
    }
}

fn spawn_llama_server(l: &LaunchConfig) -> Result<Child> {
    let mut cmd = Command::new(&l.llama_server);
    cmd.arg("-m")
        .arg(&l.gguf)
        .args(["--embeddings", "--host", "127.0.0.1", "--port", &l.port.to_string()]);
    if let Some(mm) = &l.mmproj {
        cmd.arg("--mmproj").arg(mm);
    }
    cmd.args(l.extra_args.split_whitespace());
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().with_context(|| format!("starting {}", l.llama_server.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running server, e.g. `python tools/st_server.py --model sentence-transformers/clip-ViT-B-32`:
    /// EMBED_URL=http://127.0.0.1:8001/v1 SIGLIP_IMAGES=<dir> cargo test -- --ignored server
    #[test]
    #[ignore]
    fn server_lists_models_and_embeds() {
        let url = std::env::var("EMBED_URL").expect("EMBED_URL");
        let key = std::env::var("EMBED_KEY").unwrap_or_default();
        let found = list_models(&url, &key).unwrap();
        println!("models: {found:?}");
        let clip = found.listed.iter().find(|m| m.contains("clip")).expect("a CLIP model").clone();
        let mut e = ServerEmbedder::new(ServerConfig { model: clip, images: true, api_key: key, ..ServerConfig::plain(&url) }).unwrap();
        let imgs = std::path::PathBuf::from(std::env::var("SIGLIP_IMAGES").unwrap());
        let cat = e.embed_image(&image::open(imgs.join("cat.jpg")).unwrap()).unwrap();
        let car = e.embed_image(&image::open(imgs.join("car.jpg")).unwrap()).unwrap();
        let q = e.embed_texts(&["a photo of a cat".into(), "a photo of a car".into()], TextRole::Query).unwrap();
        println!("cat-q: cat {:.3} car {:.3} | car-q: cat {:.3} car {:.3}", q[0].score(&cat), q[0].score(&car), q[1].score(&cat), q[1].score(&car));
        assert!(q[0].score(&cat) > q[0].score(&car));
        assert!(q[1].score(&car) > q[1].score(&cat));
    }
}
