use crate::error::{Result, SpecStoreError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: String,
    pub embedding: Vec<f32>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VectorDb {
    records: HashMap<String, VectorRecord>,
}

/// Local JSON-backed vector store using cosine similarity.
/// Drop-in for Qdrant for smaller codebases.
pub struct LocalVectorStore {
    db: VectorDb,
    path: Option<std::path::PathBuf>,
}

impl LocalVectorStore {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".spec-store").join("vectors.json");
        let db = if path.exists() {
            let raw =
                std::fs::read_to_string(&path).map_err(|e| SpecStoreError::Store(e.to_string()))?;
            serde_json::from_str(&raw)
                .map_err(|e| SpecStoreError::Store(format!("Vector DB parse error: {e}")))?
        } else {
            VectorDb::default()
        };
        Ok(Self {
            db,
            path: Some(path),
        })
    }

    pub fn new_empty() -> Self {
        Self {
            db: VectorDb::default(),
            path: None,
        }
    }

    pub fn upsert(&mut self, record: VectorRecord) {
        self.db.records.insert(record.id.clone(), record);
    }

    pub fn search(&self, query: &[f32], limit: usize) -> Vec<SearchResult> {
        let mut scored: Vec<_> = self
            .db
            .records
            .values()
            .map(|r| {
                let score = cosine_similarity(query, &r.embedding);
                SearchResult {
                    id: r.id.clone(),
                    score,
                    payload: r.payload.clone(),
                }
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
        scored
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let raw = serde_json::to_string_pretty(&self.db)
            .map_err(|e| SpecStoreError::Store(e.to_string()))?;
        std::fs::write(path, raw).map_err(|e| SpecStoreError::Store(e.to_string()))
    }

    pub fn len(&self) -> usize {
        self.db.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.db.records.is_empty()
    }
}

/// Word-bag embedding: normalised term-frequency vector over 64 buckets.
/// No API calls; suitable for deduplication within a codebase.
pub fn embed_text(text: &str) -> Vec<f32> {
    const DIM: usize = 64;
    let mut counts = vec![0u32; DIM];
    for word in text.split_whitespace() {
        let word = word.to_lowercase();
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if word.is_empty() {
            continue;
        }
        let mut hasher = Sha256::new();
        hasher.update(word.as_bytes());
        let hash = hasher.finalize();
        let idx = (hash[0] as usize + hash[1] as usize * 256) % DIM;
        counts[idx] = counts[idx].saturating_add(1);
    }
    let total: f32 = counts
        .iter()
        .map(|&c| (c as f32).powi(2))
        .sum::<f32>()
        .sqrt();
    if total < f32::EPSILON {
        return vec![0.0; DIM];
    }
    counts.iter().map(|&c| c as f32 / total).collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a < f32::EPSILON || mag_b < f32::EPSILON {
        return 0.0;
    }
    (dot / (mag_a * mag_b)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_record(id: &str, text: &str) -> VectorRecord {
        VectorRecord {
            id: id.into(),
            embedding: embed_text(text),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn embed_text_returns_correct_dimension() {
        let v = embed_text("validate stake limit betting");
        assert_eq!(v.len(), 64);
    }

    #[test]
    fn embed_text_is_normalised() {
        let v = embed_text("validate stake limit");
        let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-5, "magnitude was {magnitude}");
    }

    #[test]
    fn identical_texts_have_high_similarity() {
        let a = embed_text("validate bet stake limit");
        let b = embed_text("validate bet stake limit");
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn unrelated_texts_have_lower_similarity() {
        let a = embed_text("validate bet stake limit");
        let b = embed_text("render html template page");
        assert!(cosine_similarity(&a, &b) < 0.9);
    }

    #[test]
    fn upsert_and_search_returns_best_match() {
        let mut store = LocalVectorStore::new_empty();
        store.upsert(make_record("fn_a", "validate stake limit"));
        store.upsert(make_record("fn_b", "render html page"));
        let query = embed_text("validate stake amount");
        let results = store.search(&query, 1);
        assert_eq!(results[0].id, "fn_a");
    }

    #[test]
    fn save_and_reload() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".spec-store")).unwrap();
        let mut store = LocalVectorStore::load(dir.path()).unwrap();
        store.upsert(make_record("fn_a", "validate stake"));
        store.save().unwrap();
        let reloaded = LocalVectorStore::load(dir.path()).unwrap();
        assert_eq!(reloaded.len(), 1);
    }

    #[test]
    fn cosine_similarity_zero_vectors() {
        let a = vec![0.0f32; 64];
        let b = embed_text("some text");
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
