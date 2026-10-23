//! Clipboard monitoring thread and helpers for writing back to the clipboard.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::store::fnv1a;

pub enum Captured {
    Text(String),
    Image(image::RgbaImage),
    Files(Vec<PathBuf>),
    /// A copy that was deliberately not recorded, with the reason to show.
    /// `text` is kept only when the user may still choose to save it (the
    /// secret filter can be wrong); a copy marked private by its app is not.
    Skipped {
        reason: &'static str,
        text: Option<String>,
    },
}

impl Captured {
    pub fn hash(&self) -> u64 {
        match self {
            Captured::Text(t) => fnv1a(t.as_bytes()),
            Captured::Image(i) => {
                fnv1a(i.as_raw()) ^ ((i.width() as u64) << 32 | i.height() as u64)
            }
            Captured::Files(f) => fnv1a(join_paths(f).as_bytes()),
            Captured::Skipped { .. } => 0,
        }
    }
}

pub fn join_paths(f: &[PathBuf]) -> String {
    f.iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Default)]
pub struct Flags {
    /// Monitoring on/off.
    pub enabled: AtomicBool,
    /// Skip exactly one upcoming change (the "ignore next copy" hotkey).
    pub ignore_next: AtomicBool,
    /// Don't record text that looks like a password, API key or token.
    pub skip_secrets: AtomicBool,
}

/// How long to keep retrying a clipboard that another program holds open.
/// Browsers and Office can take well over arboard's built-in ~25 ms.
const BUSY_RETRY_FOR: Duration = Duration::from_secs(2);

