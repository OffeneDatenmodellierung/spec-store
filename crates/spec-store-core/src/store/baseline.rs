use crate::error::{Result, SpecStoreError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaselineEntry {
    percentage: f64,
    updated_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BaselineFile {
    baselines: HashMap<String, BaselineEntry>,
}

/// In-memory baseline store, persisted to `.spec-store/baselines.json`.
#[derive(Debug)]
pub struct BaselineStore {
    data: BaselineFile,
    path: Option<std::path::PathBuf>,
}

impl BaselineStore {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".spec-store").join("baselines.json");
        let data = if path.exists() {
            let raw =
                std::fs::read_to_string(&path).map_err(|e| SpecStoreError::Store(e.to_string()))?;
            serde_json::from_str(&raw)
                .map_err(|e| SpecStoreError::Store(format!("Baseline parse error: {e}")))?
        } else {
            BaselineFile::default()
        };
        Ok(Self {
            data,
            path: Some(path),
        })
    }

    pub fn new_empty() -> Self {
        Self {
            data: BaselineFile::default(),
            path: None,
        }
    }

    pub fn get(&self, file: &str) -> Option<f64> {
        self.data.baselines.get(file).map(|e| e.percentage)
    }

    pub fn set(&mut self, file: &str, percentage: f64) {
        self.data.baselines.insert(
            file.to_string(),
            BaselineEntry {
                percentage,
                updated_at: Utc::now().to_rfc3339(),
            },
        );
    }

    /// Ratchet: only update if the new value is higher than current.
    pub fn ratchet(&mut self, file: &str, percentage: f64) -> bool {
        let current = self.get(file).unwrap_or(0.0);
        if percentage > current {
            self.set(file, percentage);
            return true;
        }
        false
    }

    /// Update all entries from a map of file → coverage percentage.
    pub fn update_from_map(&mut self, coverage: &HashMap<String, f64>) {
        for (file, pct) in coverage {
            self.ratchet(file, *pct);
        }
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let raw = serde_json::to_string_pretty(&self.data)
            .map_err(|e| SpecStoreError::Store(e.to_string()))?;
        std::fs::write(path, raw).map_err(|e| SpecStoreError::Store(e.to_string()))
    }

    pub fn all_entries(&self) -> impl Iterator<Item = (&str, f64)> {
        self.data
            .baselines
            .iter()
            .map(|(k, v)| (k.as_str(), v.percentage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_empty_has_no_entries() {
        let store = BaselineStore::new_empty();
        assert_eq!(store.get("src/foo.rs"), None);
    }

    #[test]
    fn set_and_get_roundtrip() {
        let mut store = BaselineStore::new_empty();
        store.set("src/foo.rs", 88.5);
        assert_eq!(store.get("src/foo.rs"), Some(88.5));
    }

    #[test]
    fn ratchet_updates_when_higher() {
        let mut store = BaselineStore::new_empty();
        store.set("src/foo.rs", 80.0);
        let updated = store.ratchet("src/foo.rs", 90.0);
        assert!(updated);
        assert_eq!(store.get("src/foo.rs"), Some(90.0));
    }

    #[test]
    fn ratchet_does_not_regress() {
        let mut store = BaselineStore::new_empty();
        store.set("src/foo.rs", 90.0);
        let updated = store.ratchet("src/foo.rs", 80.0);
        assert!(!updated);
        assert_eq!(store.get("src/foo.rs"), Some(90.0));
    }

    #[test]
    fn save_and_reload() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".spec-store")).unwrap();

        let mut store = BaselineStore::load(dir.path()).unwrap();
        store.set("src/foo.rs", 87.5);
        store.save().unwrap();

        let reloaded = BaselineStore::load(dir.path()).unwrap();
        assert_eq!(reloaded.get("src/foo.rs"), Some(87.5));
    }

    #[test]
    fn update_from_map_applies_ratchet() {
        let mut store = BaselineStore::new_empty();
        store.set("src/a.rs", 80.0);
        let map: HashMap<String, f64> = [
            ("src/a.rs".into(), 75.0), // lower — should not update
            ("src/b.rs".into(), 90.0), // new — should add
        ]
        .into();
        store.update_from_map(&map);
        assert_eq!(store.get("src/a.rs"), Some(80.0));
        assert_eq!(store.get("src/b.rs"), Some(90.0));
    }

    #[test]
    fn all_entries_returns_all() {
        let mut store = BaselineStore::new_empty();
        store.set("a.rs", 85.0);
        store.set("b.rs", 90.0);
        let count = store.all_entries().count();
        assert_eq!(count, 2);
    }
}
