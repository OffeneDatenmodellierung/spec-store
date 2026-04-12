//! Coverage and quality gate operations.

use crate::{
    coverage::{checker, lcov, CheckResult, FileCoverage},
    scanner::{self, quality, FileViolation, FunctionInfo},
    AppContext,
};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct CoverageReport {
    pub coverage: HashMap<String, FileCoverage>,
    pub results: Vec<CheckResult>,
}

pub fn check_coverage(ctx: &AppContext, lcov_from: Option<&str>) -> anyhow::Result<CoverageReport> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let coverage = lcov::parse(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let results = checker::check_all(&coverage, &ctx.config.coverage, &ctx.baseline);
    Ok(CoverageReport { coverage, results })
}

pub fn update_baseline(ctx: &mut AppContext, lcov_from: Option<&str>) -> anyhow::Result<usize> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let coverage = lcov::parse(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let pct_map = lcov::to_percentage_map(&coverage);
    let count = pct_map.len();
    ctx.baseline.update_from_map(&pct_map);
    ctx.baseline.save().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(count)
}

pub fn check_quality(
    ctx: &AppContext,
    path: Option<&str>,
    file: Option<&str>,
) -> anyhow::Result<Vec<FileViolation>> {
    let target = file
        .map(PathBuf::from)
        .or_else(|| path.map(PathBuf::from))
        .unwrap_or_else(|| ctx.root.join("src"));
    if target.is_file() {
        Ok(vec![quality::check_file(&target, &ctx.config.quality)
            .map_err(|e| anyhow::anyhow!("{e}"))?])
    } else {
        quality::check_dir(&target, &ctx.config.quality).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

pub fn check_quality_staged(ctx: &AppContext) -> anyhow::Result<Vec<FileViolation>> {
    let staged = crate::git::staged_files(&ctx.root);
    let mut violations = Vec::new();
    for file in &staged {
        let path = ctx.root.join(file);
        if path.is_file() && scanner::detect_language(&path) != scanner::Language::Unknown {
            violations.push(
                quality::check_file(&path, &ctx.config.quality)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            );
        }
    }
    Ok(violations)
}

pub fn scan_functions(path: &std::path::Path) -> Vec<FunctionInfo> {
    scanner::scan_dir_functions(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config,
        store::{BaselineStore, LocalVectorStore, StructuredStore},
    };
    use tempfile::TempDir;

    fn test_ctx(dir: &TempDir) -> AppContext {
        AppContext {
            root: dir.path().to_path_buf(),
            config: config::Config::default(),
            structured: StructuredStore::open_in_memory().unwrap(),
            baseline: BaselineStore::new_empty(),
            vectors: LocalVectorStore::new_empty(),
        }
    }

    fn write_lcov(dir: &TempDir) {
        std::fs::write(
            dir.path().join("lcov.info"),
            "SF:src/a.rs\nLF:100\nLH:90\nend_of_record\n",
        )
        .unwrap();
    }

    #[test]
    fn check_coverage_parses_lcov() {
        let dir = TempDir::new().unwrap();
        write_lcov(&dir);
        let ctx = test_ctx(&dir);
        let report = check_coverage(&ctx, Some("lcov.info")).unwrap();
        assert_eq!(report.coverage.len(), 1);
        assert!(!report.results.is_empty());
    }

    #[test]
    fn check_coverage_fails_on_missing_lcov() {
        let dir = TempDir::new().unwrap();
        let ctx = test_ctx(&dir);
        assert!(check_coverage(&ctx, Some("nonexistent.info")).is_err());
    }

    #[test]
    fn update_baseline_ratchets() {
        let dir = TempDir::new().unwrap();
        write_lcov(&dir);
        let mut ctx = test_ctx(&dir);
        let count = update_baseline(&mut ctx, Some("lcov.info")).unwrap();
        assert_eq!(count, 1);
        assert!(ctx.baseline.get("src/a.rs").is_some());
    }

    #[test]
    fn check_quality_on_single_file() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("test.rs");
        std::fs::write(&src, "fn short() {}\n").unwrap();
        let ctx = test_ctx(&dir);
        let violations = check_quality(&ctx, None, Some(src.to_str().unwrap())).unwrap();
        assert_eq!(violations.len(), 1);
        assert!(!violations[0].has_errors());
    }

    #[test]
    fn check_quality_on_directory() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.path().join("b.rs"), "fn b() {}\n").unwrap();
        let ctx = test_ctx(&dir);
        let violations = check_quality(&ctx, Some(dir.path().to_str().unwrap()), None).unwrap();
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn check_quality_defaults_to_src() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}\n").unwrap();
        let ctx = test_ctx(&dir);
        let violations = check_quality(&ctx, None, None).unwrap();
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn scan_functions_finds_rust_fns() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn hello() {}\nfn world() {}\n",
        )
        .unwrap();
        let fns = scan_functions(dir.path());
        assert_eq!(fns.len(), 2);
    }
}
