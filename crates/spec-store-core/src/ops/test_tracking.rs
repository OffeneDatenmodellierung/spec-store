//! Operations for test tracking: listing tests, mappings, and per-function coverage.

use crate::{
    coverage::{fn_coverage, lcov, FnCoverageResult},
    scanner::{self, test_mapper::TestMapping, FunctionInfo},
    AppContext,
};
use std::path::{Path, PathBuf};

/// List all test functions found by the scanner.
pub fn list_tests(path: &Path) -> Vec<FunctionInfo> {
    scanner::scan_dir_functions(path)
        .into_iter()
        .filter(|f| f.is_test)
        .collect()
}

/// Compute per-function coverage from LCOV DA lines.
pub fn function_coverage(
    ctx: &AppContext,
    lcov_from: Option<&str>,
    fn_filter: Option<&str>,
) -> anyhow::Result<Vec<FnCoverageResult>> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let line_coverage = lcov::parse_detail(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    let scan_path = ctx.root.join("src");
    let functions = scanner::scan_dir_functions(&scan_path);

    let mut results = fn_coverage::compute_fn_coverage(&functions, &line_coverage);

    if let Some(filter) = fn_filter {
        results.retain(|r| r.name.contains(filter));
    }

    results.sort_by(|a, b| {
        a.percentage()
            .partial_cmp(&b.percentage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

/// Compute per-function coverage for a specific path.
pub fn function_coverage_for_path(
    ctx: &AppContext,
    lcov_from: Option<&str>,
    scan_path: Option<&str>,
) -> anyhow::Result<Vec<FnCoverageResult>> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let line_coverage = lcov::parse_detail(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    let path = scan_path
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.root.join("src"));
    let functions = scanner::scan_dir_functions(&path);

    let mut results = fn_coverage::compute_fn_coverage(&functions, &line_coverage);
    results.sort_by(|a, b| {
        a.percentage()
            .partial_cmp(&b.percentage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(results)
}

/// Compute test-to-function mappings using name and file heuristics.
pub fn test_mappings(
    ctx: &AppContext,
    scan_path: Option<&str>,
    fn_filter: Option<&str>,
) -> Vec<TestMapping> {
    let path = scan_path
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.root.join("src"));
    let functions = scanner::scan_dir_functions(&path);
    let mut mappings = crate::scanner::test_mapper::compute_mappings(&functions);

    if let Some(filter) = fn_filter {
        mappings.retain(|m| m.function_name.contains(filter));
    }

    mappings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::lcov;

    #[test]
    fn list_tests_filters_correctly() {
        // Scan our own test_detect.rs — it should contain test functions
        let path = std::path::Path::new("src/scanner/test_detect.rs");
        if !path.exists() {
            return; // skip if not running from crate root
        }
        let tests = list_tests(path.parent().unwrap());
        assert!(
            tests.iter().all(|f| f.is_test),
            "list_tests should only return test functions"
        );
    }

    #[test]
    fn parse_detail_extracts_da_lines() {
        let content = "\
SF:src/a.rs
DA:10,1
DA:11,0
DA:12,3
LF:3
LH:2
end_of_record
";
        let result = lcov::parse_detail_content(content).unwrap();
        let lines = &result["src/a.rs"];
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line, 10);
        assert_eq!(lines[0].hits, 1);
        assert_eq!(lines[1].hits, 0);
    }
}
