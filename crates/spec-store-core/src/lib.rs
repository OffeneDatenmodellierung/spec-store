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
