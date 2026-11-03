//! Reading text with a vision model on a server: any server with an
//! OpenAI-style `/chat/completions` that takes images, such as llama.cpp,
//! vLLM or Ollama. The models the app knows (`ReaderModel`) get the prompt
//! and settings that suit them, and their answer is made plain text.

use std::io::Cursor;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{anyhow, Result};
use base64::Engine as _;
use regex::Regex;
use serde_json::{json, Value};

use crate::config::{ReaderModel, ReaderServer};
use crate::net::ServerError;

/// Bigger images are scaled down first: vision models read at a few
/// thousand pixels at most, and a smaller request goes faster.
const MAX_SIDE: u32 = 2048;

/// The longest answer: a model that falls into repeating itself stops
/// here instead of running for minutes. With the image it fits in
/// DeepSeek-OCR-2's 8192 tokens, as some servers refuse a request whose
/// limit doesn't fit the model.
const MAX_TOKENS: u32 = 4096;

impl ReaderModel {
    /// In the order Settings offers them.
    pub const ALL: [ReaderModel; 5] = [
        ReaderModel::DeepSeekOcr2,
        ReaderModel::UnlimitedOcr2,
        ReaderModel::MiniCpmOcr,
        ReaderModel::GlmOcr,
        ReaderModel::Other,
    ];

    /// The model's name; empty for another model.
    pub fn name(self) -> &'static str {
        match self {
            ReaderModel::DeepSeekOcr2 => "DeepSeek-OCR-2",
            ReaderModel::UnlimitedOcr2 => "Unlimited-OCR-2",
            ReaderModel::MiniCpmOcr => "MiniCPM-OCR",
            ReaderModel::GlmOcr => "GLM-OCR",
            ReaderModel::Other => "",
        }
    }

    /// The name the model's own guide serves it under; a server that runs
    /// one model ignores it.
    pub fn served_name(self) -> &'static str {
        match self {
            ReaderModel::DeepSeekOcr2 => "deepseek-ai/DeepSeek-OCR-2",
            ReaderModel::UnlimitedOcr2 => "Unlimited-OCR-2",
            ReaderModel::MiniCpmOcr => "minicpm-ocr",
            // llama.cpp serves it as `llama-server -hf ggml-org/GLM-OCR-GGUF`.
            ReaderModel::GlmOcr => "zai-org/GLM-OCR",
            ReaderModel::Other => "",
        }
    }

    /// The prompt each model is made for, for plain text (from its model
    /// page).
    pub fn prompt(self) -> &'static str {
        match self {
            ReaderModel::DeepSeekOcr2 => "Free OCR.",
            ReaderModel::UnlimitedOcr2 => "Multi page parsing.",
            ReaderModel::MiniCpmOcr | ReaderModel::GlmOcr => "Text Recognition:",
            ReaderModel::Other => ReaderServer::PROMPT,
        }
    }

    /// What the model's guide adds to each request. A server that doesn't
    /// know a setting ignores it.
    fn request_settings(self) -> Value {
        match self {
            // DeepSeek's models: vLLM's repetition guard (vLLM must be
            // started with it), and their special tokens kept, as the table
            // cell tags <td> and </td> are special tokens.
            ReaderModel::DeepSeekOcr2 => json!({
                "skip_special_tokens": false,
                "vllm_xargs": {
                    "ngram_size": 30,
                    "window_size": 90,
                    "whitelist_token_ids": [128821, 128822]
                }
            }),
            ReaderModel::UnlimitedOcr2 => json!({
                "skip_special_tokens": false,
                "vllm_xargs": { "ngram_size": 35, "window_size": 128 }
            }),
            // Its guides turn its thinking off: it would only slow the answer.
            ReaderModel::MiniCpmOcr => json!({
                "chat_template_kwargs": { "enable_thinking": false }
            }),
            ReaderModel::GlmOcr | ReaderModel::Other => json!({}),
        }
    }
}

/// The image the reading test sends: "Clipboard Manager" and a line with a
/// number, in black on white.
const TEST_IMAGE: &[u8] = include_bytes!("../../assets/read-test.png");

