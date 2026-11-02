//! Downloads that continue where they stopped, for a weak or metered line.
//!
//! What arrived stays in a `.part` file next to the target, and a `.part-tag`
//! file keeps the address and the server's tag for the file (its ETag, or its
//! date). The next try asks only for the rest, and only if the server's file
//! still has that tag; if it changed, the server sends all of it again. After
//! a dropped connection the download waits, then tries again by itself.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;

/// How a download is going, for a progress bar. Several files downloaded one
/// after another can share one.
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
    /// When the next try starts after a dropped connection, in milliseconds
    /// since 1970; 0 while there is no wait.
    retry_at: AtomicU64,
}

impl Progress {
    /// The seconds left before the next try, while waiting after a dropped
    /// connection.
    pub fn retry_in(&self) -> Option<u64> {
        let at = self.retry_at.load(Ordering::Relaxed);
        let now = now_ms();
        (at > now).then(|| (at - now).div_ceil(1000))
    }
}

/// The waits before each new try after a dropped connection. After the last
/// one the error shows, and the partial file stays for the next time.
const WAITS: [Duration; 5] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(120),
    Duration::from_secs(300),
];

/// A file to download.
pub struct Download<'a> {
    pub url: &'a str,
    pub dest: &'a Path,
    /// The whole file's size when it is known in advance, as an update's
    /// signed size: a server that sends more is refused.
    pub size: Option<u64>,
    /// Refuse plain http, also after a redirect.
    pub https_only: bool,
    /// How long one try may take to receive the file.
    pub max_body: Duration,
}

/// Why a try failed.
enum Failure {
    /// The connection dropped: worth another try after a wait.
    Retry(anyhow::Error),
    /// The partial file was no use and is gone: try again at once.
    Again,
    /// Another try won't help.
    Stop(anyhow::Error),
}