pub fn spawn_watcher(flags: Arc<Flags>, tx: Sender<Captured>, wake: impl Fn() + Send + 'static) {
    std::thread::Builder::new()
        .name("clipboard".into())
        .spawn(move || {
            let mut cb = loop {
                match arboard::Clipboard::new() {
                    Ok(c) => break c,
                    Err(e) => {
                        log::warn!("clipboard unavailable: {e}");
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            };
            let mut last_seq = sequence_number();
            // Whatever is on the clipboard at start-up is not a new copy.
            let mut last_hash = read_with_retry(&mut cb).ok().flatten().map(|c| c.hash());
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let seq = sequence_number();
                if seq.is_some() && seq == last_seq {
                    continue;
                }
                let c = match read_with_retry(&mut cb) {
                    Ok(Some(c)) => c,
                    // Empty, or a format we don't record (e.g. only HTML or a custom format).
                    Ok(None) => {
                        last_seq = seq;
                        continue;
                    }
                    Err(e) => {
                        log::warn!(
                            "clipboard stayed busy for {BUSY_RETRY_FOR:?}, skipping this copy: {e}"
                        );
                        last_seq = seq;
                        continue;
                    }
                };
                // Only mark the change as handled once it was actually read.
                last_seq = seq;
                let h = c.hash();
                // Without a change counter (Linux, macOS) the content hash is the only
                // way to tell a new copy from the same one. With it (Windows) every copy
                // counts, so copying the same text again still moves it to the top, and a
                // copy that was skipped earlier is judged again with current settings.
                if seq.is_none() && Some(h) == last_hash {
                    continue;
                }
                last_hash = Some(h);
                if flags.ignore_next.swap(false, Ordering::SeqCst) {
                    log::debug!("skipped one copy (ignore-next or our own copy)");
                    continue;
                }
                if !flags.enabled.load(Ordering::SeqCst) {
                    continue;
                }
                let c = if is_sensitive() {
                    Captured::Skipped {
                        reason: "Not saved: the app you copied from marked it as private",
                        text: None,
                    }
                } else {
                    match c {
                        Captured::Text(t)
                            if flags.skip_secrets.load(Ordering::SeqCst)
                                && looks_like_secret(&t) =>
                        {
                            Captured::Skipped {
                                reason:
                                    "Not saved: looks like a password or API key (see Settings)",
                                text: Some(t),
                            }
                        }
                        other => other,
                    }
                };
                if let Captured::Skipped { reason, .. } = &c {
                    log::info!("{reason}");
                }
                if tx.send(c).is_err() {
                    return;
                }
                wake();
            }
        })
        .expect("spawn clipboard watcher");
}

/// Reads the clipboard, backing off while another program holds it open.
/// `Ok(None)` means there is nothing we record; `Err` means it stayed busy.
fn read_with_retry(cb: &mut arboard::Clipboard) -> Result<Option<Captured>, arboard::Error> {
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(20);
    loop {
        match read(cb) {
            Err(arboard::Error::ClipboardOccupied) if start.elapsed() < BUSY_RETRY_FOR => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            r => return r,
        }
    }
}

fn read(cb: &mut arboard::Clipboard) -> Result<Option<Captured>, arboard::Error> {
    // A busy clipboard must be told apart from "this format isn't there".
    fn check<T>(r: Result<T, arboard::Error>) -> Result<Option<T>, arboard::Error> {
        match r {
            Ok(v) => Ok(Some(v)),
            Err(arboard::Error::ClipboardOccupied) => Err(arboard::Error::ClipboardOccupied),
            Err(_) => Ok(None),
        }
    }
    if let Some(files) = check(cb.get().file_list())? {
        if !files.is_empty() {
            return Ok(Some(Captured::Files(files)));
        }
    }
    if let Some(t) = check(cb.get_text())? {
        if !t.trim().is_empty() {
            return Ok(Some(Captured::Text(t)));
        }
    }
    if let Some(img) = check(cb.get_image())? {
        let (w, h) = (img.width as u32, img.height as u32);
        if let Some(buf) = image::RgbaImage::from_raw(w, h, img.bytes.into_owned()) {
            return Ok(Some(Captured::Image(buf)));
        }
    }
    Ok(None)
}

/// Cheap change counter on Windows, so we only read the clipboard when it changed.
#[cfg(windows)]
fn sequence_number() -> Option<u32> {
    clipboard_win::raw::seq_num().map(|n| n.get())
}

#[cfg(not(windows))]
fn sequence_number() -> Option<u32> {
    None
}

/// Password managers mark secrets with these formats; never record them.
#[cfg(windows)]
fn is_sensitive() -> bool {
    use clipboard_win::raw;
    let _guard = match clipboard_win::Clipboard::new_attempts(5) {
        Ok(g) => g,
        Err(_) => return false,
    };
    [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore",
    ]
    .iter()
    .filter_map(|n| raw::register_format(n))
    .any(|f| raw::is_format_avail(f.get()))
}

#[cfg(not(windows))]
fn is_sensitive() -> bool {
    false
}

/// Text that looks like a credential: a private key block, a token with a
/// known prefix (also inside longer text such as an `.env` file), a JWT, a
/// `Bearer` token, a `password=...` pair, a URL that carries a password or a
/// token, a long random-looking key, or a generated password. Deliberately
/// conservative, so file names, paths, ordinary URLs, e-mail addresses,
/// hashes, IDs and code identifiers are still recorded.
pub fn looks_like_secret(text: &str) -> bool {
    let t = text.trim();
    if t.contains("-----BEGIN") && t.contains("PRIVATE KEY") {
        return true;
    }
    if t.len() < 12 || t.len() > 8192 {
        return false;
    }
    if t.contains(char::is_whitespace) {
        // In text with spaces, only unmistakable tokens count; the rest
        // could be prose or code.
        let lower = t.to_ascii_lowercase();
        if let Some(at) = lower.find("bearer ") {
            if t[at + 7..]
                .split_whitespace()
                .next()
                .is_some_and(|token| is_token(token, 16))
            {
                return true;
            }
        }
        if !t.contains('\n') && is_secret_assignment(t) {
            return true;
        }
        return t
            .split(|c: char| c.is_whitespace() || "=:\"'`,;()[]{}<>".contains(c))
            .any(|word| has_known_prefix(word) || is_jwt(word));
    }
    if is_secret_assignment(t) {
        return true;
    }
    if t.contains("://") {
        return url_carries_secret(t);
    }
    if has_known_prefix(t) || is_jwt(t) {
        return true;
    }
    if t.len() > 512 || t.starts_with("www.") || t.contains('\\') || is_email(t) || is_file_name(t)
    {
        return false;
    }
    is_random_token(t) || is_base64_key(t) || is_generated_password(t)
}

/// Prefixes of well-known API keys and tokens, with the shortest rest.
const PREFIXES: [(&str, usize); 30] = [
    ("sk-ant-", 20),
    ("sk-proj-", 20),
    ("sk-", 20),
    ("sk_live_", 16),
    ("sk_test_", 16),
    ("pk_live_", 16),
    ("rk_live_", 16),
    ("ghp_", 30),
    ("gho_", 30),
    ("ghu_", 30),
    ("ghs_", 30),
    ("ghr_", 30),
    ("github_pat_", 30),
    ("glpat-", 20),
    ("xoxb-", 20),
    ("xoxp-", 20),
    ("xoxa-", 20),
    ("xoxr-", 20),
    ("xapp-", 20),
    ("AKIA", 16),
    ("ASIA", 16),
    ("AIza", 30),
    ("ya29.", 20),
    ("hf_", 30),
    ("npm_", 30),
    ("pypi-", 30),
    ("SG.", 20),
    ("shpat_", 20),
    ("dop_v1_", 40),
    ("gsk_", 30),
];

fn has_known_prefix(t: &str) -> bool {
    PREFIXES.iter().any(|(prefix, min)| {
        t.strip_prefix(prefix)
            .is_some_and(|rest| is_token(rest, *min))
    })
}

/// At least `min` characters of a token alphabet, with a digit in them, so a
/// name like `npm_config_registry` is not a token.
fn is_token(s: &str, min: usize) -> bool {
    s.len() >= min
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        && s.chars().any(|c| c.is_ascii_digit())
}

/// Three base64url parts, the first starting with `{"` (`eyJ`).
fn is_jwt(t: &str) -> bool {
    let parts: Vec<&str> = t.split('.').collect();
    t.starts_with("eyJ")
        && parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

/// One line like `password=...` or `API_KEY: ...`: a name about a password,
/// token or key, and a value with no spaces and no code characters.
fn is_secret_assignment(t: &str) -> bool {
    const NAMES: [&str; 10] = [
        "password",
        "passwd",
        "pwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "access_key",
        "private_key",
    ];
    let Some(split) = t.find(['=', ':']) else {
        return false;
    };
    let quotes = |c: char| c == '"' || c == '\'';
    let name = t[..split].trim().trim_matches(quotes).to_ascii_lowercase();
    let value = t[split + 1..].trim().trim_matches(quotes);
    name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-. ".contains(c))
        && NAMES.iter().any(|n| name.ends_with(n))
        && (8..=256).contains(&value.len())
        && !value.contains(char::is_whitespace)
        && !value.contains(['(', ')', ';', '{', '}', '$'])
}

/// A URL with a password in it, a token-like query value, or a webhook path.
fn url_carries_secret(t: &str) -> bool {
    const WEBHOOKS: [&str; 3] = [
        "hooks.slack.com/services/",
        "discord.com/api/webhooks/",
        "discordapp.com/api/webhooks/",
    ];
    const PARAMS: [&str; 10] = [
        "token",
        "access_token",
        "refresh_token",
        "api_key",
        "apikey",
        "key",
        "sig",
        "signature",
        "secret",
        "password",
    ];
    let Some((_, rest)) = t.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority
        .split_once('@')
        .is_some_and(|(user, _)| user.contains(':'))
    {
        return true;
    }
    if WEBHOOKS.iter().any(|w| rest.contains(w)) {
        return true;
    }
    let query = rest.split_once(['?', '#']).map_or("", |(_, q)| q);
    query
        .split(['&', '#'])
        .filter_map(|pair| pair.split_once('='))
        .any(|(k, v)| PARAMS.contains(&k.to_ascii_lowercase().as_str()) && v.len() >= 16)
}

fn is_email(t: &str) -> bool {
    let Some((local, domain)) = t.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain.rsplit_once('.').is_some_and(|(host, tld)| {
            !host.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
        })
}

/// `name.ext`, with an extension of 1 to 5 letters or digits.
fn is_file_name(t: &str) -> bool {
    t.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty()
            && (1..=5).contains(&ext.len())
            && ext.chars().all(|c| c.is_ascii_alphanumeric())
            && ext.chars().any(|c| c.is_ascii_alphabetic())
    })
}

