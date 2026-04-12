use crate::{
    config::CoverageConfig,
    coverage::lcov::FileCoverage,
    error::{Result, SpecStoreError},
    store::baseline::BaselineStore,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum CheckResult {
    Pass {
        file: String,
        pct: f64,
    },
    Legacy {
        file: String,
        pct: f64,
        baseline: f64,
    },
    BelowThreshold {
        file: String,
        pct: f64,
        required: f64,
    },
    Regressed {
        file: String,
        pct: f64,
        baseline: f64,
    },
}

impl CheckResult {
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::BelowThreshold { .. } | Self::Regressed { .. })
    }

    pub fn file(&self) -> &str {
        match self {
            Self::Pass { file, .. }
            | Self::Legacy { file, .. }
            | Self::BelowThreshold { file, .. }
            | Self::Regressed { file, .. } => file,
        }
    }

    pub fn pct(&self) -> f64 {
        match self {
            Self::Pass { pct, .. }
            | Self::Legacy { pct, .. }
            | Self::BelowThreshold { pct, .. }
            | Self::Regressed { pct, .. } => *pct,
        }
    }
}

pub fn check_all(
    coverage: &HashMap<String, FileCoverage>,
    config: &CoverageConfig,
    baselines: &BaselineStore,
) -> Vec<CheckResult> {
    coverage
        .values()
        .filter(|fc| !is_excluded(&fc.path, &config.exclude))
        .map(|fc| check_one(fc, config, baselines))
        .collect()
}

fn check_one(fc: &FileCoverage, config: &CoverageConfig, baselines: &BaselineStore) -> CheckResult {
    let pct = fc.percentage();
    let file = fc.path.clone();

    match baselines.get(&file) {
        None => {
            if pct >= config.min_per_file {
                CheckResult::Pass { file, pct }
            } else {
                CheckResult::BelowThreshold {
                    file,
                    pct,
                    required: config.min_per_file,
                }
            }
        }
        Some(baseline) => {
            if config.ratchet && config.fail_on_regression && pct < baseline - f64::EPSILON {
                return CheckResult::Regressed {
                    file,
                    pct,
                    baseline,
                };
            }
            if pct >= config.min_per_file {
                CheckResult::Pass { file, pct }
            } else {
                // Below threshold but has a legacy baseline — don't fail
                CheckResult::Legacy {
                    file,
                    pct,
                    baseline,
                }
            }
        }
    }
}

pub fn assert_no_failures(results: &[CheckResult]) -> Result<()> {
    let failures: Vec<_> = results.iter().filter(|r| r.is_failure()).collect();
    if failures.is_empty() {
        return Ok(());
    }
    let detail = failures
        .iter()
        .map(|r| format!("  {}: {:.1}%", r.file(), r.pct()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(SpecStoreError::Coverage(format!(
        "{} file(s) failed coverage gate:\n{detail}",
        failures.len()
    )))
}

fn is_excluded(path: &str, patterns: &[String]) -> bool {
    crate::util::is_excluded(path, patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CoverageConfig;

    fn cfg() -> CoverageConfig {
        CoverageConfig {
            min_per_file: 85.0,
            lcov_path: "lcov.info".into(),
            lcov_max_age_mins: 60,
            ratchet: true,
            fail_on_regression: true,
            exclude: vec!["src/generated/**".into()],
        }
    }

    fn fc(path: &str, found: u64, hit: u64) -> FileCoverage {
        FileCoverage {
            path: path.into(),
            lines_found: found,
            lines_hit: hit,
        }
    }

    fn map(items: &[FileCoverage]) -> HashMap<String, FileCoverage> {
        items.iter().map(|f| (f.path.clone(), f.clone())).collect()
    }

    #[test]
    fn new_file_above_threshold_passes() {
        let r = check_all(
            &map(&[fc("src/a.rs", 100, 90)]),
            &cfg(),
            &BaselineStore::new_empty(),
        );
        assert!(r.iter().all(|x| !x.is_failure()));
    }

    #[test]
    fn new_file_below_threshold_fails() {
        let r = check_all(
            &map(&[fc("src/a.rs", 100, 80)]),
            &cfg(),
            &BaselineStore::new_empty(),
        );
        assert!(r.iter().any(|x| x.is_failure()));
    }

    #[test]
    fn regression_below_baseline_fails() {
        let mut bl = BaselineStore::new_empty();
        bl.set("src/a.rs", 88.0);
        let r = check_all(&map(&[fc("src/a.rs", 100, 82)]), &cfg(), &bl);
        assert!(matches!(&r[0], CheckResult::Regressed { .. }));
    }

    #[test]
    fn legacy_file_below_threshold_but_not_regressed_does_not_fail() {
        let mut bl = BaselineStore::new_empty();
        bl.set("src/a.rs", 70.0);
        let r = check_all(&map(&[fc("src/a.rs", 100, 72)]), &cfg(), &bl);
        assert!(matches!(&r[0], CheckResult::Legacy { .. }));
        assert!(!r[0].is_failure());
    }

    #[test]
    fn excluded_files_not_checked() {
        let r = check_all(
            &map(&[fc("src/generated/schema.rs", 100, 0)]),
            &cfg(),
            &BaselineStore::new_empty(),
        );
        assert!(r.is_empty());
    }

    #[test]
    fn assert_no_failures_ok_when_all_pass() {
        let results = vec![CheckResult::Pass {
            file: "a.rs".into(),
            pct: 90.0,
        }];
        assert!(assert_no_failures(&results).is_ok());
    }

    #[test]
    fn assert_no_failures_err_when_failures_exist() {
        let results = vec![CheckResult::BelowThreshold {
            file: "a.rs".into(),
            pct: 70.0,
            required: 85.0,
        }];
        assert!(assert_no_failures(&results).is_err());
    }

    #[test]
    fn check_result_pct_accessor_works() {
        let r = CheckResult::Pass {
            file: "x.rs".into(),
            pct: 91.2,
        };
        assert!((r.pct() - 91.2).abs() < 0.01);
    }
}