impl Download<'_> {
    /// Downloads the file to `dest`, continuing what an earlier try left.
    pub fn run(&self, p: &Progress) -> anyhow::Result<()> {
        self.run_with_waits(p, &WAITS)
    }

    fn run_with_waits(&self, p: &Progress, waits: &[Duration]) -> anyhow::Result<()> {
        let mut counted = Counted::default();
        let mut waits = waits.iter();
        loop {
            match self.try_once(p, &mut counted) {
                Ok(()) => return Ok(()),
                Err(Failure::Again) => {}
                Err(Failure::Stop(e)) => return Err(e),
                Err(Failure::Retry(e)) => {
                    let Some(wait) = waits.next() else {
                        return Err(e);
                    };
                    log::warn!("{e:#}; trying again in {} s", wait.as_secs());
                    wait_unless_cancelled(p, *wait)?;
                }
            }
        }
    }

    fn part(&self) -> PathBuf {
        self.dest.with_extension("part")
    }

    fn tag_file(&self) -> PathBuf {
        self.dest.with_extension("part-tag")
    }

    fn forget_part(&self) {
        let _ = std::fs::remove_file(self.part());
        let _ = std::fs::remove_file(self.tag_file());
    }

    /// One try: asks for what the `.part` file lacks and adds it.
    fn try_once(&self, p: &Progress, counted: &mut Counted) -> Result<(), Failure> {
        if p.cancel.load(Ordering::Relaxed) {
            return Err(Failure::Stop(anyhow!("download cancelled")));
        }
        let (part, tag_file) = (self.part(), self.tag_file());
        if let Some(dir) = self.dest.parent() {
            std::fs::create_dir_all(dir).map_err(stop)?;
        }
        // What an earlier try left, if it was for this address and has a tag
        // to check that the server's file is still the same.
        let tag = read_tag(&tag_file, self.url);
        let have = match &tag {
            Some(_) => std::fs::metadata(&part).map_or(0, |m| m.len()),
            None => 0,
        };
        let mut request = crate::net::download(self.max_body).get(self.url);
        if let Some(tag) = tag.as_deref().filter(|_| have > 0) {
            request = request
                .header("Range", format!("bytes={have}-"))
                .header("If-Range", tag);
        }
        let mut resp = request
            .config()
            .https_only(self.https_only)
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| connection_failure(e, self.url))?;
        let status = resp.status().as_u16();
        let range = content_range(&resp);
        let resumed = have > 0;
        let start = match status {
            200 => 0,
            // GitHub and Hugging Face send the range even when the tag in
            // If-Range is an old one, so the tag is checked here too.
            206 if resumed
                && range.is_some_and(|(from, _)| from == have)
                && response_tag(&resp) == tag =>
            {
                have
            }
            // Another range than the one asked for, a file that changed, or
            // nothing left to send: the partial file is no use.
            206 | 416 if resumed => {
                self.forget_part();
                return Err(Failure::Again);
            }
            408 | 429 | 500..=599 => {
                return Err(Failure::Retry(anyhow!(
                    "downloading {}: the server answered {status}",
                    self.url
                )))
            }
            _ => {
                return Err(Failure::Stop(anyhow!(
                    "downloading {}: the server answered {status}",
                    self.url
                )))
            }
        };
        // The whole file's size.
        let total = if start > 0 {
            range.and_then(|(_, total)| total)
        } else {
            header(&resp, "content-length").and_then(|v| v.trim().parse().ok())
        };
        if let (Some(size), Some(total)) = (self.size, total) {
            if size != total {
                self.forget_part();
                return Err(Failure::Stop(anyhow!(
                    "downloading {}: the file is {total} bytes, not the {size} expected",
                    self.url
                )));
            }
        }
        let total = self.size.or(total);
        // The server's tag, so a later try can continue this one.
        match response_tag(&resp) {
            Some(tag) => std::fs::write(&tag_file, format!("{}\n{tag}", self.url)).map_err(stop)?,
            None => {
                let _ = std::fs::remove_file(&tag_file);
            }
        }
        let mut out = if start == 0 {
            std::fs::File::create(&part)
        } else {
            std::fs::OpenOptions::new().append(true).open(&part)
        }
        .map_err(stop)?;
        counted.reset(p, start, total);
        // ureq refuses every read once the limit is used up, also the last
        // one, which only finds the end, so the limit is one byte past what
        // is left.
        let limit = total.map_or(u64::MAX, |t| t - start + 1);
        let mut reader = resp.body_mut().with_config().limit(limit).reader();
        let mut got = start;
        let mut buf = vec![0u8; 1 << 16];
        let ended = loop {
            if p.cancel.load(Ordering::Relaxed) {
                return Err(Failure::Stop(anyhow!("download cancelled")));
            }
            match reader.read(&mut buf) {
                Ok(0) => break Ok(()),
                Ok(n) => {
                    out.write_all(&buf[..n]).map_err(stop)?;
                    got += n as u64;
                    counted.add(p, n as u64);
                }
                Err(e) => break Err(e),
            }
        };
        out.sync_all().map_err(stop)?;
        drop(out);
        if let Some(t) = total.filter(|t| got > *t) {
            self.forget_part();
            return Err(Failure::Stop(anyhow!(
                "downloading {}: the server sent more than the {t} bytes expected",
                self.url
            )));
        }
        if let Err(e) = ended {
            return Err(Failure::Retry(
                anyhow::Error::from(e).context(format!("downloading {}", self.url)),
            ));
        }
        if let Some(t) = total.filter(|t| got < *t) {
            return Err(Failure::Retry(anyhow!(
                "the download of {} stopped at {got} of {t} bytes",
                self.url
            )));
        }
        std::fs::rename(&part, self.dest).map_err(stop)?;
        let _ = std::fs::remove_file(&tag_file);
        Ok(())
    }
}

/// This file's share of the progress, so that bytes are counted once when a
/// try starts over or picks up again.
#[derive(Default)]
struct Counted {
    done: u64,
    total: u64,
}

impl Counted {
    fn reset(&mut self, p: &Progress, done: u64, total: Option<u64>) {
        let total = total.unwrap_or(0);
        p.total.fetch_add(total, Ordering::Relaxed);
        p.total.fetch_sub(self.total, Ordering::Relaxed);
        p.done.fetch_add(done, Ordering::Relaxed);
        p.done.fetch_sub(self.done, Ordering::Relaxed);
        (self.done, self.total) = (done, total);
    }

    fn add(&mut self, p: &Progress, n: u64) {
        self.done += n;
        p.done.fetch_add(n, Ordering::Relaxed);
    }
}

fn stop(e: std::io::Error) -> Failure {
    Failure::Stop(e.into())
}

/// A request that got no answer: worth another try, unless the address
/// itself is refused.
fn connection_failure(e: ureq::Error, url: &str) -> Failure {
    let refused = matches!(e, ureq::Error::RequireHttpsOnly(_) | ureq::Error::BadUri(_));
    let e = anyhow::Error::from(e).context(format!("downloading {url}"));
    if refused {
        Failure::Stop(e)
    } else {
        Failure::Retry(e)
    }
}

fn header<'a>(resp: &'a ureq::http::Response<ureq::Body>, name: &str) -> Option<&'a str> {
    resp.headers().get(name)?.to_str().ok()
}

