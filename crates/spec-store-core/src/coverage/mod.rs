pub mod checker;
pub mod fn_coverage;
pub mod lcov;

pub use checker::{assert_no_failures, check_all, CheckResult};
pub use fn_coverage::{compute_fn_coverage, FnCoverageResult};
pub use lcov::{parse, parse_content, FileCoverage, LineCoverage};
