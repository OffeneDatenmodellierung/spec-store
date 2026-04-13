use crate::error::{Result, SpecStoreError};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct FileCoverage {
    pub path: String,
    pub lines_found: u64,
    pub lines_hit: u64,
}

impl FileCoverage {
    pub fn percentage(&self) -> f64 {
        if self.lines_found == 0 {
            return 100.0;
        }
        (self.lines_hit as f64 / self.lines_found as f64) * 100.0
    }
}

pub fn parse(lcov_path: &Path) -> Result<HashMap<String, FileCoverage>> {
    let content = fs::read_to_string(lcov_path).map_err(|e| {
        SpecStoreError::Coverage(format!("Cannot read {}: {e}", lcov_path.display()))
    })?;
    // Determine project root: parent of the lcov file, or cwd
    let root = lcov_path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_default();
    let mut results = parse_content(&content)?;
    relativise_paths(&mut results, &root);
    Ok(results)
}

fn relativise_paths(results: &mut HashMap<String, FileCoverage>, root: &Path) {
    let root_str = format!("{}/", root.display());
    let updated: Vec<(String, FileCoverage)> = results
        .drain()
        .map(|(k, mut v)| {
            let rel = k.strip_prefix(&root_str).unwrap_or(&k).to_string();
            v.path = rel.clone();
            (rel, v)
        })
        .collect();
    results.extend(updated);
}

pub fn parse_content(content: &str) -> Result<HashMap<String, FileCoverage>> {
    let mut results = HashMap::new();
    let mut cur_file: Option<String> = None;
    let mut lf = 0u64;
    let mut lh = 0u64;

    for line in content.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("SF:") {
            cur_file = Some(path.to_string());
            lf = 0;
            lh = 0;
        } else if let Some(val) = line.strip_prefix("LF:") {
            lf = val.parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("LH:") {
            lh = val.parse().unwrap_or(0);
        } else if line == "end_of_record" {
            if let Some(path) = cur_file.take() {
                results.insert(
                    path.clone(),
                    FileCoverage {
                        path,
                        lines_found: lf,
                        lines_hit: lh,
                    },
                );
            }
        }
    }
    Ok(results)
}