/// `Content-Range: bytes <from>-<to>/<total>`, as (from, total).
fn content_range(resp: &ureq::http::Response<ureq::Body>) -> Option<(u64, Option<u64>)> {
    let value = header(resp, "content-range")?
        .trim()
        .strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let from = range.split_once('-')?.0.trim().parse().ok()?;
    Some((from, total.trim().parse().ok()))
}

/// What tells this version of the server's file from another: its ETag, or
/// its date when the ETag is a weak one, which a range can't rely on.
fn response_tag(resp: &ureq::http::Response<ureq::Body>) -> Option<String> {
    header(resp, "etag")
        .filter(|tag| !tag.starts_with("W/"))
        .or_else(|| header(resp, "last-modified"))
        .map(str::to_string)
}

/// The tag an earlier try kept, if it was for `url`.
fn read_tag(file: &Path, url: &str) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let (kept_url, tag) = text.split_once('\n')?;
    let tag = tag.trim();
    (kept_url == url && !tag.is_empty()).then(|| tag.to_string())
}

/// Waits before the next try, with the time left in `p` for the progress
/// bar, unless the download is cancelled.
fn wait_unless_cancelled(p: &Progress, wait: Duration) -> anyhow::Result<()> {
    let until = now_ms() + wait.as_millis() as u64;
    p.retry_at.store(until, Ordering::Relaxed);
    let result = loop {
        if p.cancel.load(Ordering::Relaxed) {
            break Err(anyhow!("download cancelled"));
        }
        let now = now_ms();
        if now >= until {
            break Ok(());
        }
        std::thread::sleep(Duration::from_millis((until - now).min(200)));
    };
    p.retry_at.store(0, Ordering::Relaxed);
    result
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// A test server: answers one connection after another with the given
    /// raw responses, and keeps every request it got.
    fn serve(responses: Vec<Vec<u8>>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file.bin", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut request = Vec::new();
                let mut buf = [0u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => request.extend_from_slice(&buf[..n]),
                    }
                }
                seen.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request).to_lowercase());
                let _ = stream.write_all(&response);
            }
        });
        (url, requests)
    }

    /// A response with `body`; `length` is the Content-Length it claims, if
    /// any, and `extra` more header lines.
    fn response(status: &str, length: Option<usize>, extra: &str, body: &[u8]) -> Vec<u8> {
        let length = length.map_or(String::new(), |n| format!("Content-Length: {n}\r\n"));
        let mut out =
            format!("HTTP/1.1 {status}\r\n{length}{extra}Connection: close\r\n\r\n").into_bytes();
        out.extend_from_slice(body);
        out
    }

    fn content(len: usize, seed: u8) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8 ^ seed).collect()
    }

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cbm-download-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn download<'a>(url: &'a str, dest: &'a Path, size: Option<u64>) -> Download<'a> {
        Download {
            url,
            dest,
            size,
            https_only: false,
            max_body: Duration::from_secs(30),
        }
    }

    const NO_WAIT: [Duration; 1] = [Duration::ZERO];

    #[test]
    fn a_dropped_download_continues_where_it_stopped() {
        let file = content(200_000, 0);
        let (url, requests) = serve(vec![
            // The connection drops after 120 000 of the 200 000 bytes.
            response(
                "200 OK",
                Some(200_000),
                "ETag: \"v1\"\r\n",
                &file[..120_000],
            ),
            response(
                "206 Partial Content",
                Some(80_000),
                "Content-Range: bytes 120000-199999/200000\r\nETag: \"v1\"\r\n",
                &file[120_000..],
            ),
        ]);
        let dir = folder("resume");
        let dest = dir.join("file.bin");
        let p = Progress::default();
        download(&url, &dest, None)
            .run_with_waits(&p, &NO_WAIT)
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), file);
        let requests = requests.lock().unwrap();
        assert!(
            requests[1].contains("range: bytes=120000-"),
            "{}",
            requests[1]
        );
        assert!(requests[1].contains("if-range: \"v1\""), "{}", requests[1]);
        assert_eq!(p.done.load(Ordering::Relaxed), 200_000, "counted once");
        assert_eq!(p.total.load(Ordering::Relaxed), 200_000);
        assert!(!dest.with_extension("part").exists());
        assert!(!dest.with_extension("part-tag").exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_file_that_changed_on_the_server_starts_over() {
        let dir = folder("changed");
        let dest = dir.join("file.bin");
        let new = content(150_000, 7);
        let (url, requests) = serve(vec![response(
            "200 OK",
            Some(new.len()),
            "ETag: \"v2\"\r\n",
            &new,
        )]);
        // Half of the old version, from an earlier try.
        std::fs::write(dest.with_extension("part"), &content(150_000, 0)[..75_000]).unwrap();
        std::fs::write(dest.with_extension("part-tag"), format!("{url}\n\"v1\"")).unwrap();
        let p = Progress::default();
        download(&url, &dest, None)
            .run_with_waits(&p, &NO_WAIT)
            .unwrap();
        assert!(requests.lock().unwrap()[0].contains("if-range: \"v1\""));
        assert_eq!(std::fs::read(&dest).unwrap(), new);
        assert_eq!(p.done.load(Ordering::Relaxed), 150_000);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_changed_file_starts_over_even_when_the_server_sends_the_range() {
        let dir = folder("ignored-tag");
        let dest = dir.join("file.bin");
        let new = content(150_000, 9);
        let (url, requests) = serve(vec![
            // The range of the new version, as if the old tag had matched.
            response(
                "206 Partial Content",
                Some(75_000),
                "Content-Range: bytes 75000-149999/150000\r\nETag: \"v2\"\r\n",
                &new[75_000..],
            ),
            response("200 OK", Some(new.len()), "ETag: \"v2\"\r\n", &new),
        ]);
        std::fs::write(dest.with_extension("part"), &content(150_000, 0)[..75_000]).unwrap();
        std::fs::write(dest.with_extension("part-tag"), format!("{url}\n\"v1\"")).unwrap();
        let p = Progress::default();
        download(&url, &dest, None)
            .run_with_waits(&p, &NO_WAIT)
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), new, "no mix of two versions");
        let requests = requests.lock().unwrap();
        assert!(
            !requests[1].contains("range:"),
            "the second try asks for all"
        );
        assert_eq!(p.done.load(Ordering::Relaxed), 150_000);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_expected_size_is_read_whole_and_more_is_refused() {
        let dir = folder("size");
        let file = content(100_000, 3);
        let (url, _) = serve(vec![response("200 OK", Some(100_000), "", &file)]);
        let dest = dir.join("exact.bin");
        download(&url, &dest, Some(100_000))
            .run_with_waits(&Progress::default(), &NO_WAIT)
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), file);
        // A server that sends a byte more, without saying how much.
        let (url, requests) = serve(vec![response("200 OK", None, "", &content(100_001, 3))]);
        let dest = dir.join("longer.bin");
        assert!(download(&url, &dest, Some(100_000))
            .run_with_waits(&Progress::default(), &NO_WAIT)
            .is_err());
        assert!(!dest.exists() && !dest.with_extension("part").exists());
        assert_eq!(requests.lock().unwrap().len(), 1, "not tried again");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn after_the_last_wait_the_error_shows_and_the_part_stays() {
        let file = content(200_000, 5);
        let cut = response(
            "200 OK",
            Some(200_000),
            "ETag: \"v1\"\r\n",
            &file[..120_000],
        );
        let (url, requests) = serve(vec![cut.clone(), cut]);
        let dir = folder("give-up");
        let dest = dir.join("file.bin");
        assert!(download(&url, &dest, None)
            .run_with_waits(&Progress::default(), &NO_WAIT)
            .is_err());
        assert_eq!(requests.lock().unwrap().len(), 2, "one try and one more");
        assert!(!dest.exists());
        assert_eq!(
            std::fs::metadata(dest.with_extension("part"))
                .unwrap()
                .len(),
            120_000,
            "kept for the next time"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_missing_file_is_not_tried_again() {
        let (url, requests) = serve(vec![response("404 Not Found", Some(0), "", b"")]);
        let dir = folder("missing");
        let dest = dir.join("file.bin");
        let error = download(&url, &dest, None)
            .run_with_waits(&Progress::default(), &[Duration::ZERO; 3])
            .unwrap_err();
        assert!(format!("{error:#}").contains("404"), "{error:#}");
        assert_eq!(requests.lock().unwrap().len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_wait_before_a_new_try_shows_and_ends() {
        let p = Progress::default();
        assert_eq!(p.retry_in(), None);
        let until = now_ms() + 30_000;
        p.retry_at.store(until, Ordering::Relaxed);
        assert!(matches!(p.retry_in(), Some(29 | 30)));
        p.cancel.store(true, Ordering::Relaxed);
        assert!(wait_unless_cancelled(&p, Duration::from_secs(30)).is_err());
        assert_eq!(p.retry_in(), None, "no wait shows after a cancel");
    }
}
