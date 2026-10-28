//! Reading text in images, with the reader chosen in Settings: the one built
//! into the app (ocrs + rten, pure Rust, below), Windows' own, or a vision
//! model on a server.

use std::path::Path;

use anyhow::Result;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

mod server;
#[cfg(windows)]
mod windows_reader;

pub use server::ServerReader;
#[cfg(windows)]
pub use windows_reader::WindowsReader;

/// A text reader, ready to use.
pub enum Reader {
    BuiltIn(Ocr),
    #[cfg(windows)]
    Windows(WindowsReader),
    Server(ServerReader),
}

impl Reader {
    pub fn recognize(&self, img: &image::DynamicImage) -> Result<String> {
        match self {
            Self::BuiltIn(ocr) => ocr.recognize(img),
            #[cfg(windows)]
            Self::Windows(reader) => reader.recognize(img),
            Self::Server(reader) => reader.recognize(img),
        }
    }
}

/// The reader built into the app. It reads Latin letters.
pub struct Ocr {
    engine: OcrEngine,
}

impl Ocr {
    pub fn load(dir: &Path) -> Result<Self> {
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(Model::load_file(dir.join("text-detection.rten"))?),
            recognition_model: Some(Model::load_file(dir.join("text-recognition.rten"))?),
            ..Default::default()
        })?;
        Ok(Self { engine })
    }

    pub fn recognize(&self, img: &image::DynamicImage) -> Result<String> {
        // Very large screenshots are slow and gain little; cap the long side.
        let img = if img.width().max(img.height()) > 2400 {
            img.thumbnail(2400, 2400)
        } else {
            img.clone()
        };
        let rgb = img.into_rgb8();
        let src = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions())?;
        let input = self.engine.prepare_input(src)?;
        let words = self.engine.detect_words(&input)?;
        let lines = self.engine.find_text_lines(&input, &words);
        let texts = self.engine.recognize_text(&input, &lines)?;
        let lines: Vec<String> = texts
            .iter()
            .flatten()
            .map(|l| l.to_string())
            .filter(|l| looks_like_words(l))
            .collect();
        Ok(lines.join("\n"))
    }
}

/// Photos without text still produce fragments like "1e1", "ooG" or "an0Paauas".
/// Keep a line only if most of its tokens look like real words or numbers and
/// at least one is a word of 3+ letters.
fn looks_like_words(line: &str) -> bool {
    let tokens: Vec<&str> = line
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }
    let is_word = |t: &str| {
        let letters: Vec<char> = t.chars().collect();
        if !letters.iter().all(|c| c.is_alphabetic()) {
            return false;
        }
        let rest_lower = letters[1..].iter().all(|c| !c.is_uppercase());
        let all_upper = letters.iter().all(|c| !c.is_lowercase());
        rest_lower || all_upper
    };
    let is_number = |t: &str| {
        t.chars()
            .all(|c| c.is_ascii_digit() || ",.:/-%".contains(c))
    };
    let good = tokens.iter().filter(|t| is_word(t) || is_number(t)).count();
    let has_word = tokens.iter().any(|t| is_word(t) && t.chars().count() >= 3);
    has_word && good * 10 >= tokens.len() * 6
}

#[cfg(test)]
mod tests {
    #[test]
    fn junk_filter() {
        assert!(!super::looks_like_words("1e1"));
        assert!(!super::looks_like_words("ooG"));
        assert!(!super::looks_like_words("an0Paauas"));
        assert!(super::looks_like_words("Offline clipboard manager"));
        assert!(super::looks_like_words("QX7J2L Riyadh to Jeddah"));
        assert!(super::looks_like_words("Invoice total: 1,250 SAR"));
    }

    /// OCR_DIR=<dir with the .rten models> OCR_IMAGE=<image with text> cargo test -- --ignored ocr
    #[test]
    #[ignore]
    fn ocr_reads_text() {
        let ocr =
            super::Ocr::load(std::path::Path::new(&std::env::var("OCR_DIR").unwrap())).unwrap();
        let text = ocr
            .recognize(&image::open(std::env::var("OCR_IMAGE").unwrap()).unwrap())
            .unwrap();
        println!("{text}");
        assert!(text.to_lowercase().contains("clipboard"));
    }
}
