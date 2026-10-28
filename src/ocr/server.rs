//! Reading text with a vision model on a server: any server with an
//! OpenAI-style `/chat/completions` that takes images, such as llama.cpp,
//! vLLM (DeepSeek-OCR, PaddleOCR-VL) or Ollama.

use std::io::Cursor;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use serde_json::{json, Value};

use crate::config::ReaderServer;

/// Bigger images are scaled down first: vision models read at a few
/// thousand pixels at most, and a smaller request goes faster.
const MAX_SIDE: u32 = 2048;

pub struct ServerReader {
    cfg: ReaderServer,
    agent: ureq::Agent,
}

impl ServerReader {
    pub fn new(cfg: ReaderServer) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(180)))
            .http_status_as_error(false)
            .build()
            .into();
        Self { cfg, agent }
    }

    pub fn recognize(&self, img: &image::DynamicImage) -> Result<String> {
        let img = if img.width().max(img.height()) > MAX_SIDE {
            img.thumbnail(MAX_SIDE, MAX_SIDE)
        } else {
            img.clone()
        };
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        let image_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let mut body = json!({
            "messages": [{ "role": "user", "content": [
                { "type": "image_url", "image_url": { "url": image_url } },
                { "type": "text", "text": self.cfg.prompt.trim() }
            ]}],
            "temperature": 0,
            "stream": false
        });
        // An empty model name means "the server's default", so it is left out.
        if !self.cfg.model.trim().is_empty() {
            body["model"] = json!(self.cfg.model.trim());
        }
        let url = format!(
            "{}/chat/completions",
            self.cfg.base_url.trim().trim_end_matches('/')
        );
        let mut request = self.agent.post(&url);
        if !self.cfg.api_key.trim().is_empty() {
            request = request.header(
                "Authorization",
                format!("Bearer {}", self.cfg.api_key.trim()),
            );
        }
        let mut response = request
            .send_json(&body)
            .with_context(|| format!("cannot reach {url}"))?;
        let status = response.status();
        let text = response.body_mut().read_to_string()?;
        if status.as_u16() == 401 || status.as_u16() == 403 {
            bail!("{url} rejected the request ({status}): set the server's API key in Settings");
        }
        if !status.is_success() {
            bail!(
                "{url} returned {status}: {}",
                text.chars().take(400).collect::<String>()
            );
        }
        reply_text(&serde_json::from_str(&text)?)
    }
}

/// The model's answer in a chat reply.
fn reply_text(reply: &Value) -> Result<String> {
    reply["choices"][0]["message"]["content"]
        .as_str()
        .map(|text| text.trim().to_string())
        .ok_or_else(|| anyhow!("the server's reply has no text"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_is_read_from_the_reply() {
        let reply = json!({ "choices": [{ "message": { "role": "assistant",
            "content": "  Invoice 42\nTotal: 1,250 SAR\n" } }] });
        assert_eq!(reply_text(&reply).unwrap(), "Invoice 42\nTotal: 1,250 SAR");
        assert!(reply_text(&json!({ "error": "no model" })).is_err());
    }

    /// READER_URL=<server address> READER_KEY=<key> READER_MODEL=<model>
    /// OCR_IMAGE=<image with the word "clipboard"> cargo test -- --ignored server_reader
    #[test]
    #[ignore]
    fn server_reader_reads_text() {
        let env = |name| std::env::var(name).unwrap_or_default();
        let reader = ServerReader::new(ReaderServer {
            base_url: env("READER_URL"),
            api_key: env("READER_KEY"),
            model: env("READER_MODEL"),
            ..Default::default()
        });
        let text = reader
            .recognize(&image::open(env("OCR_IMAGE")).unwrap())
            .unwrap();
        println!("{text}");
        assert!(text.to_lowercase().contains("clipboard"));
    }
}
