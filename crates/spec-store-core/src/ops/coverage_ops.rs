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
