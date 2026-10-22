//! Built-in SigLIP text/image encoder, run on the CPU with candle (pure Rust).
//!
//! Text and images land in one shared space, so "red sneakers" finds a photo of
//! red sneakers and screenshots find similar screenshots.

use std::path::Path;

use super::siglip_model as siglip;
use anyhow::{anyhow, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use image::DynamicImage;
use tokenizers::Tokenizer;

use super::{Embedder, Embedding, TextRole};

pub const FILES: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

pub struct SigLip {
    key: String,
    model: siglip::Model,
    tokenizer: Tokenizer,
    max_len: usize,
    pad_id: u32,
    image_size: usize,
    device: Device,
}

impl SigLip {
    pub fn load(dir: &Path, key: &str) -> Result<Self> {
        let device = Device::Cpu;
        let cfg: siglip::Config = serde_json::from_str(
            &std::fs::read_to_string(dir.join("config.json")).context("config.json")?,
        )?;
        let tokenizer = Tokenizer::from_file(dir.join("tokenizer.json"))
            .map_err(|e| anyhow!("tokenizer.json: {e}"))?;
        // SigLIP pads with the end-of-sequence token `</s>`, not `<pad>`.
        let pad_id = tokenizer
            .token_to_id("</s>")
            .unwrap_or(cfg.text_config.pad_token_id);
        // SAFETY: the weights file is not modified while mapped.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[dir.join("model.safetensors")],
                DType::F32,
                &device,
            )?
        };
        let model = siglip::Model::new(&cfg, vb)?;
        Ok(Self {
            key: key.to_string(),
            model,
            tokenizer,
            max_len: cfg.text_config.max_position_embeddings,
            pad_id,
            image_size: cfg.vision_config.image_size,
            device,
        })
    }

    fn encode_ids(&self, text: &str) -> Result<Vec<u32>> {
        let enc = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("tokenize: {e}"))?;
        let mut ids = enc.get_ids().to_vec();
        // SigLIP was trained on fixed-length, padded sequences and pools the last position.
        if ids.len() > self.max_len {
            let eos = *ids.last().unwrap();
            ids.truncate(self.max_len);
            ids[self.max_len - 1] = eos;
        }
        ids.resize(self.max_len, self.pad_id);
        Ok(ids)
    }

    pub fn preprocess(&self, img: &DynamicImage) -> Result<Tensor> {
        let s = self.image_size as u32;
        let rgb = img
            .resize_exact(s, s, image::imageops::FilterType::Triangle)
            .to_rgb8();
        let data: Vec<f32> = rgb
            .into_raw()
            .into_iter()
            .map(|p| p as f32 / 127.5 - 1.0)
            .collect();
        Ok(
            Tensor::from_vec(data, (s as usize, s as usize, 3), &self.device)?
                .permute((2, 0, 1))?
                .unsqueeze(0)?,
        )
    }
}

impl Embedder for SigLip {
    fn key(&self) -> String {
        self.key.clone()
    }

    fn supports_images(&self) -> bool {
        true
    }

    fn embed_texts(&mut self, texts: &[String], _role: TextRole) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let mut rows = Vec::with_capacity(texts.len() * self.max_len);
        for t in texts {
            rows.extend(self.encode_ids(t)?);
        }
        let ids = Tensor::from_vec(rows, (texts.len(), self.max_len), &self.device)?;
        let feats = self.model.get_text_features(&ids)?.to_vec2::<f32>()?;
        Ok(feats.into_iter().map(|v| Embedding::dense(v, 0)).collect())
    }

    fn embed_image(&mut self, img: &DynamicImage) -> Result<Embedding> {
        let px = self.preprocess(img)?;
        let feats = self.model.get_image_features(&px)?.to_vec2::<f32>()?;
        Ok(Embedding::dense(
            feats.into_iter().next().unwrap_or_default(),
            0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a downloaded model: SIGLIP_DIR=<dir> SIGLIP_IMAGES=<dir with cat.jpg, car.jpg> cargo test -- --ignored
    #[test]
    #[ignore]
    fn text_finds_matching_image() {
        let dir = std::env::var("SIGLIP_DIR").expect("SIGLIP_DIR");
        let imgs = std::path::PathBuf::from(std::env::var("SIGLIP_IMAGES").expect("SIGLIP_IMAGES"));
        let mut m = SigLip::load(Path::new(&dir), "test").unwrap();
        let cat = m
            .embed_image(&image::open(imgs.join("cat.jpg")).unwrap())
            .unwrap();
        let car = m
            .embed_image(&image::open(imgs.join("car.jpg")).unwrap())
            .unwrap();
        let q = m
            .embed_texts(
                &["a photo of a cat".into(), "a photo of a car".into()],
                TextRole::Query,
            )
            .unwrap();
        let (qc, qr) = (&q[0], &q[1]);
        println!("cat-q: cat {:.3} car {:.3}", qc.score(&cat), qc.score(&car));
        println!("car-q: cat {:.3} car {:.3}", qr.score(&cat), qr.score(&car));
        assert!(qc.score(&cat) > qc.score(&car));
        assert!(qr.score(&car) > qr.score(&cat));
    }
}
