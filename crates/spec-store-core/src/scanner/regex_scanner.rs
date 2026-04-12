use crate::error::{Result, SpecStoreError};
use regex::Regex;
use std::{path::Path, sync::OnceLock};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    Unknown,
}

pub fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Language::Rust,
        Some("py") => Language::Python,
        Some("ts") | Some("tsx") => Language::TypeScript,
        _ => Language::Unknown,
    }
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
    match lang {
        Language::Rust => extract_rust(source, file),
        Language::Python => extract_python(source, file),
        Language::TypeScript => extract_typescript(source, file),
        Language::Unknown => vec![],
    }
}

static RUST_FN: OnceLock<Regex> = OnceLock::new();
static PYTHON_FN: OnceLock<Regex> = OnceLock::new();
static TS_FN: OnceLock<Regex> = OnceLock::new();
static COMPLEXITY: OnceLock<Regex> = OnceLock::new();
static RUST_PARAMS: OnceLock<Regex> = OnceLock::new();

fn rust_fn_re() -> &'static Regex {
    RUST_FN.get_or_init(|| {
        Regex::new(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)")
            .unwrap()
    })
}
fn python_fn_re() -> &'static Regex {
    PYTHON_FN.get_or_init(|| {
        Regex::new(r"(?m)^(?:    )*(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)").unwrap()
    })
}
fn ts_fn_re() -> &'static Regex {
    TS_FN.get_or_init(|| {
        Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)").unwrap()
    })
}
fn complexity_re() -> &'static Regex {
    COMPLEXITY.get_or_init(|| Regex::new(r"\b(if|else if|while|for|loop|match)\b|&&|\|\|").unwrap())
}
fn rust_params_re() -> &'static Regex {
    RUST_PARAMS.get_or_init(|| Regex::new(r"(?:&(?:mut\s+)?self\s*,?\s*)?").unwrap())
}

fn extract_rust(source: &str, file: &str) -> Vec<FunctionInfo> {
    use super::test_detect;
    let lines: Vec<&str> = source.lines().collect();
    let cfg_test_ranges = test_detect::find_cfg_test_ranges(source);
    rust_fn_re()
        .captures_iter(source)
        .map(|cap| {
            let name = cap[1].to_string();
            let params_str = cap[2].to_string();
            let param_count = count_params_rust(&params_str);
            let line = byte_offset_to_line(source, cap.get(0).unwrap().start());
            let (line_count, complexity) = measure_block(&lines, line);
            let is_test = test_detect::is_test_rust(source, line, &cfg_test_ranges);
            FunctionInfo {
                name,
                file: file.to_string(),
                line,
                line_count,
                param_count,
                complexity,
                is_test,
            }
        })
        .collect()
}

fn count_params_rust(params: &str) -> usize {
    let cleaned = rust_params_re().replace(params, "");
    if cleaned.trim().is_empty() {
        return 0;
    }
    cleaned.split(',').filter(|p| !p.trim().is_empty()).count()
}

fn extract_python(source: &str, file: &str) -> Vec<FunctionInfo> {
    use super::test_detect;
    let lines: Vec<&str> = source.lines().collect();
    python_fn_re()
        .captures_iter(source)
        .map(|cap| {
            let name = cap[1].to_string();
            let params_str = &cap[2];
            let param_count = count_params_python(params_str);
            let line = byte_offset_to_line(source, cap.get(0).unwrap().start());
            let (line_count, complexity) = measure_block(&lines, line);
            let is_test = test_detect::is_test_python(&name, source, line);
            FunctionInfo {
                name,
                file: file.to_string(),
                line,
                line_count,
                param_count,
                complexity,
                is_test,
            }
        })
        .collect()
}

fn count_params_python(params: &str) -> usize {
    if params.trim().is_empty() {
        return 0;
    }
    params
        .split(',')
        .filter(|p| !p.trim().is_empty() && p.trim() != "self")
        .count()
}

fn extract_typescript(source: &str, file: &str) -> Vec<FunctionInfo> {
    use super::test_detect;
    let lines: Vec<&str> = source.lines().collect();
    ts_fn_re()
        .captures_iter(source)
        .map(|cap| {
            let name = cap[1].to_string();
            let param_count = if cap[2].trim().is_empty() {
                0
            } else {
                cap[2].split(',').filter(|p| !p.trim().is_empty()).count()
            };
            let line = byte_offset_to_line(source, cap.get(0).unwrap().start());
            let (line_count, complexity) = measure_block(&lines, line);
            let is_test = test_detect::is_test_typescript(&name, file);
            FunctionInfo {
                name,
                file: file.to_string(),
                line,
                line_count,
                param_count,
                complexity,
                is_test,
            }
        })
        .collect()
}

fn byte_offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset].chars().filter(|&c| c == '\n').count() + 1
}

fn measure_block(lines: &[&str], start_line: usize) -> (usize, usize) {
    let idx = start_line.saturating_sub(1);
    let line_count = count_block_lines(lines, idx);
    // Count complexity only within the function's actual lines
    let snippet: String = lines
        .iter()
        .skip(idx)
        .take(line_count)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let complexity = complexity_re().find_iter(&snippet).count().min(50) + 1;
    (line_count, complexity)
}

fn count_block_lines(lines: &[&str], start: usize) -> usize {
    let mut brace_depth: i32 = 0;
    let mut found_open = false;
    let mut count = 0;

    for line in lines.iter().skip(start) {
        count += 1;
        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
                found_open = true;
            } else if ch == '}' {
                brace_depth -= 1;
                // Closing brace of the function
                if found_open && brace_depth == 0 {
                    return count;
                }
            }
        }
        // Python: indentation-based (no braces)
        if !found_open && count > 1 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // For Python, check if we've returned to the same or lesser indent
            let base = lines
                .get(start)
                .map(|l| l.chars().take_while(|c| *c == ' ').count())
                .unwrap_or(0);
            let depth = line.chars().take_while(|c| *c == ' ').count();
            if depth <= base && !trimmed.starts_with('#') && !trimmed.starts_with("def ") {
                return count.saturating_sub(1).max(1);
            }
        }
        if count > 200 {
            break;
        }
    }
    count
}

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
    fn unknown_language_returns_empty() {
        let fns = scan_source("some content", "data.json", Language::Unknown);
        assert!(fns.is_empty());
    }

    #[test]
    fn complexity_includes_base_of_one() {
        let fns = scan_source(RUST_SRC, "src/risk.rs", Language::Rust);
        let stake = fns.iter().find(|f| f.name == "validate_stake").unwrap();
        assert!(stake.complexity >= 1);
    }
}
