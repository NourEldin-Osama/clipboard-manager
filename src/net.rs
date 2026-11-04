//! HTTP agents with timeouts, where a server address points, and why a
//! request to a model server failed. ureq has no timeouts by default, so one
//! stalled connection could hold a download or the update check forever, and
//! the OCR model download would freeze indexing with it.

use std::fmt;
use std::time::Duration;

use ureq::http::StatusCode;

use crate::i18n::t;

/// The host of a URL, with its port and without any user name or password,
/// as in `example.com:8080`.
pub fn host(url: &str) -> &str {
    let url = url.trim();
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host)
}

/// Whether `url` points at this PC: `localhost` or a loopback address such
/// as 127.0.0.1 or ::1. Anything else is another machine, which gets what
/// the app sends it.
pub fn is_on_this_pc(url: &str) -> bool {
    let host = host(url);
    let name = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    name.eq_ignore_ascii_case("localhost")
        || name
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// For small requests such as the update manifest: all within a minute.
pub fn quick() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into()
}

/// For downloads: connecting and the first response byte must be quick; the
/// body may then take as long as a slow line needs, up to `max_body`.
pub fn download(max_body: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_resolve(Some(Duration::from_secs(15)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_send_request(Some(Duration::from_secs(30)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(max_body))
        .build()
        .into()
}

/// Why a request to a model server failed, by kind, so the window can say it
/// in the user's language. `Display` says it in English, with the address,
/// for the log.
#[derive(Debug)]
pub enum ServerError {
    /// No answer: the server isn't running, or the address is wrong.
    Unreachable { url: String, reason: String },
    /// 401 or 403: the server needs an API key, or the key is wrong.
    NeedsKey { url: String, status: StatusCode },
    /// 404: the server doesn't know the address or the model.
    NotFound { url: String, said: String },
    /// Another 4xx: the server refused the request, in its own words.
    Refused {
        url: String,
        status: StatusCode,
        said: String,
    },
    /// 5xx: the server failed.
    Failed {
        url: String,
        status: StatusCode,
        said: String,
    },
    /// The server refused an image: it takes only text.
    NoImages { url: String, said: String },
    /// The server offers no models.
    NoModels { url: String },
    /// The answer is not what the app expected.
    BadAnswer { url: String, reason: String },
}

impl ServerError {
    /// The error for an answer whose status says it failed; `body` is the
    /// answer, where servers put their own message.
    pub fn from_answer(url: &str, status: StatusCode, body: &str) -> Self {
        let url = url.to_string();
        let said = server_words(body).unwrap_or_else(|| status.to_string());
        match status.as_u16() {
            401 | 403 => Self::NeedsKey { url, status },
            404 => Self::NotFound { url, said },
            400..=499 => Self::Refused { url, status, said },
            _ => Self::Failed { url, status, said },
        }
    }

    pub fn unreachable(url: &str, error: &ureq::Error) -> Self {
        Self::Unreachable {
            url: url.to_string(),
            reason: error.to_string(),
        }
    }

    pub fn bad_answer(url: &str, reason: impl fmt::Display) -> Self {
        Self::BadAnswer {
            url: url.to_string(),
            reason: reason.to_string(),
        }
    }

    /// The error in the interface language.
    pub fn text(&self) -> String {
        match self {
            Self::Unreachable { url, .. } => t!("error-server-unreachable", host = host(url)),
            Self::NeedsKey { .. } => t!("error-server-needs-key"),
            Self::NotFound { said, .. } => t!("error-server-not-found", said = said.as_str()),
            Self::Refused { said, .. } => t!("error-server-refused", said = said.as_str()),
            Self::Failed { status, said, .. } => t!(
                "error-server-failed",
                status = status.as_u16(),
                said = said.as_str()
            ),
            Self::NoImages { said, .. } => t!("error-server-no-images", said = said.as_str()),
            Self::NoModels { .. } => t!("error-server-no-models"),
            Self::BadAnswer { .. } => t!("error-server-bad-answer"),
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable { url, reason } => write!(f, "cannot reach {url}: {reason}"),
            Self::NeedsKey { url, status } => write!(
                f,
                "{url} answered {status}: the server needs an API key, or the key is wrong"
            ),
            Self::NotFound { url, said } => write!(f, "{url} answered 404 Not Found: {said}"),
            Self::Refused { url, status, said } => {
                write!(f, "{url} refused the request ({status}): {said}")
            }
            Self::Failed { url, status, said } => write!(f, "{url} failed ({status}): {said}"),
            Self::NoImages { url, said } => write!(f, "{url} doesn't take images: {said}"),
            Self::NoModels { url } => write!(f, "{url} offers no models"),
            Self::BadAnswer { url, reason } => write!(f, "unexpected answer from {url}: {reason}"),
        }
    }
}

impl std::error::Error for ServerError {}

/// What a server wrote about a failure: the message in its JSON answer, or
/// its answer as plain text, on one line and without a closing period, as
/// the texts quote it in the middle of a sentence. None for an empty answer
/// or a web page.
fn server_words(body: &str) -> Option<String> {
    let body = body.trim();
    let said = match serde_json::from_str::<serde_json::Value>(body) {
        // OpenAI and vLLM: {"error": {"message": ...}}; llama.cpp and
        // Ollama: {"error": ...}; FastAPI servers: {"detail": ...}, or a list
        // of {"msg": ...} for a request it can't read.
        Ok(answer) => [
            &answer["error"]["message"],
            &answer["error"],
            &answer["detail"],
            &answer["detail"][0]["msg"],
            &answer["message"],
        ]
        .into_iter()
        .find_map(|value| value.as_str())
        .unwrap_or(body)
        .to_string(),
        Err(_) if body.starts_with('<') => return None,
        Err(_) => body.to_string(),
    };
    let said = said.split_whitespace().collect::<Vec<_>>().join(" ");
    let said = said.trim_end_matches('.');
    let said = match said.char_indices().nth(300) {
        Some((end, _)) => format!("{}…", &said[..end]),
        None => said.to_string(),
    };
    (!said.is_empty()).then_some(said)
}

/// An error as the window shows it: a server's error in the interface
/// language, anything else as it is written (in English).
pub fn error_text(error: &anyhow::Error) -> String {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<ServerError>())
        .map_or_else(|| format!("{error:#}"), ServerError::text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_get_the_kind_of_their_status() {
        let url = "http://127.0.0.1:8080/v1/embeddings";
        let kind = |status: u16, body: &str| {
            ServerError::from_answer(url, StatusCode::from_u16(status).unwrap(), body)
        };
        assert!(matches!(kind(401, ""), ServerError::NeedsKey { .. }));
        assert!(matches!(kind(403, "no"), ServerError::NeedsKey { .. }));
        assert!(matches!(kind(404, ""), ServerError::NotFound { .. }));
        assert!(matches!(kind(400, ""), ServerError::Refused { .. }));
        assert!(matches!(kind(422, ""), ServerError::Refused { .. }));
        assert!(matches!(kind(500, ""), ServerError::Failed { .. }));
        assert!(matches!(kind(503, ""), ServerError::Failed { .. }));
    }

    #[test]
    fn the_servers_own_words_are_kept() {
        let said = |body: &str| match ServerError::from_answer(
            "http://x/v1",
            StatusCode::BAD_REQUEST,
            body,
        ) {
            ServerError::Refused { said, .. } => said,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            said(r#"{"error":{"message":"model not loaded"}}"#),
            "model not loaded"
        );
        assert_eq!(said(r#"{"error":"bad input"}"#), "bad input");
        assert_eq!(
            said(r#"{"detail":"'input' is required for embeddings"}"#),
            "'input' is required for embeddings"
        );
        assert_eq!(
            said(r#"{"detail":[{"msg":"Field required"}]}"#),
            "Field required"
        );
        assert_eq!(said("plain\n  text"), "plain text");
        assert_eq!(
            said(r#"{"detail":"'input' is required for embeddings."}"#),
            "'input' is required for embeddings"
        );
        // Nothing useful: the status says it.
        assert_eq!(said(""), "400 Bad Request");
        assert_eq!(said("<html><body>Bad</body></html>"), "400 Bad Request");
        assert_eq!(said(&"x".repeat(500)).chars().count(), 301);
    }

    #[test]
    fn the_window_gets_the_kind_and_the_log_the_details() {
        let error = anyhow::Error::new(ServerError::NeedsKey {
            url: "http://x/v1/models".into(),
            status: StatusCode::UNAUTHORIZED,
        })
        .context("checking the server");
        assert_eq!(error_text(&error), t!("error-server-needs-key"));
        assert!(format!("{error:#}").contains("http://x/v1/models answered 401 Unauthorized"));
        // Any other error is shown as it is.
        let other = anyhow::anyhow!("disk full");
        assert_eq!(error_text(&other), "disk full");
    }

    #[test]
    fn addresses_on_this_pc() {
        for url in [
            "http://127.0.0.1:8080/v1",
            "http://localhost:1234/v1",
            "http://LOCALHOST/v1",
            "http://[::1]:8001/v1",
            "http://127.5.6.7:1/v1",
            "http://user:secret@127.0.0.1:8080/v1",
            "127.0.0.1:8080/v1",
        ] {
            assert!(is_on_this_pc(url), "{url}");
        }
    }

    #[test]
    fn addresses_on_other_machines() {
        for url in [
            "https://api.example.com/v1",
            "http://192.168.1.20:8080/v1",
            "http://127.0.0.1.evil.example/v1",
            "http://localhost.evil.example/v1",
            "http://127.0.0.1@evil.example/v1",
            "",
        ] {
            assert!(!is_on_this_pc(url), "{url}");
        }
    }

    #[test]
    fn the_host_leaves_out_the_password() {
        assert_eq!(
            host("http://user:secret@example.com:8080/v1"),
            "example.com:8080"
        );
        assert_eq!(host("https://api.example.com/v1?x=1"), "api.example.com");
    }
}
