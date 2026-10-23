//! Model catalogue and one-time downloads. After a model is on disk everything
//! runs offline.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};

pub struct SigLipModel {
    pub repo: &'static str,
    pub label: &'static str,
    pub size_mb: u32,
}

pub const SIGLIP_MODELS: [SigLipModel; 2] = [
    SigLipModel {
        repo: "google/siglip-base-patch16-256-multilingual",
        label: "SigLIP base, multilingual (text + images)",
        size_mb: 1483,
    },
    SigLipModel {
        repo: "google/siglip-base-patch16-224",
        label: "SigLIP base, English (text + images)",
        size_mb: 813,
    },
];

pub const OCR_FILES: [(&str, &str); 2] = [
    (
        "text-detection.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten",
    ),
    (
        "text-recognition.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten",
    ),
];

pub fn siglip_dir(models: &Path, repo: &str) -> PathBuf {
    models.join(repo.replace('/', "--"))
}

pub fn ocr_dir(models: &Path) -> PathBuf {
    models.join("ocrs")
}

pub fn siglip_ready(models: &Path, repo: &str) -> bool {
    let d = siglip_dir(models, repo);
    crate::embed::siglip::FILES
        .iter()
        .all(|f| d.join(f).is_file())
}

pub fn ocr_ready(models: &Path) -> bool {
    let d = ocr_dir(models);
    OCR_FILES.iter().all(|(f, _)| d.join(f).is_file())
}

#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
}

pub fn download_siglip(models: &Path, repo: &str, p: &Arc<Progress>) -> Result<()> {
    let dir = siglip_dir(models, repo);
    for f in crate::embed::siglip::FILES {
        let url = format!("https://huggingface.co/{repo}/resolve/main/{f}");
        download(&url, &dir.join(f), p)?;
    }
    Ok(())
}

pub fn download_ocr(models: &Path, p: &Arc<Progress>) -> Result<()> {
    let dir = ocr_dir(models);
    for (f, url) in OCR_FILES {
        download(url, &dir.join(f), p)?;
    }
    Ok(())
}

/// Streams `url` to `dest` via a temporary file, so a half-finished download
/// is never mistaken for a model.
pub fn download(url: &str, dest: &Path, p: &Arc<Progress>) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(dest.parent().unwrap())?;
    // Models are up to about 1.5 GB: two hours covers a slow line.
    let mut resp = crate::net::download(std::time::Duration::from_secs(2 * 60 * 60))
        .get(url)
        .call()
        .with_context(|| format!("downloading {url}"))?;
    let len: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    p.total.fetch_add(len, Ordering::Relaxed);
    let tmp = dest.with_extension("part");
    let mut out = std::fs::File::create(&tmp)?;
    let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
    let mut buf = vec![0u8; 1 << 16];
    let mut written: u64 = 0;
    let copied = loop {
        if p.cancel.load(Ordering::Relaxed) {
            break Err(anyhow::anyhow!("download cancelled"));
        }
        let n = match reader.read(&mut buf) {
            Ok(n) => n,
            Err(e) => break Err(anyhow::Error::from(e).context(format!("downloading {url}"))),
        };
        if n == 0 {
            // A connection that ends early must not leave a cut-off model
            // that looks finished.
            break if len > 0 && written != len {
                Err(anyhow::anyhow!(
                    "download of {url} stopped at {written} of {len} bytes"
                ))
            } else {
                Ok(())
            };
        }
        if let Err(e) = out.write_all(&buf[..n]) {
            break Err(e.into());
        }
        written += n as u64;
        p.done.fetch_add(n as u64, Ordering::Relaxed);
    };
    drop(out);
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A one-request HTTP server that promises `promised` bytes and sends `body`.
    fn serve_once(promised: usize, body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {promised}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(body);
            }
        });
        format!("http://{addr}/model.bin")
    }

    #[test]
    fn a_cut_off_download_leaves_no_model() {
        let dir = std::env::temp_dir().join(format!("cbm-download-{}", std::process::id()));
        let dest = dir.join("model.bin");
        let progress = Arc::new(Progress::default());

        let cut_off = serve_once(100, b"only ten b");
        assert!(download(&cut_off, &dest, &progress).is_err());
        assert!(!dest.exists(), "a cut-off file must not look like a model");
        assert!(!dest.with_extension("part").exists());

        let complete = serve_once(10, b"only ten b");
        download(&complete, &dest, &progress).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"only ten b");
        std::fs::remove_dir_all(dir).ok();
    }
}
