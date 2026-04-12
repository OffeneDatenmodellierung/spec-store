pub mod quality;
pub mod regex_scanner;
pub mod test_detect;
pub mod test_mapper;

pub use quality::{check_dir, check_file, has_errors, FileViolation, Violation};
pub use regex_scanner::{detect_language, scan_file, scan_source, FunctionInfo, Language};

use std::path::Path;
use walkdir::WalkDir;

/// Scan all source files under `root` and return every function found.
pub fn scan_dir_functions(root: &Path) -> Vec<FunctionInfo> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("rs") | Some("py") | Some("ts") | Some("tsx")
            )
        })
        .flat_map(|e| scan_file(e.path()).unwrap_or_default())
        .collect()
}
