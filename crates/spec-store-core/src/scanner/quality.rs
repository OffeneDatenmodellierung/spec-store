use crate::{
    config::QualityConfig,
    error::{Result, SpecStoreError},
    scanner::regex_scanner::{self, FunctionInfo},
};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileViolation {
    pub file: String,
    pub violations: Vec<Violation>,
}

impl FileViolation {
    pub fn has_errors(&self) -> bool {
        self.violations.iter().any(|v| !v.is_warning)
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub message: String,
    pub is_warning: bool,
}

impl Violation {
    fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            is_warning: false,
        }
    }
    fn warn(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            is_warning: true,
        }
    }
}

pub fn check_file(path: &Path, config: &QualityConfig) -> Result<FileViolation> {
    if is_excluded(path, &config.exclude) {
        return Ok(FileViolation {
            file: path.to_string_lossy().into(),
            violations: vec![],
        });
    }
    let source =
        std::fs::read_to_string(path).map_err(|e| SpecStoreError::Scanner(e.to_string()))?;
    let lang = regex_scanner::detect_language(path);
    let file = path.to_string_lossy().to_string();
    let functions = regex_scanner::scan_source(&source, &file, lang);
    let code_lines = count_code_lines(&source);
    let mut violations = vec![];
    let prod_functions: Vec<_> = functions.iter().filter(|f| !f.is_test).collect();
    check_file_length(code_lines, config, &mut violations);
    check_function_count(prod_functions.len(), config, &mut violations);
    for f in &prod_functions {
        check_function(f, config, &mut violations);
    }
    Ok(FileViolation { file, violations })
}

pub fn check_dir(root: &Path, config: &QualityConfig) -> Result<Vec<FileViolation>> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_source_file(e.path()))
        .filter(|e| !is_excluded(e.path(), &config.exclude))
        .map(|e| check_file(e.path(), config))
        .collect()
}

pub fn has_errors(violations: &[FileViolation], warn_only: bool) -> bool {
    if warn_only {
        return false;
    }
    violations.iter().any(|v| v.has_errors())
}

fn check_file_length(lines: usize, config: &QualityConfig, v: &mut Vec<Violation>) {
    if lines > config.max_file_lines {
        let mk = if config.warn_only {
            Violation::warn
        } else {
            Violation::error
        };
        v.push(mk(format!(
            "File has {lines} code lines (max {})",
            config.max_file_lines
        )));
    }
}

fn check_function_count(count: usize, config: &QualityConfig, v: &mut Vec<Violation>) {
    if count > config.max_fns_per_file {
        let mk = if config.warn_only {
            Violation::warn
        } else {
            Violation::error
        };
        v.push(mk(format!(
            "{count} functions in file (max {})",
            config.max_fns_per_file
        )));
    }
}

fn check_function(f: &FunctionInfo, config: &QualityConfig, v: &mut Vec<Violation>) {
    let mk = |msg| {
        if config.warn_only {
            Violation::warn(msg)
        } else {
            Violation::error(msg)
        }
    };
    if f.line_count > config.max_fn_lines {
        v.push(mk(format!(
            "fn {}(): {} lines (max {})",
            f.name, f.line_count, config.max_fn_lines
        )));
    }
    if f.complexity > config.max_fn_complexity {
        v.push(mk(format!(
            "fn {}(): complexity {} (max {})",
            f.name, f.complexity, config.max_fn_complexity
        )));
    }
    if f.param_count > config.max_fn_params {
        v.push(Violation::warn(format!(
            "fn {}(): {} params (max {} — consider a config struct)",
            f.name, f.param_count, config.max_fn_params
        )));
    }
}

fn count_code_lines(source: &str) -> usize {
    let test_ranges = crate::scanner::test_detect::find_cfg_test_ranges(source);
    source
        .lines()
        .enumerate()
        .filter(|(i, l)| {
            let line_num = i + 1; // 1-indexed
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with('#')
                && !test_ranges
                    .iter()
                    .any(|&(start, end)| line_num >= start && line_num <= end)
        })
        .count()
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs") | Some("py") | Some("ts") | Some("tsx")
    )
}

