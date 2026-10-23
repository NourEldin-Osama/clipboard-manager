//! Keyword, semantic and hybrid ranking over the in-memory history, on a
//! thread of its own (`Searcher`).

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{SearchMode, SortOrder};
use crate::embed::Embedding;
use crate::i18n::t;
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
    pub const ALL: [Filter; 6] = [
        Filter::All,
        Filter::Text,
        Filter::Links,
        Filter::Images,
        Filter::Files,
        Filter::Pinned,
    ];

    /// Its name in the interface language.
    pub fn label(self) -> String {
        match self {
            Filter::All => t!("filter-all"),
            Filter::Text => t!("filter-text"),
            Filter::Links => t!("filter-links"),
            Filter::Images => t!("filter-images"),
            Filter::Files => t!("filter-files"),
            Filter::Pinned => t!("filter-pinned"),
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

/// One search result. By id, not by position: the history can change while
/// the search runs.
#[derive(Clone, Debug)]
pub struct Hit {
    pub id: u64,
    pub semantic: Option<f32>,
    /// What the list is ordered by when meaning is used (higher first); the
    /// relevance bars are drawn from it, so they always read top-down.
    pub relevance: Option<f32>,
}

/// A result while ranking: `idx` points into the items searched.
struct Ranked {
    idx: usize,
    keyword: f32,
    semantic: Option<f32>,
    relevance: Option<f32>,
}

/// What a search reads. Made by `Library::snapshot`; cloning it copies
/// pointers, not texts or vectors.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub items: Vec<Arc<Item>>,
    /// Each item's lower-cased searchable text; items missing here are
    /// lower-cased on the fly.
    pub lower: HashMap<u64, Arc<str>>,
    pub embeddings: HashMap<u64, Arc<Embedding>>,
}

pub struct Query {
    pub text: String,
    pub vector: Option<Arc<Embedding>>,
    pub mode: SearchMode,
    pub filter: Filter,
    /// Only items with this tag. Applied before results are cut to the best
    /// matches, so tagged items further down are not lost.
    pub tag: Option<String>,
    pub min_similarity: f32,
    /// Image items carry image vectors (not OCR-text vectors).
    pub image_vectors: bool,
    pub sort: SortOrder,
    /// Pinned items first, whatever the sort order.
    pub pinned_first: bool,
}

/// Fraction of query terms present (case-insensitive), plus a bonus for the
/// exact phrase. 0 means no match.
pub fn keyword_score(haystack_lower: &str, query_lower: &str, terms: &[&str]) -> f32 {
    if terms.is_empty() {
        return 0.0;
    }
    let found = terms
        .iter()
        .filter(|t| haystack_lower.contains(**t))
        .count();
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

/// The items matching `q`, in the order to show them.
pub fn search(s: &Snapshot, q: &Query) -> Vec<Hit> {
    let mut ranked = rank(s, q);
    order(&s.items, &mut ranked, q.sort, q.pinned_first);
    ranked
        .into_iter()
        .map(|r| Hit {
            id: s.items[r.idx].id,
            semantic: r.semantic,
            relevance: r.relevance,
        })
        .collect()
}

fn rank(s: &Snapshot, q: &Query) -> Vec<Ranked> {
    let (items, lower, embeddings) = (&s.items, &s.lower, &s.embeddings);
    let query_lower = q.text.trim().to_lowercase();
    let candidates: Vec<usize> = (0..items.len())
        .filter(|&i| {
            q.filter.accepts(&items[i]) && q.tag.as_ref().is_none_or(|t| items[i].has_tag(t))
        })
        .collect();

    if query_lower.is_empty() {
        let mut out: Vec<usize> = candidates;
        out.sort_by(|&a, &b| {
            (items[b].pinned, items[b].created, items[b].id).cmp(&(
                items[a].pinned,
                items[a].created,
                items[a].id,
            ))
        });
        return out
            .into_iter()
            .map(|idx| Ranked {
                idx,
                keyword: 0.0,
                semantic: None,
                relevance: None,
            })
            .collect();
    }

    let terms: Vec<&str> = query_lower.split_whitespace().collect();
    let mut kw: Vec<(usize, f32)> = Vec::new();
    if q.mode != SearchMode::Semantic {
        for &i in &candidates {
            let s = match lower.get(&items[i].id) {
                Some(l) => keyword_score(l, &query_lower, &terms),
                None => keyword_score(
                    &items[i].searchable_text().to_lowercase(),
                    &query_lower,
                    &terms,
                ),
            };
            if s > 0.0 {
                kw.push((i, s));
            }
        }
        kw.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then(items[b.0].created.cmp(&items[a.0].created))
        });
    }

    // Text-to-image similarities are far lower than text-to-text ones (the
    // "modality gap"), so each modality is collected on its own and scaled by
    // its best match before they're merged.
    let mut sem_text: Vec<(usize, f32)> = Vec::new();
    let mut sem_image: Vec<(usize, f32)> = Vec::new();
    if q.mode != SearchMode::Keyword {
        if let Some(v) = &q.vector {
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
        return kw
            .iter()
            .map(|&(idx, s)| Ranked {
                idx,
                keyword: s,
                semantic: None,
                relevance: None,
            })
            .collect();
    }

    // Ordered by similarity. Each modality is scaled by its own best match, so
    // a photo that is the best image match isn't buried under text matches.
    let mut by_idx: HashMap<usize, Ranked> = HashMap::new();
    for list in [&sem_text, &sem_image] {
        let best = list.first().map(|&(_, s)| s).unwrap_or(1.0).max(1e-6);
        for &(idx, s) in list {
            by_idx.insert(
                idx,
                Ranked {
                    idx,
                    keyword: 0.0,
                    semantic: Some(s),
                    relevance: Some(s / best),
                },
            );
        }
    }
    if q.mode == SearchMode::Hybrid {
        // Containing the words counts too, on the same scale.
        let best_kw = kw.first().map(|&(_, s)| s).unwrap_or(1.0).max(1e-6);
        for &(idx, s) in &kw {
            let h = by_idx.entry(idx).or_insert(Ranked {
                idx,
                keyword: 0.0,
                semantic: None,
                relevance: Some(0.0),
            });
            h.keyword = s;
            h.relevance = Some(h.relevance.unwrap_or(0.0) + KEYWORD_WEIGHT * s / best_kw);
        }
    }
    let mut v: Vec<Ranked> = by_idx.into_values().collect();
    v.sort_by(|a, b| {
        b.relevance
            .unwrap_or(0.0)
            .total_cmp(&a.relevance.unwrap_or(0.0))
            .then(items[b.idx].created.cmp(&items[a.idx].created))
    });
    v
}

/// How much containing all the query words adds to a similarity scaled to 0..1.
const KEYWORD_WEIGHT: f32 = 0.25;

/// Puts `hits` in the chosen order. `BestMatch` keeps the ranking from
/// `rank`; pinned items move to the top (keeping their order) when asked.
fn order(items: &[Arc<Item>], hits: &mut [Ranked], sort: SortOrder, pinned_first: bool) {
    let newest = |a: &Ranked, b: &Ranked| {
        (items[b.idx].created, items[b.idx].id).cmp(&(items[a.idx].created, items[a.idx].id))
    };
    match sort {
        SortOrder::BestMatch => {}
        SortOrder::Recent => hits.sort_by(newest),
        SortOrder::Oldest => hits.sort_by(|a, b| newest(b, a)),
        SortOrder::AToZ => hits.sort_by_cached_key(|h| {
            (
                items[h.idx].title().to_lowercase(),
                std::cmp::Reverse(items[h.idx].created),
            )
        }),
        SortOrder::MostUsed => hits.sort_by(|a, b| {
            items[b.idx]
                .uses
                .cmp(&items[a.idx].uses)
                .then_with(|| newest(a, b))
        }),
    }
    if pinned_first {
        hits.sort_by_key(|h| !items[h.idx].pinned);
    }
}

/// Runs searches on a thread of its own: with a big history or a slow model
/// a search takes long enough to stall typing. Only the newest request
/// counts; requests still waiting when a newer one comes are skipped.
pub struct Searcher {
    requests: Sender<(u64, Snapshot, Query)>,
    results: Receiver<(u64, Vec<Hit>)>,
}

impl Searcher {
    pub fn spawn(wake: impl Fn() + Send + 'static) -> Self {
        let (requests, inbox) = channel::<(u64, Snapshot, Query)>();
        let (outbox, results) = channel();
        std::thread::Builder::new()
            .name("search".into())
            .spawn(move || {
                while let Ok(mut request) = inbox.recv() {
                    while let Ok(newer) = inbox.try_recv() {
                        request = newer;
                    }
                    let (seq, snapshot, query) = request;
                    match catch_unwind(AssertUnwindSafe(|| search(&snapshot, &query))) {
                        Ok(hits) => {
                            if outbox.send((seq, hits)).is_err() {
                                return;
                            }
                            wake();
                        }
                        Err(_) => log::error!("a search crashed; the list keeps the last results"),
                    }
                }
            })
            .expect("spawn search thread");
        Self { requests, results }
    }

    /// Starts search number `seq`; `results_for(seq)` returns its results.
    pub fn search(&self, seq: u64, snapshot: Snapshot, query: Query) {
        let _ = self.requests.send((seq, snapshot, query));
    }

    /// The results of search `seq` once they are ready. Results of older
    /// searches are dropped: they may show items deleted since.
    pub fn results_for(&self, seq: u64) -> Option<Vec<Hit>> {
        let mut found = None;
        while let Ok((done, hits)) = self.results.try_recv() {
            if done == seq {
                found = Some(hits);
            }
        }
        found
    }

    /// Like `results_for`, but waits up to `timeout` for them.
    pub fn wait_for(&self, seq: u64, timeout: Duration) -> Option<Vec<Hit>> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.results.recv_timeout(left) {
                Ok((done, hits)) if done == seq => return Some(hits),
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    }
}

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
            uses: 0,
        }
    }

    /// No lower-cased texts: search lower-cases them on the fly.
    fn snapshot(items: Vec<Item>, embeddings: Vec<(u64, Embedding)>) -> Snapshot {
        Snapshot {
            items: items.into_iter().map(Arc::new).collect(),
            lower: HashMap::new(),
            embeddings: embeddings
                .into_iter()
                .map(|(id, e)| (id, Arc::new(e)))
                .collect(),
        }
    }

    /// In ranking order (BestMatch), nothing pinned first, no limits.
    fn query(text: &str, vector: Option<Embedding>, mode: SearchMode) -> Query {
        Query {
            text: text.into(),
            vector: vector.map(Arc::new),
            mode,
            filter: Filter::All,
            tag: None,
            min_similarity: 0.0,
            image_vectors: false,
            sort: SortOrder::BestMatch,
            pinned_first: false,
        }
    }

    fn ids(hits: &[Hit]) -> Vec<u64> {
        hits.iter().map(|h| h.id).collect()
    }

    #[test]
    fn keyword_and_hybrid() {
        let s = snapshot(
            vec![
                item(1, "invoice for March", 1),
                item(2, "march madness tickets", 2),
                item(3, "hello", 3),
            ],
            vec![
                (1, Embedding::dense(vec![1.0, 0.0], 0)),
                (2, Embedding::dense(vec![0.0, 1.0], 0)),
                (3, Embedding::dense(vec![0.9, 0.1], 0)),
            ],
        );
        let q = |mode| Query {
            min_similarity: 0.5,
            ..query(
                "march invoice",
                Some(Embedding::dense(vec![1.0, 0.0], 0)),
                mode,
            )
        };
        assert_eq!(ids(&search(&s, &q(SearchMode::Keyword))), vec![1, 2]);
        assert_eq!(ids(&search(&s, &q(SearchMode::Semantic))), vec![1, 3]);
        let h = search(&s, &q(SearchMode::Hybrid));
        assert_eq!(h[0].id, 1);
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn modality_gap_does_not_bury_images() {
        let mut items = vec![
            item(1, "some link", 1),
            item(2, "grocery list", 2),
            item(3, "", 3),
        ];
        items[2].kind = Kind::Image;
        // Text-text scores ~0.8, text-image ~0.2: the image is still the best image.
        let s = snapshot(
            items,
            vec![
                (1, Embedding::dense(vec![0.8, 0.6, 0.0], 0)),
                (2, Embedding::dense(vec![0.7, 0.71, 0.0], 0)),
                (3, Embedding::dense(vec![0.2, 0.0, 0.98], 0)),
            ],
        );
        let q = Query {
            image_vectors: true,
            ..query(
                "kitten",
                Some(Embedding::dense(vec![1.0, 0.0, 0.0], 0)),
                SearchMode::Semantic,
            )
        };
        let found = ids(&search(&s, &q));
        assert!(found[..2].contains(&3), "{found:?}");
    }

    /// Smart results come back best first, so the relevance bars read top-down.
    #[test]
    fn hybrid_is_ordered_by_relevance() {
        let s = snapshot(
            vec![
                item(1, "time format", 1),
                item(2, "grid view list view time", 2),
                item(3, "clock", 3),
                item(4, "hello", 4),
            ],
            vec![
                (1, Embedding::dense(vec![1.0, 0.0], 0)),
                (2, Embedding::dense(vec![0.3, 0.95], 0)),
                (3, Embedding::dense(vec![0.95, 0.3], 0)),
                (4, Embedding::dense(vec![0.6, 0.8], 0)),
            ],
        );
        let q = query(
            "time",
            Some(Embedding::dense(vec![1.0, 0.0], 0)),
            SearchMode::Hybrid,
        );
        let hits = search(&s, &q);
        let rel: Vec<f32> = hits.iter().map(|h| h.relevance.unwrap()).collect();
        assert!(rel.windows(2).all(|w| w[0] >= w[1]), "{rel:?}");
        // A close meaning match outranks a weak one that merely contains the word.
        let found = ids(&hits);
        assert_eq!(&found[..2], &[1, 3], "{found:?}");
    }

    #[test]
    fn sort_orders_and_pinned_switch() {
        let mut items = vec![
            item(1, "banana", 10),
            item(2, "apple", 30),
            item(3, "cherry", 20),
        ];
        items[2].pinned = true;
        items[0].uses = 5;
        let s = snapshot(items, vec![]);
        let order_of = |sort, pinned_first| {
            ids(&search(
                &s,
                &Query {
                    sort,
                    pinned_first,
                    ..query(" ", None, SearchMode::Hybrid)
                },
            ))
        };
        assert_eq!(order_of(SortOrder::Recent, false), vec![2, 3, 1]);
        assert_eq!(order_of(SortOrder::Oldest, false), vec![1, 3, 2]);
        assert_eq!(order_of(SortOrder::AToZ, false), vec![2, 1, 3]);
        assert_eq!(order_of(SortOrder::MostUsed, false), vec![1, 2, 3]);
        assert_eq!(order_of(SortOrder::AToZ, true), vec![3, 2, 1]);
    }

    #[test]
    fn empty_query_lists_pinned_then_newest() {
        let mut items = vec![item(1, "a", 1), item(2, "b", 2), item(3, "c", 3)];
        items[0].pinned = true;
        let s = snapshot(items, vec![]);
        let found = ids(&search(&s, &query(" ", None, SearchMode::Hybrid)));
        assert_eq!(found, vec![1, 3, 2]);
    }

    #[test]
    fn tag_filter_keeps_tagged_items_ranked_low_by_meaning() {
        // 250 items; the only tagged one is the weakest meaning match, below
        // the 200 best that meaning search keeps.
        let mut items: Vec<Item> = (1..=250).map(|id| item(id, "note", id as i64)).collect();
        items[249].tags = vec!["work".into()];
        let embeddings = items
            .iter()
            .map(|it| {
                let s = 1.0 - it.id as f32 / 1000.0;
                (it.id, Embedding::dense(vec![s, (1.0 - s * s).sqrt()], 0))
            })
            .collect();
        let s = snapshot(items, embeddings);
        let q = Query {
            tag: Some("WORK".into()),
            ..query(
                "anything",
                Some(Embedding::dense(vec![1.0, 0.0], 0)),
                SearchMode::Semantic,
            )
        };
        assert_eq!(ids(&search(&s, &q)), vec![250]);
    }

    #[test]
    fn the_searcher_answers_the_newest_request() {
        let searcher = Searcher::spawn(|| {});
        let s = snapshot(vec![item(1, "alpha", 1), item(2, "beta", 2)], vec![]);
        searcher.search(1, s.clone(), query("alpha", None, SearchMode::Keyword));
        searcher.search(2, s, query("beta", None, SearchMode::Keyword));
        let hits = searcher
            .wait_for(2, Duration::from_secs(10))
            .expect("results within 10 s");
        assert_eq!(ids(&hits), vec![2]);
        assert!(
            searcher.results_for(1).is_none(),
            "older results never come back"
        );
    }
}
