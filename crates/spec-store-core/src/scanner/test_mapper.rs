//! Map test functions to the production functions they test, using name and file heuristics.

use crate::scanner::FunctionInfo;

#[derive(Debug, Clone)]
pub struct TestMapping {
    pub function_file: String,
    pub function_name: String,
    pub test_file: String,
    pub test_name: String,
    pub match_type: String,
}

/// Compute mappings between production and test functions.
///
/// Heuristics:
/// 1. **Name match**: test `test_validate_stake` maps to prod `validate_stake`
/// 2. **File match**: test in same file's `#[cfg(test)]` module maps to all prod fns in that file
pub fn compute_mappings(functions: &[FunctionInfo]) -> Vec<TestMapping> {
    let tests: Vec<&FunctionInfo> = functions.iter().filter(|f| f.is_test).collect();
    let prods: Vec<&FunctionInfo> = functions.iter().filter(|f| !f.is_test).collect();
    let mut mappings = Vec::new();

    for test in &tests {
        let stripped = strip_test_prefix(&test.name);
        let mut matched_by_name = false;

        // Name heuristic: test_validate_stake → validate_stake
        if let Some(stripped) = &stripped {
            for prod in &prods {
                if prod.name == *stripped || stripped.starts_with(&prod.name) {
                    mappings.push(TestMapping {
                        function_file: prod.file.clone(),
                        function_name: prod.name.clone(),
                        test_file: test.file.clone(),
                        test_name: test.name.clone(),
                        match_type: "name".into(),
                    });
                    matched_by_name = true;
                }
            }
        }

        // File heuristic: same file, no name match found
        if !matched_by_name {
            let test_base = normalize_file(&test.file);
            for prod in &prods {
                let prod_base = normalize_file(&prod.file);
                if test_base == prod_base {
                    mappings.push(TestMapping {
                        function_file: prod.file.clone(),
                        function_name: prod.name.clone(),
                        test_file: test.file.clone(),
                        test_name: test.name.clone(),
                        match_type: "file".into(),
                    });
                }
            }
        }
    }

    mappings
}

/// Strip test prefixes: `test_foo` → `foo`, `tests::test_foo` → `foo`
fn strip_test_prefix(name: &str) -> Option<String> {
    let name = name.strip_prefix("tests::").unwrap_or(name);
    name.strip_prefix("test_").map(|s| s.to_string())
}

/// Normalize file path for comparison: strip `tests/test_` prefix, `.test.` suffix, etc.
fn normalize_file(path: &str) -> String {
    path.replace("tests/test_", "src/")
        .replace("tests/", "src/")
        .replace(".test.ts", ".ts")
        .replace(".test.tsx", ".tsx")
        .replace(".spec.ts", ".ts")
        .replace(".spec.tsx", ".tsx")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prod(name: &str, file: &str) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            file: file.into(),
            line: 1,
            line_count: 10,
            param_count: 0,
            complexity: 1,
            is_test: false,
        }
    }

    fn test_fn(name: &str, file: &str) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            file: file.into(),
            line: 50,
            line_count: 5,
            param_count: 0,
            complexity: 1,
            is_test: true,
        }
    }

    #[test]
    fn name_match_links_test_to_prod() {
        let fns = vec![
            prod("validate_stake", "src/risk.rs"),
            test_fn("test_validate_stake", "src/risk.rs"),
        ];
        let mappings = compute_mappings(&fns);
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].function_name, "validate_stake");
        assert_eq!(mappings[0].test_name, "test_validate_stake");
        assert_eq!(mappings[0].match_type, "name");
    }

    #[test]
    fn name_prefix_match() {
        let fns = vec![
            prod("validate", "src/risk.rs"),
            test_fn("test_validate_returns_true", "src/risk.rs"),
        ];
        let mappings = compute_mappings(&fns);
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].function_name, "validate");
        assert_eq!(mappings[0].match_type, "name");
    }

    #[test]
    fn file_match_when_no_name_match() {
        let fns = vec![
            prod("parse_config", "src/config.rs"),
            test_fn("test_roundtrip", "src/config.rs"),
        ];
        let mappings = compute_mappings(&fns);
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].function_name, "parse_config");
        assert_eq!(mappings[0].match_type, "file");
    }

    #[test]
    fn no_mapping_for_unrelated() {
        let fns = vec![
            prod("validate_stake", "src/risk.rs"),
            test_fn("test_render_page", "src/ui.rs"),
        ];
        let mappings = compute_mappings(&fns);
        assert!(mappings.is_empty());
    }

    #[test]
    fn strip_test_prefix_works() {
        assert_eq!(strip_test_prefix("test_foo"), Some("foo".into()));
        assert_eq!(strip_test_prefix("tests::test_bar"), Some("bar".into()));
        assert_eq!(strip_test_prefix("helper"), None);
    }

    #[test]
    fn multiple_tests_for_one_function() {
        let fns = vec![
            prod("validate", "src/risk.rs"),
            test_fn("test_validate_positive", "src/risk.rs"),
            test_fn("test_validate_negative", "src/risk.rs"),
        ];
        let mappings = compute_mappings(&fns);
        assert_eq!(mappings.len(), 2);
        assert!(mappings.iter().all(|m| m.function_name == "validate"));
    }
}
