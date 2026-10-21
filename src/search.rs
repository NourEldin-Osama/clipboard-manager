//! Keyword, semantic and hybrid ranking over the in-memory history.

use std::collections::HashMap;

use crate::config::SearchMode;
use crate::embed::Embedding;
use crate::store::{Item, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Text,
    Links,
    Images,
    Files,
    Pinned,
}

impl Filter {
    pub const ALL: [Filter; 6] =
        [Filter::All, Filter::Text, Filter::Links, Filter::Images, Filter::Files, Filter::Pinned];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Text => "Text",
            Filter::Links => "Links",
            Filter::Images => "Images",
            Filter::Files => "Files",
            Filter::Pinned => "Pinned",
        }
    }

    pub fn accepts(self, it: &Item) -> bool {
        match self {
            Filter::All => true,
            Filter::Text => it.kind == Kind::Text,
            Filter::Links => it.kind == Kind::Link,
            Filter::Images => it.kind == Kind::Image,
            Filter::Files => it.kind == Kind::Files,
            Filter::Pinned => it.pinned,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hit {
    /// Index into the item slice that was searched.
    pub idx: usize,
    pub keyword: f32,
    pub semantic: Option<f32>,
    /// What the list is ordered by when meaning is used (higher first); the
    /// relevance bars are drawn from it, so they always read top-down.
    pub relevance: Option<f32>,
}

pub struct Query<'a> {
    pub text: &'a str,
    pub vector: Option<&'a Embedding>,
    pub mode: SearchMode,
    pub filter: Filter,
    pub min_similarity: f32,
    /// Image items carry image vectors (not OCR-text vectors).
    pub image_vectors: bool,
}

/// Fraction of query terms present (case-insensitive), plus a bonus for the
/// exact phrase. 0 means no match.
pub fn keyword_score(haystack_lower: &str, query_lower: &str, terms: &[&str]) -> f32 {
    if terms.is_empty() {
        return 0.0;
    }
    let found = terms.iter().filter(|t| haystack_lower.contains(**t)).count();
    if found == 0 {
        return 0.0;
    }
    let mut s = found as f32 / terms.len() as f32;
    if terms.len() > 1 && haystack_lower.contains(query_lower) {
        s += 1.0;
    }
    s
}

const SEMANTIC_DEPTH: usize = 200;

/// `lower` caches each item's lower-cased searchable text by id; items missing
/// from it are lower-cased on the fly.
pub fn search(items: &[Item], lower: &HashMap<u64, String>, embeddings: &HashMap<u64, Embedding>, q: &Query) -> Vec<Hit> {
    let query_lower = q.text.trim().to_lowercase();
    let candidates: Vec<usize> = (0..items.len()).filter(|&i| q.filter.accepts(&items[i])).collect();

    if query_lower.is_empty() {
        let mut out: Vec<usize> = candidates;
        out.sort_by(|&a, &b| {
            (items[b].pinned, items[b].created, items[b].id).cmp(&(items[a].pinned, items[a].created, items[a].id))
        });
        return out.into_iter().map(|idx| Hit { idx, keyword: 0.0, semantic: None, relevance: None }).collect();
    }

    let terms: Vec<&str> = query_lower.split_whitespace().collect();
    let mut kw: Vec<(usize, f32)> = Vec::new();
    if q.mode != SearchMode::Semantic {
        for &i in &candidates {
            let s = match lower.get(&items[i].id) {
                Some(l) => keyword_score(l, &query_lower, &terms),
                None => keyword_score(&items[i].searchable_text().to_lowercase(), &query_lower, &terms),
            };
            if s > 0.0 {
                kw.push((i, s));
            }
        }
        kw.sort_by(|a, b| b.1.total_cmp(&a.1).then(items[b.0].created.cmp(&items[a.0].created)));
    }

    // Text-to-image similarities are far lower than text-to-text ones (the
    // "modality gap"), so each modality is collected on its own and scaled by
    // its best match before they're merged.
    let mut sem_text: Vec<(usize, f32)> = Vec::new();
    let mut sem_image: Vec<(usize, f32)> = Vec::new();
    if q.mode != SearchMode::Keyword {
        if let Some(v) = q.vector {
            for &i in &candidates {
                if let Some(e) = embeddings.get(&items[i].id) {
                    let s = v.score(e);
                    if s >= q.min_similarity {
                        if q.image_vectors && items[i].kind == Kind::Image {
                            sem_image.push((i, s));
                        } else {
                            sem_text.push((i, s));
                        }
                    }
                }
            }
            for l in [&mut sem_text, &mut sem_image] {
                l.sort_by(|a, b| b.1.total_cmp(&a.1));
                l.truncate(SEMANTIC_DEPTH);
            }
        }
    }

    let have_meaning = !sem_text.is_empty() || !sem_image.is_empty();
    if q.mode == SearchMode::Keyword || !have_meaning {
        // Words only (or no query vector yet): best word match first.
        return kw.iter().map(|&(idx, s)| Hit { idx, keyword: s, semantic: None, relevance: None }).collect();
    }

    // Ordered by similarity. Each modality is scaled by its own best match, so
    // a photo that is the best image match isn't buried under text matches.
    let mut by_idx: HashMap<usize, Hit> = HashMap::new();
    for list in [&sem_text, &sem_image] {
        let best = list.first().map(|&(_, s)| s).unwrap_or(1.0).max(1e-6);
        for &(idx, s) in list {
            by_idx.insert(idx, Hit { idx, keyword: 0.0, semantic: Some(s), relevance: Some(s / best) });
        }
    }
    if q.mode == SearchMode::Hybrid {
        // Containing the words counts too, on the same scale.
        let best_kw = kw.first().map(|&(_, s)| s).unwrap_or(1.0).max(1e-6);
        for &(idx, s) in &kw {
            let h = by_idx.entry(idx).or_insert(Hit { idx, keyword: 0.0, semantic: None, relevance: Some(0.0) });
            h.keyword = s;
            h.relevance = Some(h.relevance.unwrap_or(0.0) + KEYWORD_WEIGHT * s / best_kw);
        }
    }
    let mut v: Vec<Hit> = by_idx.into_values().collect();
    v.sort_by(|a, b| b.relevance.unwrap_or(0.0).total_cmp(&a.relevance.unwrap_or(0.0)).then(items[b.idx].created.cmp(&items[a.idx].created)));
    v
}

/// How much containing all the query words adds to a similarity scaled to 0..1.
const KEYWORD_WEIGHT: f32 = 0.25;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{fnv1a, Origin};

    fn item(id: u64, text: &str, created: i64) -> Item {
        Item {
            id,
            kind: Kind::Text,
            origin: Origin::Clipboard,
            text: text.into(),
            image: None,
            width: 0,
            height: 0,
            ocr: None,
            pinned: false,
            created,
            hash: fnv1a(text.as_bytes()),
            source: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn keyword_and_hybrid() {
        let items = vec![item(1, "invoice for March", 1), item(2, "march madness tickets", 2), item(3, "hello", 3)];
        let mut embs = HashMap::new();
        embs.insert(1, Embedding::dense(vec![1.0, 0.0], 0));
        embs.insert(2, Embedding::dense(vec![0.0, 1.0], 0));
        embs.insert(3, Embedding::dense(vec![0.9, 0.1], 0));
        let qv = Embedding::dense(vec![1.0, 0.0], 0);
        let q = |mode| Query { text: "march invoice", vector: Some(&qv), mode, filter: Filter::All, min_similarity: 0.5, image_vectors: false };

        let k = search(&items, &HashMap::new(), &embs, &q(SearchMode::Keyword));
        assert_eq!(k.iter().map(|h| items[h.idx].id).collect::<Vec<_>>(), vec![1, 2]);

        let s = search(&items, &HashMap::new(), &embs, &q(SearchMode::Semantic));
        assert_eq!(s.iter().map(|h| items[h.idx].id).collect::<Vec<_>>(), vec![1, 3]);

        let h = search(&items, &HashMap::new(), &embs, &q(SearchMode::Hybrid));
        assert_eq!(items[h[0].idx].id, 1);
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn modality_gap_does_not_bury_images() {
        let mut items = vec![item(1, "some link", 1), item(2, "grocery list", 2), item(3, "", 3)];
        items[2].kind = Kind::Image;
        let mut embs = HashMap::new();
        // Text-text scores ~0.8, text-image ~0.2: the image is still the best image.
        embs.insert(1, Embedding::dense(vec![0.8, 0.6, 0.0], 0));
        embs.insert(2, Embedding::dense(vec![0.7, 0.71, 0.0], 0));
        embs.insert(3, Embedding::dense(vec![0.2, 0.0, 0.98], 0));
        let qv = Embedding::dense(vec![1.0, 0.0, 0.0], 0);
        let q = Query { text: "kitten", vector: Some(&qv), mode: SearchMode::Semantic, filter: Filter::All, min_similarity: 0.0, image_vectors: true };
        let ids: Vec<u64> = search(&items, &HashMap::new(), &embs, &q).iter().map(|h| items[h.idx].id).collect();
        assert!(ids[..2].contains(&3), "{ids:?}");
    }

    /// Smart results come back best first, so the relevance bars read top-down.
    #[test]
    fn hybrid_is_ordered_by_relevance() {
        let items = vec![item(1, "time format", 1), item(2, "grid view list view time", 2), item(3, "clock", 3), item(4, "hello", 4)];
        let mut embs = HashMap::new();
        embs.insert(1, Embedding::dense(vec![1.0, 0.0], 0));
        embs.insert(2, Embedding::dense(vec![0.3, 0.95], 0));
        embs.insert(3, Embedding::dense(vec![0.95, 0.3], 0));
        embs.insert(4, Embedding::dense(vec![0.6, 0.8], 0));
        let qv = Embedding::dense(vec![1.0, 0.0], 0);
        let q = Query { text: "time", vector: Some(&qv), mode: SearchMode::Hybrid, filter: Filter::All, min_similarity: 0.0, image_vectors: false };
        let hits = search(&items, &HashMap::new(), &embs, &q);
        let rel: Vec<f32> = hits.iter().map(|h| h.relevance.unwrap()).collect();
        assert!(rel.windows(2).all(|w| w[0] >= w[1]), "{rel:?}");
        // A close meaning match outranks a weak one that merely contains the word.
        let ids: Vec<u64> = hits.iter().map(|h| items[h.idx].id).collect();
        assert_eq!(&ids[..2], &[1, 3], "{ids:?}");
    }

    #[test]
    fn empty_query_lists_pinned_then_newest() {
        let mut items = vec![item(1, "a", 1), item(2, "b", 2), item(3, "c", 3)];
        items[0].pinned = true;
        let q = Query { text: " ", vector: None, mode: SearchMode::Hybrid, filter: Filter::All, min_similarity: 0.0, image_vectors: false };
        let ids: Vec<u64> = search(&items, &HashMap::new(), &HashMap::new(), &q).iter().map(|h| items[h.idx].id).collect();
        assert_eq!(ids, vec![1, 3, 2]);
    }
}