/// The longest run of letters and digits.
fn longest_word(t: &str) -> &str {
    t.split(|c: char| !c.is_ascii_alphanumeric())
        .max_by_key(|w| w.len())
        .unwrap_or("")
}

/// How many separate groups of digits `s` has.
fn digit_groups(s: &str) -> usize {
    let mut groups = 0;
    let mut in_group = false;
    for c in s.chars() {
        let digit = c.is_ascii_digit();
        if digit && !in_group {
            groups += 1;
        }
        in_group = digit;
    }
    groups
}

/// One long run of letters and digits that mixes upper case, lower case and
/// digits, with the digits spread through it: a generated key, not an
/// identifier such as `ParseConfig2024` or a hash (lower case only).
fn is_random_token(t: &str) -> bool {
    let word = longest_word(t);
    word.len() >= 20
        && word.chars().any(|c| c.is_ascii_uppercase())
        && word.chars().any(|c| c.is_ascii_lowercase())
        && digit_groups(word) >= 3
        && entropy(word) >= 3.5
}

/// Base64 with `+` or `/`, such as AWS or Azure secret keys; a path has
/// shorter parts and fewer digits.
fn is_base64_key(t: &str) -> bool {
    t.len() >= 32
        && !t.starts_with('/')
        && t.contains(['+', '/'])
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c))
        && t.chars().any(|c| c.is_ascii_uppercase())
        && t.chars().any(|c| c.is_ascii_lowercase())
        && digit_groups(t) >= 2
        && longest_word(t).len() >= 12
        && entropy(t) >= 4.0
}

