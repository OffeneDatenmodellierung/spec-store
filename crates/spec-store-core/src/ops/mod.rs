mod context_ops;
mod coverage_ops;
pub mod test_tracking;
mod worktree_ops;

pub use context_ops::{generate_context, project_rules, write_context};
pub use coverage_ops::{
    check_coverage, check_quality, check_quality_staged, scan_functions, update_baseline,
    CoverageReport,
};
pub use worktree_ops::{claim_worktree, list_worktrees, release_worktree, verify_worktrees};

use crate::{
    config, hooks,
    reuse::{self, SimilarityLevel},
    scanner::{self, FunctionInfo},
    store::structured::Decision,
    AppContext,
};
use std::path::PathBuf;

// ── Return types ──────────────────────────────────────────────────────

pub struct CatchupResult {
    pub missing: Vec<FunctionInfo>,
    pub total_scanned: usize,
}

pub struct StatusReport {
    pub function_count: usize,
    pub decision_count: usize,
    pub worktree_count: usize,
    pub baseline_count: usize,
    pub hooks_installed: bool,
}

// ── Core operations ───────────────────────────────────────────────────

pub fn init(root: &std::path::Path) -> anyhow::Result<()> {
    config::save_default(root)?;
    hooks::install(root, &hooks::HookSet::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

pub fn search(ctx: &AppContext, query: &str, limit: usize) -> Vec<crate::store::SearchResult> {
    let embedding = crate::store::embed_text(query);
    ctx.vectors.search(&embedding, limit)
}

pub fn register_fn(
    ctx: &mut AppContext,
    name: &str,
    file: &str,
    line: usize,
    desc: &str,
) -> anyhow::Result<String> {
    let id = ctx
        .structured
        .register_fn(&crate::store::structured::RegisterFnInput {
            name,
            file,
            line,
            desc,
            is_test: false,
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let info = FunctionInfo {
        name: name.into(),
        file: file.into(),
        line,
        line_count: 0,
        param_count: 0,
        complexity: 1,
        is_test: false,
    };
    reuse::register_function(&mut ctx.vectors, &info, desc);
    ctx.vectors.save().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(id)
}

pub fn add_decision(ctx: &mut AppContext, text: &str, tags: &[String]) -> anyhow::Result<String> {
    ctx.structured
        .add_decision(text, tags)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn list_decisions(ctx: &AppContext) -> anyhow::Result<Vec<Decision>> {
    ctx.structured
        .list_decisions()
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn reuse_check(ctx: &AppContext, func: &FunctionInfo, desc: &str) -> SimilarityLevel {
    let enforcer = reuse::ReuseEnforcer::new(&ctx.vectors, &ctx.config.reuse);
    enforcer.check(func, desc)
}

pub fn catchup(
    ctx: &AppContext,
    scan_path: Option<&str>,
    staged_only: bool,
) -> anyhow::Result<CatchupResult> {
    let existing = ctx
        .structured
        .list_functions()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let known: std::collections::HashSet<String> = existing
        .iter()
        .map(|f| format!("{}:{}", f.file, f.name))
        .collect();

    let found = if staged_only {
        scan_staged_functions(&ctx.root)
    } else {
        let path = scan_path
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.root.join("src"));
        scanner::scan_dir_functions(&path)
    };

    let total_scanned = found.len();
    let missing: Vec<_> = found
        .into_iter()
        .filter(|f| !known.contains(&format!("{}:{}", f.file, f.name)))
        .collect();
    Ok(CatchupResult {
        missing,
        total_scanned,
    })
}

fn scan_staged_functions(root: &std::path::Path) -> Vec<FunctionInfo> {
    crate::git::staged_files(root)
        .iter()
        .filter_map(|f| scanner::scan_file(&root.join(f)).ok())
        .flatten()
        .collect()
}

pub fn status(ctx: &AppContext) -> anyhow::Result<StatusReport> {
    let fns = ctx
        .structured
        .list_functions()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let decisions = ctx
        .structured
        .list_decisions()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let worktrees = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let baselines = ctx.baseline.all_entries().count();
    let hooks_installed = hooks::verify_hooks_installed(&ctx.root);
    Ok(StatusReport {
        function_count: fns.len(),
        decision_count: decisions.len(),
        worktree_count: worktrees.len(),
        baseline_count: baselines,
        hooks_installed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BaselineStore, LocalVectorStore, StructuredStore};

    fn test_context() -> AppContext {
        AppContext {
            root: PathBuf::from("."),
            config: config::Config::default(),
            structured: StructuredStore::open_in_memory().unwrap(),
            baseline: BaselineStore::new_empty(),
            vectors: LocalVectorStore::new_empty(),
        }
    }

    #[test]
    fn search_returns_results_after_register() {
        let mut ctx = test_context();
        register_fn(
            &mut ctx,
            "validate_stake",
            "src/risk.rs",
            42,
            "Validates stake limit",
        )
        .unwrap();
        let results = search(&ctx, "validate stake", 5);
        assert!(!results.is_empty());
    }

    #[test]
    fn search_empty_store_returns_empty() {
        let ctx = test_context();
        let results = search(&ctx, "anything", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn register_fn_adds_to_both_stores() {
        let mut ctx = test_context();
        let id = register_fn(&mut ctx, "foo", "src/foo.rs", 1, "does foo").unwrap();
        assert!(!id.is_empty());
        let fns = ctx.structured.list_functions().unwrap();
        assert_eq!(fns.len(), 1);
        assert!(!ctx.vectors.is_empty());
    }

    #[test]
    fn add_and_list_decisions() {
        let mut ctx = test_context();
        add_decision(&mut ctx, "Use HMAC", &["security".into()]).unwrap();
        let decisions = list_decisions(&ctx).unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].text.contains("HMAC"));
    }

    #[test]
    fn worktree_claim_release_list() {
        let mut ctx = test_context();
        claim_worktree(&mut ctx, "feat/auth", Some("auth.yaml"), Some("agent-1")).unwrap();
        let wts = list_worktrees(&ctx).unwrap();
        assert!(
            wts.iter().any(|w| w.branch == "feat/auth"),
            "claimed worktree should appear in list"
        );
        release_worktree(&mut ctx, "feat/auth").unwrap();
        let wts = list_worktrees(&ctx).unwrap();
        assert!(
            !wts.iter().any(|w| w.branch == "feat/auth"),
            "released worktree should not appear"
        );
    }

    #[test]
    fn status_report_counts() {
        let mut ctx = test_context();
        add_decision(&mut ctx, "Use JWT", &[]).unwrap();
        register_fn(&mut ctx, "bar", "src/bar.rs", 1, "bar fn").unwrap();
        let report = status(&ctx).unwrap();
        assert_eq!(report.decision_count, 1);
        assert_eq!(report.function_count, 1);
        assert_eq!(report.worktree_count, 0);
    }

    #[test]
    fn reuse_check_clear_on_empty_store() {
        let ctx = test_context();
        let func = FunctionInfo {
            name: "anything".into(),
            file: "src/x.rs".into(),
            line: 1,
            line_count: 5,
            param_count: 0,
            complexity: 1,
            is_test: false,
        };
        assert!(matches!(
            reuse_check(&ctx, &func, "does stuff"),
            SimilarityLevel::Clear
        ));
    }

    #[test]
    fn catchup_finds_missing_functions() {
        let ctx = test_context();
        let result = catchup(&ctx, Some("/nonexistent"), false).unwrap();
        assert!(result.missing.is_empty());
        assert_eq!(result.total_scanned, 0);
    }

    #[test]
    fn project_rules_includes_config_values() {
        let ctx = test_context();
        let rules = project_rules(&ctx).unwrap();
        assert!(rules.contains("300"), "should include max_file_lines");
        assert!(rules.contains("50"), "should include max_fn_lines");
        assert!(rules.contains("85"), "should include coverage threshold");
        assert!(rules.contains("0.95"), "should include similarity_block");
    }

    #[test]
    fn project_rules_includes_decisions() {
        let mut ctx = test_context();
        add_decision(&mut ctx, "Use JWT for auth", &[]).unwrap();
        let rules = project_rules(&ctx).unwrap();
        assert!(rules.contains("JWT"));
        assert!(rules.contains("Architectural Decisions"));
    }

    #[test]
    fn project_rules_includes_worktrees() {
        let mut ctx = test_context();
        claim_worktree(&mut ctx, "feat/auth", None, Some("agent-1")).unwrap();
        let rules = project_rules(&ctx).unwrap();
        assert!(rules.contains("feat/auth"));
        assert!(rules.contains("agent-1"));
    }
}
