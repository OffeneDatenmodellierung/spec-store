use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub coverage: CoverageConfig,
    pub quality: QualityConfig,
    pub reuse: ReuseConfig,
    /// Legacy — AI is handled by the calling agent, not spec-store.
    #[serde(default)]
    pub ai: AiConfig,
    /// Legacy — embeddings use local word-bag; this section is ignored.
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageConfig {
    pub min_per_file: f64,
    pub lcov_path: String,
    pub lcov_max_age_mins: u64,
    pub ratchet: bool,
    pub fail_on_regression: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    pub max_file_lines: usize,
    pub max_fn_lines: usize,
    pub max_fn_complexity: usize,
    pub max_fns_per_file: usize,
    pub max_fn_params: usize,
    pub warn_only: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReuseConfig {
    pub similarity_warn: f32,
    pub similarity_block: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    pub lightllm: Option<LightllmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightllmConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingsConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            coverage: CoverageConfig {
                min_per_file: 85.0,
                lcov_path: "lcov.info".into(),
                lcov_max_age_mins: 60,
                ratchet: true,
                fail_on_regression: true,
                exclude: vec![
                    "src/generated/**".into(),
                    "tests/**".into(),
                    "benches/**".into(),
                ],
            },
            quality: QualityConfig {
                max_file_lines: 300,
                max_fn_lines: 50,
                max_fn_complexity: 10,
                max_fns_per_file: 15,
                max_fn_params: 5,
                warn_only: false,
                exclude: vec!["src/generated/**".into()],
            },
            reuse: ReuseConfig {
                similarity_warn: 0.85,
                similarity_block: 0.95,
            },
            ai: AiConfig::default(),
            embeddings: EmbeddingsConfig::default(),
        }
    }
}

pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(".spec-store").join("config.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    toml::from_str(&raw).map_err(|e| anyhow::anyhow!("Config parse error: {e}"))
}

pub fn find_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join(".spec-store").exists() || dir.join(".git").exists() {
            return dir;
        }
        if !dir.pop() {
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

pub fn save_default(root: &Path) -> Result<()> {
    let dir = root.join(".spec-store");
    std::fs::create_dir_all(&dir)?;
    let content = toml::to_string_pretty(&Config::default())
        .map_err(|e| anyhow::anyhow!("Serialise error: {e}"))?;
    std::fs::write(dir.join("config.toml"), content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_thresholds_are_correct() {
        let c = Config::default();
        assert_eq!(c.coverage.min_per_file, 85.0);
        assert_eq!(c.quality.max_file_lines, 300);
        assert_eq!(c.quality.max_fn_lines, 50);
        assert!(c.coverage.ratchet);
        assert!(c.coverage.fail_on_regression);
    }

    #[test]
    fn load_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let c = load(dir.path()).unwrap();
        assert_eq!(c.coverage.min_per_file, 85.0);
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let dir = TempDir::new().unwrap();
        save_default(dir.path()).unwrap();
        let c = load(dir.path()).unwrap();
        assert_eq!(c.quality.max_fns_per_file, 15);
    }

    #[test]
    fn find_root_returns_existing_path() {
        let root = find_root();
        assert!(root.exists());
    }

    #[test]
    fn reuse_thresholds_ordered_correctly() {
        let c = Config::default();
        assert!(c.reuse.similarity_block > c.reuse.similarity_warn);
    }
}
