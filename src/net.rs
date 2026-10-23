//! HTTP agents with timeouts. ureq has none by default, so one stalled
//! connection could hold a download or the update check forever, and the
//! OCR model download would freeze indexing with it.

use std::time::Duration;

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
