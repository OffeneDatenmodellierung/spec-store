pub mod config;
pub mod context;
pub mod coverage;
pub mod error;
pub mod git;
pub mod hooks;
pub mod ops;
pub mod reuse;
pub mod scanner;
pub mod store;
pub mod util;

use std::path::PathBuf;

pub struct AppContext {
    pub root: PathBuf,
    pub config: config::Config,
    pub structured: store::StructuredStore,
    pub baseline: store::BaselineStore,
    pub vectors: store::LocalVectorStore,
}

impl AppContext {
    pub fn load() -> anyhow::Result<Self> {
        let root = config::find_root();
        Self::load_from(root)
    }

    pub fn load_from(root: PathBuf) -> anyhow::Result<Self> {
        let cfg = config::load(&root)?;
        let structured = store::StructuredStore::open(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
        let baseline = store::BaselineStore::load(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
        let vectors = store::LocalVectorStore::load(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            root,
            config: cfg,
            structured,
            baseline,
            vectors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_creates_context() {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = AppContext::load_from(dir.path().to_path_buf()).unwrap();
        assert_eq!(ctx.root, dir.path());
        assert_eq!(ctx.config.coverage.min_per_file, 85.0);
    }

    #[test]
    fn load_finds_root() {
        // load() walks up from cwd — should succeed from any dir
        let ctx = AppContext::load();
        assert!(ctx.is_ok());
    }
}
