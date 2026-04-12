use colored::Colorize;
use spec_store_core::{
    config::CoverageConfig,
    coverage::{checker::CheckResult, lcov::FileCoverage},
    store::baseline::BaselineStore,
};
use std::collections::{BTreeMap, HashMap};

pub struct ReportInput<'a> {
    pub coverage: &'a HashMap<String, FileCoverage>,
    pub results: &'a [CheckResult],
    pub config: &'a CoverageConfig,
    pub baselines: &'a BaselineStore,
    pub show_passing: bool,
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
        }
    }
}

// ── Text report (grouped by folder) ──────────────────────────────────

pub fn print_report(input: &ReportInput) {
    let date = chrono::Utc::now().format("%Y-%m-%d");
    println!("\n{}", format!("COVERAGE REPORT  {date}").bold());
    println!(
        "Threshold: {:.1}% per file | Ratchet: {}",
        input.config.min_per_file,
        if input.config.ratchet {
            "enabled".green()
        } else {
            "disabled".dimmed()
        }
    );

    let grouped = group_by_folder(input.results);

    for (folder, results) in &grouped {
        print_folder(folder, results, input);
    }

    println!("{}", "━".repeat(70));
    print_summary(input);
}

fn group_by_folder(results: &[CheckResult]) -> BTreeMap<String, Vec<&CheckResult>> {
    let mut groups: BTreeMap<String, Vec<&CheckResult>> = BTreeMap::new();
    for r in results {
        let path = r.file();
        let folder = match path.rfind('/') {
            Some(i) => &path[..i],
            None => ".",
        };
        groups.entry(folder.to_string()).or_default().push(r);
    }
    // Sort each group by coverage ascending
    for group in groups.values_mut() {
        group.sort_by(|a, b| {
            a.pct()
                .partial_cmp(&b.pct())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    groups
}

fn print_folder(folder: &str, results: &[&CheckResult], input: &ReportInput) {
    println!("\n  {}", folder.bold().dimmed());
    println!("  {:<30} {:>6}  {:>7}  STATUS", "FILE", "LINES", "COV");
    for result in results {
        if !input.show_passing && !result.is_failure() {
            continue;
        }
        let file = result.file();
        let filename = file.rsplit('/').next().unwrap_or(file);
        let lines = input.coverage.get(file).map(|f| f.lines_found).unwrap_or(0);
        let baseline = input.baselines.get(file);
        let (status, pct_str) = format_status(result, baseline);
        println!("  {:<30} {:>6}  {}  {}", filename, lines, pct_str, status);
    }
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
            format!("✗ < {required:.0}%").red().to_string(),
            format!("{pct:.1}%").red().to_string(),
        ),
        CheckResult::Regressed { pct, baseline, .. } => (
            format!("✗ was {baseline:.1}%").red().to_string(),
            format!("{pct:.1}%").red().to_string(),
        ),
    }
}

fn print_summary(input: &ReportInput) {
    let total_lines: u64 = input.coverage.values().map(|f| f.lines_found).sum();
    let total_hit: u64 = input.coverage.values().map(|f| f.lines_hit).sum();
    let overall = if total_lines > 0 {
        total_hit as f64 / total_lines as f64 * 100.0
    } else {
        100.0
    };
    let failures = input.results.iter().filter(|r| r.is_failure()).count();
    let passing = input.results.len() - failures;
    let status = if failures == 0 {
        "PASSED".green()
    } else {
        "BLOCKED".red()
    };
    println!(
        "{:<32} {:>6}  {:.1}%  {}  ({} pass, {} fail)",
        "OVERALL", total_lines, overall, status, passing, failures,
    );
}

// ── JSON report ──────────────────────────────────────────────────────

pub fn print_report_json(input: &ReportInput) {
    let files: Vec<serde_json::Value> = input
        .results
        .iter()
        .map(|r| {
            let baseline = input.baselines.get(r.file());
            serde_json::json!({
                "file": r.file(),
                "lines": input.coverage.get(r.file()).map(|f| f.lines_found).unwrap_or(0),
                "pct": format!("{:.1}", r.pct()),
                "status": match r {
                    CheckResult::Pass { .. } => "pass",
                    CheckResult::Legacy { .. } => "legacy",
                    CheckResult::BelowThreshold { .. } => "below_threshold",
                    CheckResult::Regressed { .. } => "regressed",
                },
                "is_failure": r.is_failure(),
                "baseline": baseline,
            })
        })
        .collect();

    let total_lines: u64 = input.coverage.values().map(|f| f.lines_found).sum();
    let total_hit: u64 = input.coverage.values().map(|f| f.lines_hit).sum();
    let overall = if total_lines > 0 {
        total_hit as f64 / total_lines as f64 * 100.0
    } else {
        100.0
    };

    let output = serde_json::json!({
        "threshold": input.config.min_per_file,
        "ratchet": input.config.ratchet,
        "overall_pct": format!("{:.1}", overall),
        "total_lines": total_lines,
        "total_hit": total_hit,
        "files": files,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

// ── Quality report ───────────────────────────────────────────────────

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

    #[test]
    fn group_by_folder_groups_correctly() {
        let results = vec![
            CheckResult::Pass {
                file: "src/a.rs".into(),
                pct: 90.0,
            },
            CheckResult::Pass {
                file: "src/b.rs".into(),
                pct: 85.0,
            },
            CheckResult::Pass {
                file: "tests/t.rs".into(),
                pct: 100.0,
            },
        ];
        let grouped = group_by_folder(&results);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped["src"].len(), 2);
        assert_eq!(grouped["tests"].len(), 1);
    }

    #[test]
    fn group_by_folder_sorts_by_coverage() {
        let results = vec![
            CheckResult::Pass {
                file: "src/high.rs".into(),
                pct: 99.0,
            },
            CheckResult::Pass {
                file: "src/low.rs".into(),
                pct: 50.0,
            },
        ];
        let grouped = group_by_folder(&results);
        assert!(grouped["src"][0].pct() < grouped["src"][1].pct());
    }
}
