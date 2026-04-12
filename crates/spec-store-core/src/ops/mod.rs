pub mod test_tracking;

use crate::{
    config,
    context::{self, ContextOptions},
    coverage::{checker, lcov, CheckResult, FileCoverage},
    hooks,
    reuse::{self, SimilarityLevel},
    scanner::{self, quality, FileViolation, FunctionInfo},
    store::structured::{Decision, Worktree},
    AppContext,
};
use std::collections::HashMap;
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

pub struct CoverageReport {
    pub coverage: HashMap<String, FileCoverage>,
    pub results: Vec<CheckResult>,
}

// ── Operations ────────────────────────────────────────────────────────

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
        .register_fn(name, file, line, desc, false)
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

pub fn check_coverage(ctx: &AppContext, lcov_from: Option<&str>) -> anyhow::Result<CoverageReport> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let coverage = lcov::parse(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let results = checker::check_all(&coverage, &ctx.config.coverage, &ctx.baseline);
    Ok(CoverageReport { coverage, results })
}

pub fn update_baseline(ctx: &mut AppContext, lcov_from: Option<&str>) -> anyhow::Result<usize> {
    let lcov_path = ctx
        .root
        .join(lcov_from.unwrap_or(&ctx.config.coverage.lcov_path));
    let coverage = lcov::parse(&lcov_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let pct_map = lcov::to_percentage_map(&coverage);
    let count = pct_map.len();
    ctx.baseline.update_from_map(&pct_map);
    ctx.baseline.save().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(count)
}

pub fn check_quality(
    ctx: &AppContext,
    path: Option<&str>,
    file: Option<&str>,
) -> anyhow::Result<Vec<FileViolation>> {
    let target = file
        .map(PathBuf::from)
        .or_else(|| path.map(PathBuf::from))
        .unwrap_or_else(|| ctx.root.join("src"));
    if target.is_file() {
        Ok(vec![quality::check_file(&target, &ctx.config.quality)
            .map_err(|e| anyhow::anyhow!("{e}"))?])
    } else {
        quality::check_dir(&target, &ctx.config.quality).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

pub fn check_quality_staged(ctx: &AppContext) -> anyhow::Result<Vec<FileViolation>> {
    let staged = crate::git::staged_files(&ctx.root);
    let mut violations = Vec::new();
    for file in &staged {
        let path = ctx.root.join(file);
        if path.is_file() && scanner::detect_language(&path) != scanner::Language::Unknown {
            violations.push(
                quality::check_file(&path, &ctx.config.quality)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            );
        }
    }
    Ok(violations)
}

pub fn scan_functions(path: &std::path::Path) -> Vec<FunctionInfo> {
    scanner::scan_dir_functions(path)
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

fn scan_staged_functions(root: &std::path::Path) -> Vec<scanner::FunctionInfo> {
    let staged = crate::git::staged_files(root);
    staged
        .iter()
        .filter_map(|f| {
            let path = root.join(f);
            scanner::scan_file(&path).ok()
        })
        .flatten()
        .collect()
}

pub fn claim_worktree(
    ctx: &mut AppContext,
    branch: &str,
    contract: Option<&str>,
    owner: Option<&str>,
) -> anyhow::Result<()> {
    ctx.structured
        .claim_worktree(branch, contract, owner)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn release_worktree(ctx: &mut AppContext, branch: &str) -> anyhow::Result<()> {
    ctx.structured
        .release_worktree(branch)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn list_worktrees(ctx: &AppContext) -> anyhow::Result<Vec<Worktree>> {
    let mut claimed = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Also detect git worktrees not tracked by spec-store
    let claimed_branches: std::collections::HashSet<String> =
        claimed.iter().map(|w| w.branch.clone()).collect();

    if let Ok(git_worktrees) = detect_git_worktrees(&ctx.root) {
        for gw in git_worktrees {
            if !claimed_branches.contains(&gw.branch) {
                claimed.push(gw);
            }
        }
    }
    Ok(claimed)
}

fn detect_git_worktrees(root: &std::path::Path) -> anyhow::Result<Vec<Worktree>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if let Some(branch_ref) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch_ref.to_string());
        } else if line.is_empty() {
            if let Some(branch) = current_branch.take() {
                worktrees.push(Worktree {
                    branch,
                    contract: None,
                    owner: Some("git-worktree".into()),
                    claimed_at: String::new(),
                });
            }
        }
    }
    // Handle last entry if no trailing blank line
    if let Some(branch) = current_branch {
        worktrees.push(Worktree {
            branch,
            contract: None,
            owner: Some("git-worktree".into()),
            claimed_at: String::new(),
        });
    }
    Ok(worktrees)
}

/// Verify no staged files conflict with other worktree claims.
pub fn verify_worktrees(ctx: &AppContext) -> anyhow::Result<Vec<String>> {
    let worktrees = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let staged = crate::git::staged_files(&ctx.root);
    let current = crate::git::current_branch(&ctx.root);
    Ok(crate::git::staged_files_conflict_with_worktrees(
        &staged,
        &worktrees,
        current.as_deref(),
    ))
}

pub fn reuse_check(ctx: &AppContext, func: &FunctionInfo, desc: &str) -> SimilarityLevel {
    let enforcer = reuse::ReuseEnforcer::new(&ctx.vectors, &ctx.config.reuse);
    enforcer.check(func, desc)
}

pub fn generate_context(ctx: &AppContext, opts: &ContextOptions) -> anyhow::Result<String> {
    context::generate(&ctx.root, &ctx.structured, &ctx.baseline, opts)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn write_context(ctx: &AppContext, content: &str, output: &str) -> anyhow::Result<()> {
    context::write(&ctx.root, content, output).map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn project_rules(ctx: &AppContext) -> anyhow::Result<String> {
    let cfg = &ctx.config;
    let decisions = ctx
        .structured
        .list_decisions()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let worktrees = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut out = String::new();

    // Project description
    out.push_str("# spec-store — Project Rules\n\n");
    out.push_str(
        "`spec-store` is a Rust workspace that maintains a codebase specification registry \
                  with semantic search, quality gates, and multi-agent worktree coordination.\n\n",
    );

    // Hard rules from live config
    out.push_str("## Hard Rules (enforced by gates — do not bypass)\n\n");
    out.push_str("| Rule | Limit |\n|------|-------|\n");
    out.push_str(&format!(
        "| Lines per file (code) | {} |\n",
        cfg.quality.max_file_lines
    ));
    out.push_str(&format!(
        "| Lines per function | {} |\n",
        cfg.quality.max_fn_lines
    ));
    out.push_str(&format!(
        "| Functions per file | {} |\n",
        cfg.quality.max_fns_per_file
    ));
    out.push_str(&format!(
        "| Cyclomatic complexity | {} |\n",
        cfg.quality.max_fn_complexity
    ));
    out.push_str(&format!(
        "| Parameters per function | {} |\n",
        cfg.quality.max_fn_params
    ));
    out.push_str(&format!(
        "| Test coverage per file | {:.0}% |\n",
        cfg.coverage.min_per_file
    ));
    out.push_str(&format!(
        "| Similarity to existing fn | < {:.2} (blocked), < {:.2} (warn) |\n\n",
        cfg.reuse.similarity_block, cfg.reuse.similarity_warn
    ));

    // Workflow
    out.push_str("## Before Writing Any New Function\n\n");
    out.push_str("1. Use the `search` tool with a description of what you intend to write\n");
    out.push_str("2. If similarity ≥ 0.85, extend the existing function instead\n");
    out.push_str("3. Use `reuse_check` to verify your new function won't be blocked\n\n");

    // Module map
    out.push_str("## Workspace Structure\n\n```\ncrates/\n");
    out.push_str("  spec-store-core/   — library: stores, scanners, coverage, quality gates\n");
    out.push_str("  spec-store-cli/    — thin CLI for CI/hooks (binary: spec-store)\n");
    out.push_str("  spec-store-mcp/    — MCP server for agent integration\n```\n\n");

    // Decisions from store
    if !decisions.is_empty() {
        out.push_str("## Architectural Decisions\n\n");
        for d in decisions.iter().take(15) {
            let date = &d.created_at[..10];
            out.push_str(&format!("- [{date}] {}\n", d.text));
        }
        if decisions.len() > 15 {
            out.push_str(&format!(
                "\n*{} more — use `list_decisions` to see all.*\n",
                decisions.len() - 15
            ));
        }
        out.push('\n');
    }

    // Active worktrees
    if !worktrees.is_empty() {
        out.push_str("## Active Worktrees (do not modify files owned by others)\n\n");
        for wt in &worktrees {
            let owner = wt.owner.as_deref().unwrap_or("unassigned");
            out.push_str(&format!("- `{}` ({})", wt.branch, owner));
            if let Some(c) = &wt.contract {
                out.push_str(&format!(" → `{c}`"));
            }
            out.push('\n');
        }
        out.push('\n');
    }

    // Development workflow
    out.push_str("## Development Workflow\n\n");
    out.push_str("1. `search` — check for existing code before writing\n");
    out.push_str("2. Write code — respect the hard rules above\n");
    out.push_str("3. Write tests — target 85%+ per file\n");
    out.push_str("4. `check_quality` — verify quality gates pass\n");
    out.push_str("5. `catchup` — find any unregistered functions\n");
    out.push_str("6. `check_coverage` — verify coverage gates pass before pushing\n\n");

    // Testing
    out.push_str("## Running Tests\n\n```bash\n");
    out.push_str("cargo test --workspace              # all tests\n");
    out.push_str("cargo test -p spec-store-core       # core library only\n");
    out.push_str("cargo llvm-cov --lcov --output-path lcov.info  # generate coverage\n```\n");

    Ok(out)
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
        // With an empty store and no src/ dir, catchup should return 0 missing
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
