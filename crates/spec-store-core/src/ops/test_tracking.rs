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
    fn list_tests_on_tempdir_with_test_fn() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("example.rs"),
            "#[test]\nfn test_thing() {\n}\n\nfn prod_fn() {\n}\n",
        )
        .unwrap();
        let all = crate::scanner::scan_dir_functions(dir.path());
        let tests = list_tests(dir.path());
        assert_eq!(all.len(), 2, "should find 2 functions total");
        assert_eq!(tests.len(), 1, "should find 1 test function");
        assert_eq!(tests[0].name, "test_thing");
    }

    #[test]
    fn list_tests_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(list_tests(dir.path()).is_empty());
    }

    #[test]
    fn test_mappings_with_filter() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("example.rs"),
            "fn validate() {}\n#[test]\nfn test_validate() {}\n#[test]\nfn test_other() {}\n",
        )
        .unwrap();
        let ctx = crate::AppContext {
            root: dir.path().to_path_buf(),
            config: crate::config::Config::default(),
            structured: crate::store::StructuredStore::open_in_memory().unwrap(),
            baseline: crate::store::BaselineStore::new_empty(),
            vectors: crate::store::LocalVectorStore::new_empty(),
        };
        let all = test_mappings(&ctx, Some(dir.path().to_str().unwrap()), None);
        assert!(!all.is_empty());
        let filtered = test_mappings(&ctx, Some(dir.path().to_str().unwrap()), Some("validate"));
        assert!(filtered
            .iter()
            .all(|m| m.function_name.contains("validate")));
    }

    #[test]
    fn function_coverage_from_lcov_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "pub fn foo() {\n    let x = 1;\n    let y = 2;\n}\n",
        )
        .unwrap();
        let lcov_content = format!(
            "SF:{}/src/lib.rs\nDA:1,1\nDA:2,1\nDA:3,0\nDA:4,1\nLF:4\nLH:3\nend_of_record\n",
            dir.path().display()
        );
        std::fs::write(dir.path().join("lcov.info"), lcov_content).unwrap();
        let ctx = crate::AppContext {
            root: dir.path().to_path_buf(),
            config: crate::config::Config::default(),
            structured: crate::store::StructuredStore::open_in_memory().unwrap(),
            baseline: crate::store::BaselineStore::new_empty(),
            vectors: crate::store::LocalVectorStore::new_empty(),
        };
        // Should not error — coverage may or may not match depending on paths
        let _results = function_coverage(&ctx, Some("lcov.info"), None).unwrap();
    }

    #[test]
    fn function_coverage_with_filter() {
        let dir = tempfile::TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("lib.rs"), "fn foo() {\n}\nfn bar() {\n}\n").unwrap();
        std::fs::write(
            dir.path().join("lcov.info"),
            "SF:src/lib.rs\nDA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nLF:4\nLH:4\nend_of_record\n",
        )
        .unwrap();
        let ctx = crate::AppContext {
            root: dir.path().to_path_buf(),
            config: crate::config::Config::default(),
            structured: crate::store::StructuredStore::open_in_memory().unwrap(),
            baseline: crate::store::BaselineStore::new_empty(),
            vectors: crate::store::LocalVectorStore::new_empty(),
        };
        let filtered = function_coverage(&ctx, Some("lcov.info"), Some("foo")).unwrap();
        assert!(filtered.iter().all(|r| r.name.contains("foo")));
    }

    #[test]
    fn function_coverage_missing_lcov_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = crate::AppContext {
            root: dir.path().to_path_buf(),
            config: crate::config::Config::default(),
            structured: crate::store::StructuredStore::open_in_memory().unwrap(),
            baseline: crate::store::BaselineStore::new_empty(),
            vectors: crate::store::LocalVectorStore::new_empty(),
        };
        assert!(function_coverage(&ctx, Some("nonexistent.info"), None).is_err());
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
