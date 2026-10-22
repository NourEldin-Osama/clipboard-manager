//! Embedding backends and vector scoring.

pub mod server;
pub mod siglip;
mod siglip_model;

use anyhow::Result;
use image::DynamicImage;

/// A single dense vector, or one vector per token/patch (ColBERT / ColPali style).
/// Vectors are always L2-normalised.
#[derive(Clone, Debug, PartialEq)]
pub enum Embedding {
    Dense(Vec<f32>),
    Multi(Vec<Vec<f32>>),
}

impl Embedding {
    pub fn dense(mut v: Vec<f32>, truncate: usize) -> Self {
        if truncate > 0 && v.len() > truncate {
            v.truncate(truncate);
        }
        normalize(&mut v);
        Embedding::Dense(v)
    }

    pub fn multi(vs: Vec<Vec<f32>>, truncate: usize) -> Self {
        Embedding::Multi(
            vs.into_iter()
                .map(|v| match Embedding::dense(v, truncate) {
                    Embedding::Dense(v) => v,
                    Embedding::Multi(_) => unreachable!(),
                })
                .collect(),
        )
    }

    /// Similarity in roughly [-1, 1]. Multi-vector pairs use late-interaction
    /// MaxSim averaged over query vectors so scores stay comparable.
    pub fn score(&self, doc: &Embedding) -> f32 {
        match (self, doc) {
            (Embedding::Dense(q), Embedding::Dense(d)) => dot(q, d),
            (Embedding::Dense(q), Embedding::Multi(ds)) => {
                ds.iter().map(|d| dot(q, d)).fold(f32::MIN, f32::max)
            }
            (Embedding::Multi(qs), Embedding::Dense(d)) => {
                qs.iter().map(|q| dot(q, d)).sum::<f32>() / qs.len().max(1) as f32
            }
            (Embedding::Multi(qs), Embedding::Multi(ds)) => {
                let total: f32 = qs
                    .iter()
                    .map(|q| ds.iter().map(|d| dot(q, d)).fold(f32::MIN, f32::max))
                    .sum();
                total / qs.len().max(1) as f32
            }
        }
    }

    /// Layout: tag u8 (0 dense, 1 multi), u32 rows, u32 dims, then f32 LE.
    pub fn to_bytes(&self) -> Vec<u8> {
        let (tag, rows): (u8, Vec<&Vec<f32>>) = match self {
            Embedding::Dense(v) => (0, vec![v]),
            Embedding::Multi(vs) => (1, vs.iter().collect()),
        };
        let dims = rows.first().map(|r| r.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(9 + rows.len() * dims * 4);
        out.push(tag);
        out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
        out.extend_from_slice(&(dims as u32).to_le_bytes());
        for r in rows {
            for x in r {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 9 {
            return None;
        }
        let rows = u32::from_le_bytes(b[1..5].try_into().ok()?) as usize;
        let dims = u32::from_le_bytes(b[5..9].try_into().ok()?) as usize;
        let body = &b[9..];
        if body.len() != rows * dims * 4 {
            return None;
        }
        let floats: Vec<f32> = body
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let mut vs: Vec<Vec<f32>> = floats.chunks(dims.max(1)).map(|c| c.to_vec()).collect();
        match b[0] {
            0 => Some(Embedding::Dense(vs.pop().unwrap_or_default())),
            1 => Some(Embedding::Multi(vs)),
            _ => None,
        }
    }
}

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Query,
    Document,
}

pub trait Embedder: Send {
    /// Stable key embeddings are stored under; changes when the model or
    /// anything affecting its vectors (prefixes, truncation) changes.
    fn key(&self) -> String;
    fn supports_images(&self) -> bool;
    fn embed_texts(&mut self, texts: &[String], role: TextRole) -> Result<Vec<Embedding>>;
    fn embed_image(&mut self, img: &DynamicImage) -> Result<Embedding>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_roundtrip() {
        let d = Embedding::dense(vec![3.0, 4.0], 0);
        assert_eq!(Embedding::from_bytes(&d.to_bytes()), Some(d));
        let m = Embedding::multi(vec![vec![1.0, 0.0], vec![0.0, 2.0]], 0);
        assert_eq!(Embedding::from_bytes(&m.to_bytes()), Some(m));
    }

    #[test]
    fn maxsim() {
        let q = Embedding::multi(vec![vec![1.0, 0.0], vec![0.0, 1.0]], 0);
        let d = Embedding::multi(vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![-1.0, 0.0]], 0);
        assert!((q.score(&d) - 1.0).abs() < 1e-6);
        let t = Embedding::dense(vec![1.0, 1.0, 5.0], 2);
        assert_eq!(t, Embedding::dense(vec![1.0, 1.0], 0));
    }
}
