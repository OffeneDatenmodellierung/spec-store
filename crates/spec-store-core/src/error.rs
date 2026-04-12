use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpecStoreError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Coverage error: {0}")]
    Coverage(String),

    #[error("Quality gate failed")]
    QualityGate,

    #[error("File '{file}' coverage {actual:.1}% is below the {required:.1}% threshold")]
    CoverageThreshold {
        file: String,
        actual: f64,
        required: f64,
    },

    #[error("File '{file}' coverage {current:.1}% regressed below baseline {baseline:.1}%")]
    CoverageRegression {
        file: String,
        current: f64,
        baseline: f64,
    },

    #[error("Hook installation error: {0}")]
    HookInstall(String),

    #[error("Scanner error: {0}")]
    Scanner(String),

    #[error("Worktree conflict: {0}")]
    WorktreeConflict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),
}

pub type Result<T> = std::result::Result<T, SpecStoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_threshold_displays_file_and_values() {
        let err = SpecStoreError::CoverageThreshold {
            file: "src/foo.rs".into(),
            actual: 72.3,
            required: 85.0,
        };
        let msg = err.to_string();
        assert!(msg.contains("src/foo.rs"));
        assert!(msg.contains("72.3"));
        assert!(msg.contains("85.0"));
    }

    #[test]
    fn coverage_regression_displays_baseline() {
        let err = SpecStoreError::CoverageRegression {
            file: "src/bar.rs".into(),
            current: 80.0,
            baseline: 88.5,
        };
        let msg = err.to_string();
        assert!(msg.contains("88.5"));
    }

    #[test]
    fn io_error_converts_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: SpecStoreError = io_err.into();
        assert!(matches!(err, SpecStoreError::Io(_)));
    }

    #[test]
    fn quality_gate_has_display() {
        let err = SpecStoreError::QualityGate;
        assert!(!err.to_string().is_empty());
    }
}
