use crate::error::{Result, SpecStoreError};
use regex::Regex;
use std::{path::Path, sync::OnceLock};

pub use super::language::{
    compiled_patterns, detect_language, is_source_path, profile_for, profile_for_path, Language,
    LanguageProfile,
};

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub line_count: usize,
    pub param_count: usize,
    pub complexity: usize,
    pub is_test: bool,
}

pub fn scan_file(path: &Path) -> Result<Vec<FunctionInfo>> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| SpecStoreError::Scanner(format!("{}: {e}", path.display())))?;
    let lang = detect_language(path);
    Ok(extract_functions(
        &source,
        path.to_string_lossy().as_ref(),
        lang,
    ))
}

pub fn scan_source(source: &str, file: &str, lang: Language) -> Vec<FunctionInfo> {
    extract_functions(source, file, lang)
}

fn extract_functions(source: &str, file: &str, lang: Language) -> Vec<FunctionInfo> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<FunctionInfo> = vec![];
    for re in compiled_patterns(lang) {
        for cap in re.captures_iter(source) {
            let line = byte_offset_to_line(source, cap.get(0).unwrap().start());
            let name = cap[1].to_string();
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let param_count = count_params(lang, params_str);
            let (line_count, complexity) = measure_block(&lines, line);
            let is_test = is_test_function(lang, &name, source, file, line);
            out.push(FunctionInfo {
                name,
                file: file.to_string(),
                line,
                line_count,
                param_count,
                complexity,
                is_test,
            });
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

fn count_params(lang: Language, params: &str) -> usize {
    match lang {
        Language::Rust => count_params_rust(params),
        Language::Python => count_params_named_self(params, "self"),
        Language::TypeScript => count_params_simple(params),
        Language::Unknown => 0,
    }
}

fn is_test_function(lang: Language, name: &str, source: &str, file: &str, line: usize) -> bool {
    use super::test_detect;
    match lang {
        Language::Rust => {
            let ranges = test_detect::find_cfg_test_ranges(source);
            test_detect::is_test_rust(source, line, &ranges)
        }
        Language::Python => test_detect::is_test_python(name, source, line),
        Language::TypeScript => test_detect::is_test_typescript(name, file),
        Language::Unknown => false,
    }
}

static COMPLEXITY: OnceLock<Regex> = OnceLock::new();
static RUST_PARAMS: OnceLock<Regex> = OnceLock::new();

pub fn complexity_re() -> &'static Regex {
    COMPLEXITY.get_or_init(|| Regex::new(r"\b(if|else if|while|for|loop|match)\b|&&|\|\|").unwrap())
}
fn rust_params_re() -> &'static Regex {
    RUST_PARAMS.get_or_init(|| Regex::new(r"(?:&(?:mut\s+)?self\s*,?\s*)?").unwrap())
}

fn count_params_rust(params: &str) -> usize {
    let cleaned = rust_params_re().replace(params, "");
    if cleaned.trim().is_empty() {
        return 0;
    }
    cleaned.split(',').filter(|p| !p.trim().is_empty()).count()
}

fn count_params_named_self(params: &str, self_name: &str) -> usize {
    if params.trim().is_empty() {
        return 0;
    }
    params
        .split(',')
        .filter(|p| {
            let t = p.trim();
            !t.is_empty() && t != self_name
        })
        .count()
}

fn count_params_simple(params: &str) -> usize {
    if params.trim().is_empty() {
        return 0;
    }
    params.split(',').filter(|p| !p.trim().is_empty()).count()
}

fn byte_offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset].chars().filter(|&c| c == '\n').count() + 1
}

use super::block_measure::measure_block;

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SRC: &str = r#"
pub fn validate_stake(amount: f64, limit: f64) -> bool {
    if amount <= 0.0 { return false; }
    amount <= limit
}

async fn fetch_customer(id: u64, db: &Database) -> Result<Customer> {
    db.find(id).await
}

fn no_params() {}
"#;

    #[test]
    fn detects_rust_language() {
        assert_eq!(detect_language(Path::new("src/foo.rs")), Language::Rust);
        assert_eq!(detect_language(Path::new("app.py")), Language::Python);
        assert_eq!(detect_language(Path::new("util.ts")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("data.json")), Language::Unknown);
    }

    #[test]
    fn extracts_rust_functions() {
        let fns = scan_source(RUST_SRC, "src/risk.rs", Language::Rust);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"validate_stake"));
        assert!(names.contains(&"fetch_customer"));
        assert!(names.contains(&"no_params"));
    }

    #[test]
    fn counts_rust_parameters_correctly() {
        let fns = scan_source(RUST_SRC, "src/risk.rs", Language::Rust);
        let stake = fns.iter().find(|f| f.name == "validate_stake").unwrap();
        assert_eq!(stake.param_count, 2);
        let no_params = fns.iter().find(|f| f.name == "no_params").unwrap();
        assert_eq!(no_params.param_count, 0);
    }

    #[test]
    fn extracts_python_functions() {
        let src = "def calculate_total(items, tax_rate):\n    return sum(items) * tax_rate\n";
        let fns = scan_source(src, "calc.py", Language::Python);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "calculate_total");
        assert_eq!(fns[0].param_count, 2);
    }

    #[test]
    fn extracts_typescript_functions() {
        let src = "export async function fetchUser(id: string, token: string): Promise<User> {\n  return api.get(id);\n}\n";
        let fns = scan_source(src, "api.ts", Language::TypeScript);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].param_count, 2);
    }

    #[test]
    fn extracts_typescript_arrow_functions() {
        let src = "export const fetchUser = async (id: string, token: string): Promise<User> => {\n  return api.get(id);\n};\n";
        let fns = scan_source(src, "api.ts", Language::TypeScript);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "fetchUser");
        assert_eq!(fns[0].param_count, 2);
    }

    #[test]
    fn extracts_javascript_arrow_functions() {
        let src = "const add = (a, b) => a + b;\n";
        let fns = scan_source(src, "math.js", Language::TypeScript);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "add");
        assert_eq!(fns[0].param_count, 2);
    }

    #[test]
    fn does_not_double_count_when_patterns_overlap() {
        let src = "function foo() {}\nconst bar = () => {};\n";
        let fns = scan_source(src, "x.ts", Language::TypeScript);
        assert_eq!(fns.len(), 2);
    }

    #[test]
    fn unknown_language_returns_empty() {
        let fns = scan_source("some content", "data.json", Language::Unknown);
        assert!(fns.is_empty());
    }

    #[test]
    fn count_params_unknown_returns_zero() {
        assert_eq!(count_params(Language::Unknown, "a, b, c"), 0);
    }

    #[test]
    fn is_test_function_unknown_is_false() {
        assert!(!is_test_function(
            Language::Unknown,
            "anything",
            "",
            "x.txt",
            1
        ));
    }

    #[test]
    fn count_params_named_self_empty_is_zero() {
        assert_eq!(count_params_named_self("   ", "self"), 0);
    }

    #[test]
    fn complexity_includes_base_of_one() {
        let fns = scan_source(RUST_SRC, "src/risk.rs", Language::Rust);
        let stake = fns.iter().find(|f| f.name == "validate_stake").unwrap();
        assert!(stake.complexity >= 1);
    }
}
