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

/// The text reader's two models. They download as ONNX files; versions
/// before 0.4.2 downloaded `.rten` files, which still load, but their
/// address no longer serves them.
const OCR_MODELS: [&str; 2] = ["text-detection", "text-recognition"];

const OCR_MODELS_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com";

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
    let dir = ocr_dir(models);
    OCR_MODELS
        .iter()
        .all(|name| ocr_model_file(&dir, name).is_some())
}

/// The file of the text reader's model `name` in `dir`: the ONNX download,
/// or the `.rten` file a version before 0.4.2 downloaded.
pub fn ocr_model_file(dir: &Path, name: &str) -> Option<PathBuf> {
    ["onnx", "rten"]
        .into_iter()
        .map(|ext| dir.join(format!("{name}.{ext}")))
        .find(|file| file.is_file())
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
    for name in OCR_MODELS {
        if ocr_model_file(&dir, name).is_some() {
            continue;
        }
        let url = format!("{OCR_MODELS_URL}/{name}.onnx");
        // The background worker downloads these, and searches wait behind it:
        // no waiting to try again, the next start continues the download.
        download(&url, &dir.join(format!("{name}.onnx")), p, false)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_reader_takes_its_onnx_model_or_an_older_rten_one() {
        let dir = std::env::temp_dir().join(format!("cbm-ocr-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ocr = ocr_dir(&dir);
        std::fs::create_dir_all(&ocr).unwrap();
        assert!(!ocr_ready(&dir));
        std::fs::write(ocr.join("text-detection.rten"), "older").unwrap();
        std::fs::write(ocr.join("text-recognition.onnx"), "newer").unwrap();
        assert!(ocr_ready(&dir));
        assert_eq!(
            ocr_model_file(&ocr, "text-detection"),
            Some(ocr.join("text-detection.rten"))
        );
        std::fs::write(ocr.join("text-detection.onnx"), "newer").unwrap();
        assert_eq!(
            ocr_model_file(&ocr, "text-detection"),
            Some(ocr.join("text-detection.onnx"))
        );
        std::fs::remove_dir_all(dir).ok();
    }

    /// Downloads the text reader's models from their real address (12 MB):
    /// OCR_DOWNLOAD_DIR=<empty folder> cargo test -- --ignored text_reader_models
    #[test]
    #[ignore]
    fn the_text_reader_models_download() {
        let models = PathBuf::from(std::env::var("OCR_DOWNLOAD_DIR").unwrap());
        download_ocr(&models, &Arc::default()).unwrap();
        assert!(ocr_ready(&models));
    }
}
