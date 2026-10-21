//! Model catalogue and one-time downloads. After a model is on disk everything
//! runs offline.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{bail, Context, Result};

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
    ("text-detection.rten", "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten"),
    ("text-recognition.rten", "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"),
];

pub fn siglip_dir(models: &Path, repo: &str) -> PathBuf {
    models.join(repo.replace('/', "--"))
}

pub fn ocr_dir(models: &Path) -> PathBuf {
    models.join("ocrs")
}

pub fn siglip_ready(models: &Path, repo: &str) -> bool {
    let d = siglip_dir(models, repo);
    crate::embed::siglip::FILES.iter().all(|f| d.join(f).is_file())
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
    let mut resp = ureq::get(url).call().with_context(|| format!("downloading {url}"))?;
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
    loop {
        if p.cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = std::fs::remove_file(&tmp);
            bail!("download cancelled");
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        p.done.fetch_add(n as u64, Ordering::Relaxed);
    }
    out.flush()?;
    drop(out);
    std::fs::rename(&tmp, dest)?;
    Ok(())
}