fn is_excluded(path: &Path, patterns: &[String]) -> bool {
    crate::util::is_excluded(&path.to_string_lossy(), patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::QualityConfig;

    fn default_config() -> QualityConfig {
        QualityConfig {
            max_file_lines: 300,
            max_fn_lines: 50,
            max_fn_complexity: 10,
            max_fns_per_file: 15,
            max_fn_params: 5,
            warn_only: false,
            exclude: vec![],
        }
    }

    #[test]
    fn clean_file_has_no_violations() {
        let fv = FileViolation {
            file: "f.rs".into(),
            violations: vec![],
        };
        assert!(!fv.has_errors());
    }

    #[test]
    fn file_with_error_has_errors() {
        let fv = FileViolation {
            file: "f.rs".into(),
            violations: vec![Violation::error("too long")],
        };
        assert!(fv.has_errors());
    }

    #[test]
    fn warn_only_does_not_block() {
        let fv = FileViolation {
            file: "f.rs".into(),
            violations: vec![Violation::error("too long")],
        };
        assert!(!has_errors(&[fv], true));
    }

    #[test]
    fn count_code_lines_skips_blanks_and_comments() {
        let src = "// comment\n\nfn foo() {}\n# python comment\n";
        assert_eq!(count_code_lines(src), 1);
    }

    #[test]
    fn file_length_violation_raised() {
        let mut v = vec![];
        let mut config = default_config();
        config.max_file_lines = 5;
        check_file_length(10, &config, &mut v);
        assert_eq!(v.len(), 1);
        assert!(!v[0].is_warning);
    }

    #[test]
    fn file_length_warn_only() {
        let mut v = vec![];
        let mut config = default_config();
        config.max_file_lines = 5;
        config.warn_only = true;
        check_file_length(10, &config, &mut v);
        assert!(v[0].is_warning);
    }

    #[test]
    fn function_count_violation() {
        let mut v = vec![];
        let mut config = default_config();
        config.max_fns_per_file = 2;
        check_function_count(5, &config, &mut v);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn is_source_file_matches_expected_extensions() {
        assert!(is_source_file(Path::new("foo.rs")));
        assert!(is_source_file(Path::new("bar.py")));
        assert!(is_source_file(Path::new("baz.ts")));
        assert!(!is_source_file(Path::new("data.json")));
    }

    #[test]
    fn check_file_on_clean_source() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("clean.rs");
        std::fs::write(&src, "fn hello() { println!(\"hi\"); }\n").unwrap();
        let result = check_file(&src, &default_config()).unwrap();
        assert!(!result.has_errors());
    }

    #[test]
    fn check_file_skips_excluded() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("generated.rs");
        std::fs::write(&src, "fn a(){}\nfn b(){}\n".repeat(20)).unwrap();
        let mut cfg = default_config();
        cfg.exclude = vec!["generated".into()];
        let result = check_file(&src, &cfg).unwrap();
        assert!(result.violations.is_empty());
    }

    #[test]
    fn check_dir_finds_violations() {
        let dir = tempfile::TempDir::new().unwrap();
        // Create a file that's too long
        let long_fn = format!("fn big() {{\n{}\n}}\n", "let x = 1;\n".repeat(60));
        std::fs::write(dir.path().join("big.rs"), long_fn).unwrap();
        let result = check_dir(dir.path(), &default_config()).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn check_function_complexity_violation() {
        let info = FunctionInfo {
            name: "complex".into(),
            file: "f.rs".into(),
            line: 1,
            line_count: 10,
            param_count: 0,
            complexity: 20,
            is_test: false,
        };
        let mut v = vec![];
        check_function(&info, &default_config(), &mut v);
        assert!(v.iter().any(|v| v.message.contains("complexity")));
    }

    #[test]
    fn check_function_params_warning() {
        let info = FunctionInfo {
            name: "many_params".into(),
            file: "f.rs".into(),
            line: 1,
            line_count: 5,
            param_count: 8,
            complexity: 1,
            is_test: false,
        };
        let mut v = vec![];
        check_function(&info, &default_config(), &mut v);
        assert!(v
            .iter()
            .any(|v| v.is_warning && v.message.contains("params")));
    }
}