/// Returns Err if the lcov file is older than `max_age_mins`.
pub fn check_age(lcov_path: &Path, max_age_mins: u64) -> Result<()> {
    let meta = fs::metadata(lcov_path).map_err(|_| {
        SpecStoreError::Coverage(format!(
            "lcov file not found: {}\nRun your test suite first.",
            lcov_path.display()
        ))
    })?;
    let modified = meta
        .modified()
        .map_err(|e| SpecStoreError::Coverage(e.to_string()))?;
    let age_secs = modified
        .elapsed()
        .map_err(|e| SpecStoreError::Coverage(e.to_string()))?
        .as_secs();
    if age_secs > max_age_mins * 60 {
        return Err(SpecStoreError::Coverage(format!(
            "lcov.info is {} min old (max {max_age_mins}). Run your test suite first.",
            age_secs / 60
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LineCoverage {
    pub line: usize,
    pub hits: u64,
}

/// Parse LCOV content and extract per-line coverage data (DA lines).
pub fn parse_detail(lcov_path: &Path) -> Result<HashMap<String, Vec<LineCoverage>>> {
    let content = fs::read_to_string(lcov_path).map_err(|e| {
        SpecStoreError::Coverage(format!("Cannot read {}: {e}", lcov_path.display()))
    })?;
    let root = lcov_path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_default();
    let root_str = format!("{}/", root.display());
    let mut results = parse_detail_content(&content)?;
    // Relativise keys
    let updated: Vec<(String, Vec<LineCoverage>)> = results
        .drain()
        .map(|(k, v)| {
            let rel = k.strip_prefix(&root_str).unwrap_or(&k).to_string();
            (rel, v)
        })
        .collect();
    results.extend(updated);
    Ok(results)
}

pub fn parse_detail_content(content: &str) -> Result<HashMap<String, Vec<LineCoverage>>> {
    let mut results: HashMap<String, Vec<LineCoverage>> = HashMap::new();
    let mut cur_file: Option<String> = None;
    let mut lines = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("SF:") {
            cur_file = Some(path.to_string());
            lines.clear();
        } else if let Some(da) = line.strip_prefix("DA:") {
            if let Some((line_str, hits_str)) = da.split_once(',') {
                if let (Ok(ln), Ok(hits)) = (line_str.parse(), hits_str.parse()) {
                    lines.push(LineCoverage { line: ln, hits });
                }
            }
        } else if line == "end_of_record" {
            if let Some(path) = cur_file.take() {
                results.insert(path, std::mem::take(&mut lines));
            }
        }
    }
    Ok(results)
}

/// Convert coverage map to a simpler file → percentage map.
pub fn to_percentage_map(coverage: &HashMap<String, FileCoverage>) -> HashMap<String, f64> {
    coverage
        .iter()
        .map(|(k, v)| (k.clone(), v.percentage()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "
SF:src/risk/validator.rs
LF:142
LH:128
end_of_record
SF:src/spend/accumulator.rs
LF:198
LH:173
end_of_record
SF:src/config/loader.rs
LF:201
LH:157
end_of_record
";

    #[test]
    fn parse_returns_correct_file_count() {
        assert_eq!(parse_content(SAMPLE).unwrap().len(), 3);
    }

    #[test]
    fn percentage_calculated_correctly() {
        let r = parse_content(SAMPLE).unwrap();
        let pct = r["src/risk/validator.rs"].percentage();
        assert!((pct - (128.0 / 142.0 * 100.0)).abs() < 0.01);
    }

    #[test]
    fn zero_lines_found_returns_100_percent() {
        let fc = FileCoverage {
            path: "x.rs".into(),
            lines_found: 0,
            lines_hit: 0,
        };
        assert_eq!(fc.percentage(), 100.0);
    }

    #[test]
    fn parse_handles_empty_input() {
        assert!(parse_content("").unwrap().is_empty());
    }

    #[test]
    fn parse_ignores_incomplete_record() {
        let incomplete = "SF:src/foo.rs\nLF:10\n";
        assert!(parse_content(incomplete).unwrap().is_empty());
    }

    #[test]
    fn parse_handles_invalid_numbers_gracefully() {
        let bad = "SF:src/foo.rs\nLF:oops\nLH:5\nend_of_record\n";
        let r = parse_content(bad).unwrap();
        assert_eq!(r["src/foo.rs"].lines_found, 0);
        assert_eq!(r["src/foo.rs"].lines_hit, 5);
    }

    #[test]
    fn to_percentage_map_produces_correct_values() {
        let r = parse_content(SAMPLE).unwrap();
        let map = to_percentage_map(&r);
        assert_eq!(map.len(), 3);
        assert!(map.values().all(|&v| v > 0.0));
    }

    #[test]
    fn check_age_fails_for_missing_file() {
        let result = check_age(Path::new("/nonexistent/lcov.info"), 60);
        assert!(result.is_err());
    }

    #[test]
    fn check_age_passes_for_fresh_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("lcov.info");
        std::fs::write(&path, "SF:a.rs\nLF:1\nLH:1\nend_of_record\n").unwrap();
        assert!(check_age(&path, 60).is_ok());
    }

    #[test]
    fn parse_reads_file_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("lcov.info");
        std::fs::write(&path, SAMPLE).unwrap();
        let result = parse(&path).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn parse_fails_for_missing_file() {
        assert!(parse(Path::new("/nonexistent/lcov.info")).is_err());
    }

    #[test]
    fn parse_detail_content_extracts_da_lines() {
        let content = "SF:src/a.rs\nDA:1,5\nDA:2,0\nDA:3,1\nLF:3\nLH:2\nend_of_record\n";
        let result = parse_detail_content(content).unwrap();
        let lines = &result["src/a.rs"];
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].hits, 5);
        assert_eq!(lines[1].hits, 0);
    }

    #[test]
    fn parse_detail_content_handles_empty() {
        assert!(parse_detail_content("").unwrap().is_empty());
    }

    #[test]
    fn parse_detail_content_handles_invalid_da() {
        let content = "SF:a.rs\nDA:bad,data\nDA:1,5\nend_of_record\n";
        let result = parse_detail_content(content).unwrap();
        assert_eq!(result["a.rs"].len(), 1);
    }

    #[test]
    fn parse_detail_reads_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("lcov.info");
        std::fs::write(&path, "SF:a.rs\nDA:1,1\nend_of_record\n").unwrap();
        let result = parse_detail(&path).unwrap();
        assert_eq!(result["a.rs"].len(), 1);
    }
}
