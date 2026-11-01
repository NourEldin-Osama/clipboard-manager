//! Model catalogue and one-time downloads. After a model is on disk everything
//! runs offline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::download::Progress;
use crate::i18n::t;

pub struct SigLipModel {
    pub repo: &'static str,
    /// Its name in the interface language.
    pub label: fn() -> String,
    pub size_mb: u32,
}

pub const SIGLIP_MODELS: [SigLipModel; 2] = [
    SigLipModel {
        repo: "google/siglip-base-patch16-256-multilingual",
        label: || t!("model-siglip-multilingual"),
        size_mb: 1483,
    },
    SigLipModel {
        repo: "google/siglip-base-patch16-224",
        label: || t!("model-siglip-english"),
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

pub fn download_siglip(models: &Path, repo: &str, p: &Arc<Progress>) -> Result<()> {
    let dir = siglip_dir(models, repo);
    for f in crate::embed::siglip::FILES {
        let url = format!("https://huggingface.co/{repo}/resolve/main/{f}");
        download(&url, &dir.join(f), p, true)?;
    }
    Ok(())
}

pub fn download_ocr(models: &Path, p: &Arc<Progress>) -> Result<()> {
    let dir = ocr_dir(models);
    for (f, url) in OCR_FILES {
        // The background worker downloads these, and searches wait behind it:
        // no waiting to try again, the next start continues the download.
        download(url, &dir.join(f), p, false)?;
    }
    Ok(())
}

/// Downloads `url` to `dest`. A cut-off download stays a `.part` file, never
/// mistaken for a model, and the next try continues it. With `wait`, a
/// dropped connection is also tried again after a wait.
fn download(url: &str, dest: &Path, p: &Progress, wait: bool) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    // Models are up to about 1.5 GB: two hours a try covers a slow line.
    let download = crate::download::Download {
        url,
        dest,
        size: None,
        https_only: false,
        max_body: std::time::Duration::from_secs(2 * 60 * 60),
    };
    if wait {
        download.run(p)
    } else {
        download.run_once(p)
    }
}