/// Reads the test image, to check that the model answers and to show how
/// well it reads: what it read, and how long that took.
pub fn test_read(cfg: ReaderServer) -> Result<(String, Duration)> {
    let img = image::load_from_memory(TEST_IMAGE)?;
    let start = std::time::Instant::now();
    let text = ServerReader::new(cfg).recognize(&img)?;
    Ok((text, start.elapsed()))
}

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
            "max_tokens": MAX_TOKENS,
            "stream": false
        });
        if let Value::Object(settings) = self.cfg.kind.request_settings() {
            for (key, value) in settings {
                body[&key] = value;
            }
        }
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
            .map_err(|e| ServerError::unreachable(&url, &e))?;
        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| ServerError::unreachable(&url, &e))?;
        if !status.is_success() {
            return Err(ServerError::from_answer(&url, status, &text).into());
        }
        let answer = serde_json::from_str(&text)
            .map_err(anyhow::Error::from)
            .and_then(|reply| reply_text(&reply))
            .map_err(|e| ServerError::bad_answer(&url, e))?;
        Ok(match self.cfg.kind {
            ReaderModel::Other => answer,
            _ => plain_text(&answer),
        })
    }
}

/// A known model's answer as plain text, so the read text is words to
/// search: its thinking, layout tags, boxes and page markers go, table
/// cells are separated by spaces and rows by lines, and formulas stay as
/// written.
fn plain_text(answer: &str) -> String {
    static THINKING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<think>.*?</think>").unwrap());
    // DeepSeek's layout boxes, each with the kind of block it marks:
    // <|ref|>title<|/ref|><|det|>[[x1, y1, x2, y2]]<|/det|>.
    static BOXES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(<\|ref\|>[^<>]*<\|/ref\|>)?<\|det\|>[^<>]*<\|/det\|>").unwrap()
    });
    // <|ref|>, <|grounding|> and the like, also with full-width bars, as
    // DeepSeek writes its own.
    static TOKENS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[|\x{FF5C}]/?[^|\x{FF5C}<>]{1,40}[|\x{FF5C}]>").unwrap());
    static CELL_ENDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</t[dh]>").unwrap());
    // Table rows, line breaks, and Unlimited-OCR's page markers.
    static ROW_ENDS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)</tr>|<br\s*/?>|</?PAGE>").unwrap());
    static TABLE_TAGS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)</?(table|thead|tbody|tfoot|tr|td|th|caption|colgroup|col)\b[^>]*>")
            .unwrap()
    });
    let text = THINKING.replace_all(answer, "");
    let text = BOXES.replace_all(&text, "");
    let text = TOKENS.replace_all(&text, "");
    let html_table = TABLE_TAGS.is_match(&text);
    let text = CELL_ENDS.replace_all(&text, " ");
    let text = ROW_ENDS.replace_all(&text, "\n");
    let mut text = TABLE_TAGS.replace_all(&text, "").into_owned();
    if html_table {
        for (entity, plain) in [
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&#39;", "'"),
            ("&nbsp;", " "),
            ("&amp;", "&"),
        ] {
            text = text.replace(entity, plain);
        }
    }
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let row = line.trim();
        if row.len() > 1 && row.starts_with('|') && row.ends_with('|') {
            // A markdown table row; the line under its header goes.
            let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
            let rule =
                |cell: &&str| !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':'));
            if !cells.iter().all(rule) {
                lines.push(cells.join(" "));
            }
            continue;
        }
        let line = line.trim_end();
        // At most one empty line in a row.
        if line.trim().is_empty() && lines.last().is_some_and(|l| l.trim().is_empty()) {
            continue;
        }
        lines.push(line.to_string());
    }
    lines.join("\n").trim().to_string()
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

    /// A stand-in chat server: answers one request with `answer` and keeps
    /// the request's JSON body.
    fn serve_once(answer: &str) -> (String, std::sync::mpsc::Receiver<Value>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let reply = json!({ "choices": [{ "message": { "content": answer } }] }).to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = Vec::new();
            let mut buf = [0u8; 65536];
            let body_start = loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    return;
                }
                request.extend_from_slice(&buf[..n]);
                if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break at + 4;
                }
            };
            let head = String::from_utf8_lossy(&request[..body_start]).to_lowercase();
            let length: usize = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            while request.len() < body_start + length {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
            }
            let _ = tx.send(serde_json::from_slice(&request[body_start..]).unwrap_or(Value::Null));
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        });
        (url, rx)
    }

    fn reader_for(kind: ReaderModel, url: String) -> ServerReader {
        ServerReader::new(ReaderServer {
            base_url: url,
            model: kind.served_name().to_string(),
            kind,
            prompt: kind.prompt().to_string(),
            ..Default::default()
        })
    }

    #[test]
    fn each_known_model_is_asked_as_its_guide_says() {
        let known = [
            ReaderModel::DeepSeekOcr2,
            ReaderModel::UnlimitedOcr2,
            ReaderModel::MiniCpmOcr,
            ReaderModel::GlmOcr,
        ];
        for kind in known {
            let (url, request) =
                serve_once("<|ref|>title<|/ref|><|det|>[[10, 20, 300, 40]]<|/det|>\nInvoice 42");
            let text = reader_for(kind, url)
                .recognize(&image::DynamicImage::new_rgb8(8, 8))
                .unwrap();
            assert_eq!(text, "Invoice 42");
            let body = request.recv().unwrap();
            assert_eq!(body["model"], kind.served_name());
            assert_eq!(body["messages"][0]["content"][1]["text"], kind.prompt());
            assert_eq!(body["max_tokens"], MAX_TOKENS);
            match kind {
                ReaderModel::DeepSeekOcr2 => {
                    assert_eq!(body["skip_special_tokens"], false);
                    assert_eq!(body["vllm_xargs"]["ngram_size"], 30);
                    assert_eq!(
                        body["vllm_xargs"]["whitelist_token_ids"],
                        json!([128821, 128822])
                    );
                }
                ReaderModel::UnlimitedOcr2 => {
                    assert_eq!(body["skip_special_tokens"], false);
                    assert_eq!(body["vllm_xargs"]["ngram_size"], 35);
                }
                ReaderModel::GlmOcr => {
                    assert!(body.get("vllm_xargs").is_none());
                    assert!(body.get("chat_template_kwargs").is_none());
                }
                _ => assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false),
            }
        }
    }

    #[test]
    fn the_reading_test_sends_its_image_and_returns_the_answer() {
        let (url, request) = serve_once("Clipboard Manager\nReading test: invoice 4821");
        let (text, _) = test_read(ReaderServer {
            base_url: url,
            ..Default::default()
        })
        .unwrap();
        assert!(text.starts_with("Clipboard Manager"));
        let body = request.recv().unwrap();
        let image = body["messages"][0]["content"][0]["image_url"]["url"]
            .as_str()
            .unwrap();
        assert!(image.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn another_model_gets_the_prompt_as_written_and_its_answer_as_it_came() {
        let (url, request) = serve_once("| a | b |");
        let reader = ServerReader::new(ReaderServer {
            base_url: url,
            prompt: "Read it.".into(),
            ..Default::default()
        });
        let text = reader
            .recognize(&image::DynamicImage::new_rgb8(8, 8))
            .unwrap();
        assert_eq!(text, "| a | b |");
        let body = request.recv().unwrap();
        assert_eq!(body["messages"][0]["content"][1]["text"], "Read it.");
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        for setting in [
            "model",
            "vllm_xargs",
            "skip_special_tokens",
            "chat_template_kwargs",
        ] {
            assert!(body.get(setting).is_none(), "{setting}");
        }
    }

    #[test]
    fn each_known_model_has_its_name_and_prompt() {
        for kind in ReaderModel::ALL {
            let known = kind != ReaderModel::Other;
            assert_eq!(!kind.name().is_empty(), known);
            assert_eq!(!kind.served_name().is_empty(), known);
            assert!(!kind.prompt().is_empty());
        }
        assert_eq!(ReaderModel::MiniCpmOcr.prompt(), "Text Recognition:");
        assert_eq!(ReaderModel::UnlimitedOcr2.prompt(), "Multi page parsing.");
    }

    #[test]
    fn markup_becomes_plain_text() {
        // DeepSeek's layout boxes with their kind of block, other tags, and
        // its own full-width tokens.
        assert_eq!(
            plain_text(
                "<|ref|>text<|/ref|><|det|>[[1, 2, 3, 4], [5, 6, 7, 8]]<|/det|>\n<|ref|>Total<|/ref|>: 12<\u{FF5C}end\u{2581}of\u{2581}sentence\u{FF5C}>"
            ),
            "Total: 12"
        );
        // Thinking, if the server left it on.
        assert_eq!(
            plain_text("<think>\nA receipt.\n</think>\n\nTotal: 12"),
            "Total: 12"
        );
        // Unlimited-OCR's page markers: pages don't run together.
        assert_eq!(
            plain_text("<PAGE>First page<PAGE>Second page"),
            "First page\nSecond page"
        );
        // An HTML table: cells with spaces, rows on lines, entities read.
        assert_eq!(
            plain_text("<table><tr><td>Item</td><td>Price</td></tr><tr><td>Tea &amp; milk</td><td>12</td></tr></table>"),
            "Item Price\nTea & milk 12"
        );
        // A markdown table: the line under the header goes.
        assert_eq!(
            plain_text("| Item | Price |\n|---|:---:|\n| Tea | 12 |"),
            "Item Price\nTea 12"
        );
        // Formulas and other text stay; empty lines shrink to one.
        assert_eq!(
            plain_text("E = mc^2\n\n\n\\frac{a}{b}"),
            "E = mc^2\n\n\\frac{a}{b}"
        );
    }
}
