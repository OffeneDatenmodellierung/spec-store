//! Per-function coverage computation by cross-referencing scanned functions with LCOV DA lines.

use crate::coverage::lcov::LineCoverage;
use crate::scanner::FunctionInfo;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FnCoverageResult {
    pub file: String,
    pub name: String,
    pub is_test: bool,
    pub line_start: usize,
    pub line_count: usize,
    pub lines_found: usize,
    pub lines_hit: usize,
}

impl FnCoverageResult {
    pub fn percentage(&self) -> f64 {
        if self.lines_found == 0 {
            return 100.0;
        }
        (self.lines_hit as f64 / self.lines_found as f64) * 100.0
    }
}

/// Cross-reference scanned functions with LCOV DA lines to compute per-function coverage.
pub fn compute_fn_coverage(
    functions: &[FunctionInfo],
    line_coverage: &HashMap<String, Vec<LineCoverage>>,
) -> Vec<FnCoverageResult> {
    functions
        .iter()
        .filter(|f| f.line_count > 0) // skip synthetic entries
        .filter_map(|f| {
            let file_lines = line_coverage.get(&f.file)?;
            let fn_start = f.line;
            let fn_end = f.line + f.line_count;

            let relevant: Vec<&LineCoverage> = file_lines
                .iter()
                .filter(|lc| lc.line >= fn_start && lc.line < fn_end)
                .collect();

            let lines_found = relevant.len();
            let lines_hit = relevant.iter().filter(|lc| lc.hits > 0).count();

            Some(FnCoverageResult {
                file: f.file.clone(),
                name: f.name.clone(),
                is_test: f.is_test,
                line_start: f.line,
                line_count: f.line_count,
                lines_found,
                lines_hit,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fn(name: &str, file: &str, line: usize, count: usize, is_test: bool) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            file: file.into(),
            line,
            line_count: count,
            param_count: 0,
            complexity: 1,
            is_test,
        }
    }

    fn make_da(entries: &[(usize, u64)]) -> Vec<LineCoverage> {
        entries
            .iter()
            .map(|&(line, hits)| LineCoverage { line, hits })
            .collect()
    }

    #[test]
    fn computes_full_coverage() {
        let fns = vec![make_fn("foo", "src/a.rs", 10, 5, false)];
        let mut lcov = HashMap::new();
        lcov.insert(
            "src/a.rs".into(),
            make_da(&[(10, 1), (11, 1), (12, 1), (13, 1), (14, 1)]),
        );
        let results = compute_fn_coverage(&fns, &lcov);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].lines_found, 5);
        assert_eq!(results[0].lines_hit, 5);
        assert!((results[0].percentage() - 100.0).abs() < 0.01);
    }

    #[test]
    fn computes_partial_coverage() {
        let fns = vec![make_fn("bar", "src/b.rs", 20, 4, false)];
        let mut lcov = HashMap::new();
        lcov.insert(
            "src/b.rs".into(),
            make_da(&[(20, 1), (21, 0), (22, 1), (23, 0)]),
        );
        let results = compute_fn_coverage(&fns, &lcov);
        assert_eq!(results[0].lines_hit, 2);
        assert!((results[0].percentage() - 50.0).abs() < 0.01);
    }

    #[test]
    fn skips_functions_not_in_lcov() {
        let fns = vec![make_fn("missing", "src/c.rs", 1, 10, false)];
        let lcov = HashMap::new(); // no data
        let results = compute_fn_coverage(&fns, &lcov);
        assert!(results.is_empty());
    }

    #[test]
    fn includes_test_functions() {
        let fns = vec![make_fn("test_foo", "src/a.rs", 50, 3, true)];
        let mut lcov = HashMap::new();
        lcov.insert("src/a.rs".into(), make_da(&[(50, 1), (51, 1), (52, 1)]));
        let results = compute_fn_coverage(&fns, &lcov);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_test);
    }

    #[test]
    fn skips_zero_line_count() {
        let fns = vec![make_fn("synthetic", "src/a.rs", 1, 0, false)];
        let mut lcov = HashMap::new();
        lcov.insert("src/a.rs".into(), make_da(&[(1, 1)]));
        let results = compute_fn_coverage(&fns, &lcov);
        assert!(results.is_empty());
    }

    #[test]
    fn multiple_functions_same_file() {
        let fns = vec![
            make_fn("foo", "src/a.rs", 10, 5, false),
            make_fn("bar", "src/a.rs", 20, 3, false),
        ];
        let mut lcov = HashMap::new();
        lcov.insert(
            "src/a.rs".into(),
            make_da(&[
                (10, 1),
                (11, 1),
                (12, 0),
                (13, 1),
                (14, 1),
                (20, 1),
                (21, 0),
                (22, 0),
            ]),
        );
        let results = compute_fn_coverage(&fns, &lcov);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].lines_hit, 4); // foo: 4/5
        assert_eq!(results[1].lines_hit, 1); // bar: 1/3
    }
}
