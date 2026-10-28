//! HTTP agents with timeouts, and where a server address points. ureq has no
//! timeouts by default, so one stalled connection could hold a download or
//! the update check forever, and the OCR model download would freeze
//! indexing with it.

use std::time::Duration;

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

#[cfg(test)]
mod tests {
    use super::*;

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
