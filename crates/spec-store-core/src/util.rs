/// Check whether a path should be excluded based on glob-like patterns.
/// Patterns like `src/generated/**` or `tests/**` match if the path contains the stem.
pub fn is_excluded(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        let stem = p.trim_end_matches("**").trim_end_matches('/');
        path.contains(stem) || path.starts_with(stem)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_matching_path() {
        let patterns = vec!["src/generated/**".into()];
        assert!(is_excluded("src/generated/schema.rs", &patterns));
    }

    #[test]
    fn does_not_exclude_unrelated_path() {
        let patterns = vec!["src/generated/**".into()];
        assert!(!is_excluded("src/core/main.rs", &patterns));
    }

    #[test]
    fn excludes_tests_directory() {
        let patterns = vec!["tests/**".into()];
        assert!(is_excluded("tests/integration/test_api.rs", &patterns));
    }

    #[test]
    fn empty_patterns_excludes_nothing() {
        assert!(!is_excluded("src/anything.rs", &[]));
    }
}
