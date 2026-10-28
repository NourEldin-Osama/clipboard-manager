//! Windows' own text reader (Windows.Media.Ocr), in the languages installed
//! in Windows.

use anyhow::{Context, Result};
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

pub struct WindowsReader {
    engine: OcrEngine,
}

impl WindowsReader {
    pub fn new() -> Result<Self> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().context(
            "Windows has no text reader for the languages in its settings \
             (Time & language, Language & region)",
        )?;
        Ok(Self { engine })
    }

    pub fn recognize(&self, img: &image::DynamicImage) -> Result<String> {
        let max = OcrEngine::MaxImageDimension()?;
        let img = if img.width().max(img.height()) > max {
            img.thumbnail(max, max)
        } else {
            img.clone()
        };
        // Windows takes the pixels as blue, green, red, alpha.
        let mut pixels = img.into_rgba8();
        for pixel in pixels.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        let (width, height) = pixels.dimensions();
        let writer = DataWriter::new()?;
        writer.WriteBytes(pixels.as_raw())?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &writer.DetachBuffer()?,
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
        )?;
        let result = self.engine.RecognizeAsync(&bitmap)?.join()?;
        let lines = result
            .Lines()?
            .into_iter()
            .map(|line| line.Text().map(|text| text.to_string()))
            .collect::<windows::core::Result<Vec<String>>>()?;
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    /// OCR_IMAGE=<image with the word "clipboard"> cargo test -- --ignored windows_reader
    #[test]
    #[ignore]
    fn windows_reader_reads_text() {
        let reader = super::WindowsReader::new().unwrap();
        let text = reader
            .recognize(&image::open(std::env::var("OCR_IMAGE").unwrap()).unwrap())
            .unwrap();
        println!("{text}");
        assert!(text.to_lowercase().contains("clipboard"));
    }
}