/// 12 to 64 characters with upper case, lower case, a digit and at least two
/// symbols such as `#` or `$`, and nothing that suggests code or a pattern.
fn is_generated_password(t: &str) -> bool {
    (12..=64).contains(&t.chars().count())
        && t.chars().any(|c| c.is_ascii_uppercase())
        && t.chars().any(|c| c.is_ascii_lowercase())
        && t.chars().any(|c| c.is_ascii_digit())
        && t.chars().filter(|c| "!@#$%^&*?~".contains(*c)).count() >= 2
        && !t.contains([
            '(', ')', '[', ']', '{', '}', '<', '>', '"', '\'', '`', ';', ',', '/', '=',
        ])
}

/// Shannon entropy in bits per character.
fn entropy(s: &str) -> f64 {
    let mut counts = std::collections::HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0usize) += 1;
    }
    let n = s.chars().count() as f64;
    counts
        .values()
        .map(|&k| {
            let p = k as f64 / n;
            -p * p.log2()
        })
        .sum()
}

/// One long-lived handle for writing. On Linux the copied data is served by
/// this handle, so dropping it right after a copy would make the data vanish.
static WRITER: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// Writes to the clipboard, retrying while another program holds it open.
fn write(f: impl Fn(&mut arboard::Clipboard) -> Result<(), arboard::Error>) -> anyhow::Result<()> {
    let mut guard = WRITER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new()?);
    }
    let cb = guard.as_mut().unwrap();
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(20);
    loop {
        match f(cb) {
            Ok(()) => return Ok(()),
            Err(arboard::Error::ClipboardOccupied) if start.elapsed() < BUSY_RETRY_FOR => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            Err(arboard::Error::ClipboardOccupied) => {
                anyhow::bail!("the clipboard is busy (another program is holding it open)")
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    write(|cb| cb.set_text(text))
}

pub fn copy_image(img: &image::RgbaImage) -> anyhow::Result<()> {
    write(|cb| {
        cb.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Borrowed(img.as_raw()),
        })
    })
}

pub fn copy_files(files: &[PathBuf]) -> anyhow::Result<()> {
    write(|cb| cb.set().file_list(files))
}

#[cfg(test)]
mod tests {
    use super::looks_like_secret as secret;

    #[test]
    fn secrets_are_detected() {
        let long_jwt = format!(
            "eyJhbGciOiJSUzI1NiJ9.{}.c2lnbmF0dXJl",
            "eyJzdWIiOiIxIn0".repeat(40)
        );
        for t in [
            "sk-test-0000aaaa1111bbbb2222cccc3333dddd",
            "sk-proj-AbCdEf1234567890GhIjKlMnOp",
            "ghp_aBcDeF0123456789aBcDeF0123456789abcd",
            "AKIAIOSFODNN7EXAMP1E",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJl",
            long_jwt.as_str(),
            "Xk9#mP2$vL7qR4tW8zN1",
            "hT9$kL2#mP5@qR8!",
            "aB3dE9fGh2JkLmN0pQrS7tUv",
            "wJa1rXUtnFEMI/K7MDENG/bPxRfiCY3XAMPLEKEY",
            "Bearer abcdef1234567890ABCDEFghij",
            "Authorization: Bearer abcdef1234567890ABCDEFghij",
            "OPENAI_API_KEY=sk-AbC123dEf456GhI789jKl012",
            "password: Hunter2Hunter2",
            "https://user:pa55word@example.com/repo.git",
            "https://example.com/callback#access_token=AbC123dEf456GhI789jKl0",
            "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX",
            "# settings\nAPI_TOKEN=ghp_aBcDeF0123456789aBcDeF0123456789abcd\nDEBUG=1",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----",
        ] {
            assert!(secret(t), "{t}");
        }
    }

    #[test]
    fn ordinary_text_is_kept() {
        for t in [
            "hello world",
            "Meeting with Ahmed moved to Thursday 3pm",
            "https://huggingface.co/unsloth/embeddinggemma-2-GGUF?x=AbC123dEf456",
            "https://example.com/search?q=rust+clipboard+manager&page=2",
            "C:\\Users\\me\\Documents\\report2024.docx",
            "someone@example.com",
            "Name.Surname2024@company.co",
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "da39a3ee5e6b4b0d3255bfef95601890afd80709",
            "550e8400-e29b-41d4-a716-446655440000",
            "internationalization",
            "QX7J2L",
            "/usr/local/bin/llama-server",
            "/Users/JohnSmith/Documents/MyVeryLongProjectName2024/build",
            "Report_Final_2024.pdf",
            "IMG_20240315_142530.jpg",
            "Invoice-2024-Q3-Final.xlsx",
            "MyDocument_v12_FINAL_2023-11-05.docx",
            "my-project-name-v2.1.0-release",
            "npm_config_registry",
            "SomeClassNameInCode2024",
            "getUserAccountSettings2",
            "^[A-Za-z0-9]+$",
            "password = getpass()",
            "token: the thing you pass along",
            "let token = read_token(&path);\nprintln!(\"{token}\");",
        ] {
            assert!(!secret(t), "{t}");
        }
    }
}
