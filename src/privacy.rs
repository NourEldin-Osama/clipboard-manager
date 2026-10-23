//! Telling secrets from ordinary copies: the rules behind the "skip passwords and
//! keys" setting. Kept apart from the clipboard code so the rules can be read and
//! tested on their own.

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
