//! Telling secrets from ordinary copies: the rules behind the "skip passwords and
//! keys" setting. Kept apart from the clipboard code so the rules can be read and
//! tested on their own.

/// What a copy that looks like a secret is, for the message that says it
/// wasn't saved (D57).
#[derive(Clone, Debug, PartialEq)]
pub struct Secret {
    pub kind: SecretKind,
    pub hint: Hint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SecretKind {
    Password,
    /// A token with a known prefix, or a name about a key or token.
    ApiKey,
    /// A JWT or a `Bearer` token.
    SignIn,
    PrivateKey,
    /// A web address that carries a password or a token.
    Link,
    /// A long random-looking key.
    Key,
}

/// Tells one secret from another without showing it.
#[derive(Clone, Debug, PartialEq)]
pub enum Hint {
    /// A key's public start and its last 4 characters, as websites show
    /// keys: "sk-" and "9f2a".
    Ends(String, String),
    /// The host of a link that carries a password.
    Host(String),
    /// Only its length in characters.
    Length(usize),
}

/// Text that looks like a credential: a private key block, a token with a
/// known prefix (also inside longer text such as an `.env` file), a JWT, a
/// `Bearer` token, a `password=...` pair, a URL that carries a password or a
/// token, a long random-looking key, or a generated password. Deliberately
/// conservative, so file names, paths, ordinary URLs, e-mail addresses,
/// hashes, IDs and code identifiers are still recorded.
pub fn looks_like_secret(text: &str) -> bool {
    find_secret(text).is_some()
}

/// The secret `text` looks like, with a hint that doesn't give it away; the
/// rules are those of `looks_like_secret`.
pub fn find_secret(text: &str) -> Option<Secret> {
    let t = text.trim();
    let found = |kind, hint| Some(Secret { kind, hint });
    let length = |s: &str| Hint::Length(s.chars().count());
    if t.contains("-----BEGIN") && t.contains("PRIVATE KEY") {
        return found(SecretKind::PrivateKey, length(t));
    }
    if t.len() < 12 || t.len() > 8192 {
        return None;
    }
    if t.contains(char::is_whitespace) {
        // In text with spaces, only unmistakable tokens count; the rest
        // could be prose or code.
        let lower = t.to_ascii_lowercase();
        if let Some(at) = lower.find("bearer ") {
            if let Some(token) = t[at + 7..]
                .split_whitespace()
                .next()
                .filter(|token| is_token(token, 16))
            {
                return found(SecretKind::SignIn, length(token));
            }
        }
        if !t.contains('\n') {
            if let Some(kind) = assignment_kind(t) {
                return found(kind, length(t));
            }
        }
        return t
            .split(|c: char| c.is_whitespace() || "=:\"'`,;()[]{}<>".contains(c))
            .find_map(|word| match known_prefix(word) {
                Some(prefix) => found(SecretKind::ApiKey, ends(prefix, word)),
                None if is_jwt(word) => found(SecretKind::SignIn, length(word)),
                None => None,
            });
    }
    if let Some(kind) = assignment_kind(t) {
        return found(kind, length(t));
    }
    if t.contains("://") {
        let host = crate::net::host(t).to_string();
        return url_carries_secret(t).then_some(Secret {
            kind: SecretKind::Link,
            hint: Hint::Host(host),
        });
    }
    if let Some(prefix) = known_prefix(t) {
        return found(SecretKind::ApiKey, ends(prefix, t));
    }
    if is_jwt(t) {
        return found(SecretKind::SignIn, length(t));
    }
    if t.len() > 512 || t.starts_with("www.") || t.contains('\\') || is_email(t) || is_file_name(t)
    {
        return None;
    }
    if is_random_token(t) || is_base64_key(t) {
        return found(SecretKind::Key, length(t));
    }
    if is_generated_password(t) {
        return found(SecretKind::Password, length(t));
    }
    None
}

/// A key's public start and its last 4 characters.
fn ends(prefix: &str, token: &str) -> Hint {
    let last: Vec<char> = token.chars().rev().take(4).collect();
    Hint::Ends(prefix.to_string(), last.into_iter().rev().collect())
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

/// The well-known prefix `t` starts with, followed by enough of a token.
fn known_prefix(t: &str) -> Option<&'static str> {
    PREFIXES
        .iter()
        .find(|(prefix, min)| {
            t.strip_prefix(prefix)
                .is_some_and(|rest| is_token(rest, *min))
        })
        .map(|(prefix, _)| *prefix)
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
/// token or key, and a value with no spaces and no code characters. The kind
/// follows the name.
fn assignment_kind(t: &str) -> Option<SecretKind> {
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
    let split = t.find(['=', ':'])?;
    let quotes = |c: char| c == '"' || c == '\'';
    let name = t[..split].trim().trim_matches(quotes).to_ascii_lowercase();
    let value = t[split + 1..].trim().trim_matches(quotes);
    let assignment = name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-. ".contains(c))
        && NAMES.iter().any(|n| name.ends_with(n))
        && (8..=256).contains(&value.len())
        && !value.contains(char::is_whitespace)
        && !value.contains(['(', ')', ';', '{', '}', '$']);
    if !assignment {
        return None;
    }
    Some(if name.ends_with("private_key") {
        SecretKind::PrivateKey
    } else if ["password", "passwd", "pwd"]
        .iter()
        .any(|n| name.ends_with(n))
    {
        SecretKind::Password
    } else {
        SecretKind::ApiKey
    })
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

#[cfg(test)]
mod tests {
    use super::looks_like_secret as secret;
    use super::{find_secret, Hint, Secret, SecretKind};

    #[test]
    fn a_secret_is_named_with_a_hint_that_hides_it() {
        let found = |t: &str| find_secret(t).expect(t);
        assert_eq!(
            found("sk-proj-AbCdEf1234567890GhIjKlMnOp"),
            Secret {
                kind: SecretKind::ApiKey,
                hint: Hint::Ends("sk-proj-".into(), "MnOp".into()),
            }
        );
        // Inside longer text, the key's own ends.
        assert_eq!(
            found("OPENAI_KEY = sk-proj-AbCdEf1234567890GhIjKlMnOp # mine").hint,
            Hint::Ends("sk-proj-".into(), "MnOp".into())
        );
        assert_eq!(
            found("password=hunter2hunter2"),
            Secret {
                kind: SecretKind::Password,
                hint: Hint::Length(23),
            }
        );
        assert_eq!(found("API_KEY: abcd1234efgh5678").kind, SecretKind::ApiKey);
        assert_eq!(
            found("https://me:Hunter2Hunter2@db.example.com:5432/app").hint,
            Hint::Host("db.example.com:5432".into())
        );
        assert_eq!(
            found("Bearer abcdefghijklmnop1234").kind,
            SecretKind::SignIn
        );
    }

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
