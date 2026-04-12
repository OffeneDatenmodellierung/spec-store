use colored::Colorize;
use spec_store_core::{
    config::CoverageConfig,
    coverage::{checker::CheckResult, lcov::FileCoverage},
    store::baseline::BaselineStore,
};
use std::collections::HashMap;

pub struct ReportInput<'a> {
    pub coverage: &'a HashMap<String, FileCoverage>,
    pub results: &'a [CheckResult],
    pub config: &'a CoverageConfig,
    pub baselines: &'a BaselineStore,
    pub show_passing: bool,
    pub sort_by_pct: bool,
}

impl<'a> ReportInput<'a> {
    pub fn new(
        coverage: &'a HashMap<String, FileCoverage>,
        results: &'a [CheckResult],
        config: &'a CoverageConfig,
        baselines: &'a BaselineStore,
    ) -> Self {
        Self {
            coverage,
            results,
            config,
            baselines,
            show_passing: true,
            sort_by_pct: true,
        }
    }
}

pub fn print_report(input: &ReportInput) {
    let ReportInput {
        coverage,
        results,
        config,
        baselines,
        show_passing,
        sort_by_pct,
    } = input;
    let date = chrono::Utc::now().format("%Y-%m-%d");
    println!("\n{}", format!("COVERAGE REPORT  {date}").bold());
    println!(
        "Threshold: {:.1}% per file | Ratchet: {}",
        config.min_per_file,
        if config.ratchet {
            "enabled".green()
        } else {
            "disabled".dimmed()
        }
    );
    println!("{}", "━".repeat(60));
    println!("{:<40} {:>6}  {:>7}  STATUS", "FILE", "LINES", "COV");
    println!("{}", "━".repeat(60));

    let mut sorted_results: Vec<_> = results.iter().collect();
    if *sort_by_pct {
        sorted_results.sort_by(|a, b| {
            a.pct()
                .partial_cmp(&b.pct())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        sorted_results.sort_by(|a, b| a.file().cmp(b.file()));
    }

    for result in &sorted_results {
        if !show_passing && !result.is_failure() {
            continue;
        }
        print_row(result, coverage, baselines);
    }

    println!("{}", "━".repeat(60));
    print_summary(results, coverage, config);
}

fn print_row(
    result: &CheckResult,
    coverage: &HashMap<String, FileCoverage>,
    baselines: &BaselineStore,
) {
    let file = result.file();
    let display = truncate_path(file, 38);
    let lines = coverage.get(file).map(|f| f.lines_found).unwrap_or(0);
    let baseline = baselines.get(file);
    let (status, pct_str) = format_status(result, baseline);
    println!("{:<40} {:>6}  {}  {}", display, lines, pct_str, status);
}

fn format_status(result: &CheckResult, baseline: Option<f64>) -> (String, String) {
    match result {
        CheckResult::Pass { pct, .. } => {
            let delta = baseline
                .map(|b| format!(" +{:.1}%", pct - b))
                .unwrap_or_default();
            (
                format!("{}{}", "✓".green(), delta.dimmed()),
                format!("{pct:.1}%").green().to_string(),
            )
        }
        CheckResult::Legacy { pct, .. } => (
            "⚠ legacy".yellow().to_string(),
            format!("{pct:.1}%").yellow().to_string(),
        ),
        CheckResult::BelowThreshold { pct, required, .. } => (
            format!("✗ NEW < {required:.0}%").red().to_string(),
            format!("{pct:.1}%").red().to_string(),
        ),
        CheckResult::Regressed { pct, baseline, .. } => (
            format!("✗ REGRESSED (was {baseline:.1}%)")
                .red()
                .to_string(),
            format!("{pct:.1}%").red().to_string(),
        ),
    }
}

fn print_summary(
    results: &[CheckResult],
    coverage: &HashMap<String, FileCoverage>,
    _config: &CoverageConfig,
) {
    let total_lines: u64 = coverage.values().map(|f| f.lines_found).sum();
    let total_hit: u64 = coverage.values().map(|f| f.lines_hit).sum();
    let overall = if total_lines > 0 {
        total_hit as f64 / total_lines as f64 * 100.0
    } else {
        100.0
    };
    let failures = results.iter().filter(|r| r.is_failure()).count();
    let status = if failures == 0 {
        "PASSED".green()
    } else {
        "BLOCKED".red()
    };
    println!(
        "{:<40} {:>6}  {:.1}%  {}",
        "OVERALL", total_lines, overall, status
    );
    if failures > 0 {
        println!(
            "\n{} file(s) blocked. Run {} for details.",
            failures.to_string().red(),
            "spec-store coverage explain <file>".italic()
        );
    }
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    format!("…{}", &path[path.len().saturating_sub(max_len - 1)..])
}

pub fn print_quality_report(violations: &[spec_store_core::scanner::FileViolation]) {
    println!("\n{}", "QUALITY GATES".bold());
    println!("{}", "━".repeat(60));
    let mut any = false;
    for fv in violations {
        if fv.violations.is_empty() {
            continue;
        }
        any = true;
        println!("\n{}", fv.file.bold());
        for v in &fv.violations {
            let prefix = if v.is_warning {
                "  ⚠".yellow()
            } else {
                "  ✗".red()
            };
            println!("{} {}", prefix, v.message);
        }
    }
    if !any {
        println!("{}", "  All files pass quality gates.".green());
    }
    println!("{}", "━".repeat(60));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_path_short_path_unchanged() {
        assert_eq!(truncate_path("src/foo.rs", 40), "src/foo.rs");
    }

    #[test]
    fn truncate_path_long_path_truncated() {
        let long = "src/very/deeply/nested/module/submodule/file.rs";
        let result = truncate_path(long, 20);
        assert!(result.starts_with('…'));
        assert!(result.chars().count() <= 20);
    }

    #[test]
    fn format_status_pass_shows_tick() {
        let r = CheckResult::Pass {
            file: "x.rs".into(),
            pct: 90.0,
        };
        let (status, _) = format_status(&r, None);
        assert!(status.contains('✓'));
    }

    #[test]
    fn format_status_below_threshold_shows_cross() {
        let r = CheckResult::BelowThreshold {
            file: "x.rs".into(),
            pct: 70.0,
            required: 85.0,
        };
        let (status, _) = format_status(&r, None);
        assert!(status.contains('✗'));
    }

    #[test]
    fn format_status_regression_shows_cross() {
        let r = CheckResult::Regressed {
            file: "x.rs".into(),
            pct: 80.0,
            baseline: 90.0,
        };
        let (status, _) = format_status(&r, None);
        assert!(status.contains('✗'));
    }

    #[test]
    fn format_status_legacy_shows_warning() {
        let r = CheckResult::Legacy {
            file: "x.rs".into(),
            pct: 70.0,
            baseline: 70.0,
        };
        let (status, _) = format_status(&r, None);
        assert!(status.contains('⚠'));
    }
}
