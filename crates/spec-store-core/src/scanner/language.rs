//! Language detection and per-language scanner profiles.
//!
//! Each [`LanguageProfile`] bundles the file extensions, function-detection
//! regexes, comment-line prefixes and doc-block delimiters for one language.
//! Adding a new language is a matter of adding an entry to [`ALL`] — the
//! scanner, quality gate and code-line counter pick it up automatically.

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageProfile {
    pub language: Language,
    pub extensions: &'static [&'static str],
    pub function_patterns: &'static [&'static str],
    pub comment_line_prefixes: &'static [&'static str],
    pub doc_block_delimiters: &'static [(&'static str, &'static str)],
}

const RUST: LanguageProfile = LanguageProfile {
    language: Language::Rust,
    extensions: &["rs"],
    function_patterns: &[
        r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)",
    ],
    comment_line_prefixes: &["//"],
    doc_block_delimiters: &[("/*", "*/")],
};

const PYTHON: LanguageProfile = LanguageProfile {
    language: Language::Python,
    extensions: &["py"],
    function_patterns: &[r"(?m)^(?:    )*(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)"],
    comment_line_prefixes: &["#"],
    doc_block_delimiters: &[("\"\"\"", "\"\"\""), ("'''", "'''")],
};

const TYPESCRIPT: LanguageProfile = LanguageProfile {
    language: Language::TypeScript,
    extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
    function_patterns: &[
        r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)",
        r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*(?::[^=]+)?=\s*(?:async\s+)?\(([^)]*)\)\s*(?::[^={]+)?=>",
    ],
    comment_line_prefixes: &["//"],
    doc_block_delimiters: &[("/*", "*/")],
};

const ALL: &[&LanguageProfile] = &[&RUST, &PYTHON, &TYPESCRIPT];

pub fn detect_language(path: &Path) -> Language {
    profile_for_path(path)
        .map(|p| p.language)
        .unwrap_or(Language::Unknown)
}

pub fn profile_for(lang: Language) -> Option<&'static LanguageProfile> {
    ALL.iter().copied().find(|p| p.language == lang)
}

pub fn profile_for_path(path: &Path) -> Option<&'static LanguageProfile> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    ALL.iter().copied().find(|p| p.extensions.contains(&ext))
}

pub fn is_source_path(path: &Path) -> bool {
    profile_for_path(path).is_some()
}

static COMPILED: OnceLock<HashMap<Language, Vec<Regex>>> = OnceLock::new();

pub fn compiled_patterns(lang: Language) -> &'static [Regex] {
    let map = COMPILED.get_or_init(|| {
        ALL.iter()
            .map(|p| {
                let res: Vec<Regex> = p
                    .function_patterns
                    .iter()
                    .map(|s| Regex::new(s).expect("valid language regex"))
                    .collect();
                (p.language, res)
            })
            .collect()
    });
    map.get(&lang).map(|v| v.as_slice()).unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_extensions() {
        assert_eq!(detect_language(Path::new("a.rs")), Language::Rust);
        assert_eq!(detect_language(Path::new("a.py")), Language::Python);
        assert_eq!(detect_language(Path::new("a.ts")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.tsx")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.js")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.jsx")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.mjs")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.cjs")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("a.json")), Language::Unknown);
    }

    #[test]
    fn is_source_path_recognises_supported() {
        assert!(is_source_path(Path::new("src/lib.rs")));
        assert!(is_source_path(Path::new("app/main.py")));
        assert!(is_source_path(Path::new("ui/button.tsx")));
        assert!(is_source_path(Path::new("ui/button.jsx")));
        assert!(!is_source_path(Path::new("README.md")));
    }

    #[test]
    fn profile_lookup_by_language() {
        assert_eq!(
            profile_for(Language::Rust).unwrap().language,
            Language::Rust
        );
        assert!(profile_for(Language::Unknown).is_none());
    }

    #[test]
    fn typescript_profile_compiles_two_patterns() {
        let pats = compiled_patterns(Language::TypeScript);
        assert_eq!(pats.len(), 2);
    }

    #[test]
    fn unknown_language_has_no_patterns() {
        assert!(compiled_patterns(Language::Unknown).is_empty());
    }

    #[test]
    fn python_doc_block_delimiters_include_both_quote_styles() {
        let p = profile_for(Language::Python).unwrap();
        assert_eq!(p.doc_block_delimiters.len(), 2);
    }
}
