use crate::{
    config::ReuseConfig,
    error::{Result, SpecStoreError},
    scanner::FunctionInfo,
    store::{embed_text, LocalVectorStore, VectorRecord},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SimilarityLevel {
    Clear,
    Warning {
        score: f32,
        similar_fn: String,
        similar_file: String,
    },
    Blocked {
        score: f32,
        similar_fn: String,
        similar_file: String,
    },
}

impl SimilarityLevel {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }
}

pub struct ReuseEnforcer<'a> {
    store: &'a LocalVectorStore,
    config: &'a ReuseConfig,
}

impl<'a> ReuseEnforcer<'a> {
    pub fn new(store: &'a LocalVectorStore, config: &'a ReuseConfig) -> Self {
        Self { store, config }
    }

    pub fn check(&self, func: &FunctionInfo, description: &str) -> SimilarityLevel {
        let text = format!("{} {} {}", func.name, description, func.file);
        let embedding = embed_text(&text);
        let results = self.store.search(&embedding, 3);

        for result in results {
            let id = &result.id;
            let score = result.score;
            let similar_fn = result.payload["name"].as_str().unwrap_or(id).to_string();
            let similar_file = result.payload["file"].as_str().unwrap_or("").to_string();

            // Skip self-match
            if similar_fn == func.name && similar_file == func.file {
                continue;
            }

            if score >= self.config.similarity_block {
                return SimilarityLevel::Blocked {
                    score,
                    similar_fn,
                    similar_file,
                };
            }
            if score >= self.config.similarity_warn {
                return SimilarityLevel::Warning {
                    score,
                    similar_fn,
                    similar_file,
                };
            }
        }
        SimilarityLevel::Clear
    }

    pub fn check_all(&self, functions: &[FunctionInfo]) -> Vec<(FunctionInfo, SimilarityLevel)> {
        functions
            .iter()
            .map(|f| {
                let level = self.check(f, &f.name);
                (f.clone(), level)
            })
            .collect()
    }
}

pub fn register_function(store: &mut LocalVectorStore, func: &FunctionInfo, description: &str) {
    let text = format!("{} {} {}", func.name, description, func.file);
    let record = VectorRecord {
        id: format!("{}:{}", func.file, func.name),
        embedding: embed_text(&text),
        payload: serde_json::json!({
            "name": func.name,
            "file": func.file,
            "line": func.line,
            "description": description,
            "is_test": func.is_test,
        }),
    };
    store.upsert(record);
}

pub fn assert_no_blocks(
    results: &[(FunctionInfo, SimilarityLevel)],
    block_threshold: f32,
) -> Result<()> {
    let blocked: Vec<_> = results.iter().filter(|(_, l)| l.is_blocked()).collect();
    if blocked.is_empty() {
        return Ok(());
    }
    let detail = blocked
        .iter()
        .map(|(f, l)| match l {
            SimilarityLevel::Blocked {
                score, similar_fn, ..
            } => format!("  {} ({:.2} similar to {})", f.name, score, similar_fn),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    Err(SpecStoreError::Store(format!(
        "{} function(s) blocked by reuse gate (≥{:.0}% similarity):\n{detail}",
        blocked.len(),
        block_threshold * 100.0
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::ReuseConfig, scanner::FunctionInfo, store::LocalVectorStore};

    fn config() -> ReuseConfig {
        ReuseConfig {
            similarity_warn: 0.85,
            similarity_block: 0.95,
        }
    }

    fn make_fn(name: &str, file: &str) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            file: file.into(),
            line: 1,
            line_count: 5,
            param_count: 0,
            complexity: 1,
            is_test: false,
        }
    }

    fn populated_store() -> LocalVectorStore {
        let mut store = LocalVectorStore::new_empty();
        let existing = make_fn("validate_stake_limit", "src/risk.rs");
        register_function(
            &mut store,
            &existing,
            "Validates a betting stake against the configured limit",
        );
        store
    }

    #[test]
    fn identical_function_is_blocked() {
        let store = populated_store();
        let cfg = config();
        let enforcer = ReuseEnforcer::new(&store, &cfg);
        let func = make_fn("validate_stake_limit", "src/risk2.rs");
        let level = enforcer.check(
            &func,
            "Validates a betting stake against the configured limit",
        );
        assert!(
            level.is_blocked() || level.is_warning(),
            "Expected at least a warning, got: {level:?}"
        );
    }

    #[test]
    fn unrelated_function_is_clear() {
        let store = populated_store();
        let cfg = config();
        let enforcer = ReuseEnforcer::new(&store, &cfg);
        let func = make_fn("render_html_template", "src/ui.rs");
        let level = enforcer.check(&func, "Renders an HTML template for the frontend");
        assert!(matches!(level, SimilarityLevel::Clear));
    }

    #[test]
    fn empty_store_always_clears() {
        let store = LocalVectorStore::new_empty();
        let cfg = config();
        let enforcer = ReuseEnforcer::new(&store, &cfg);
        let func = make_fn("anything", "src/x.rs");
        assert!(matches!(
            enforcer.check(&func, "does stuff"),
            SimilarityLevel::Clear
        ));
    }

    #[test]
    fn register_then_check_all_returns_results() {
        let store = populated_store();
        let cfg = config();
        let enforcer = ReuseEnforcer::new(&store, &cfg);
        let fns = vec![make_fn("render_html", "src/ui.rs")];
        let results = enforcer.check_all(&fns);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn assert_no_blocks_ok_when_clear() {
        let func = make_fn("foo", "src/foo.rs");
        let results = vec![(func, SimilarityLevel::Clear)];
        assert!(assert_no_blocks(&results, 0.95).is_ok());
    }

    #[test]
    fn similarity_level_accessors() {
        let clear = SimilarityLevel::Clear;
        let warn = SimilarityLevel::Warning {
            score: 0.88,
            similar_fn: "f".into(),
            similar_file: "g".into(),
        };
        let block = SimilarityLevel::Blocked {
            score: 0.97,
            similar_fn: "f".into(),
            similar_file: "g".into(),
        };
        assert!(!clear.is_blocked() && !clear.is_warning());
        assert!(warn.is_warning() && !warn.is_blocked());
        assert!(block.is_blocked() && !block.is_warning());
    }
}
