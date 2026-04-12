//! Context generation and project rules operations.

use crate::{
    config::Config,
    context::{self, ContextOptions},
    store::structured::{Decision, Worktree},
    AppContext,
};

pub fn generate_context(ctx: &AppContext, opts: &ContextOptions) -> anyhow::Result<String> {
    context::generate(&ctx.root, &ctx.structured, &ctx.baseline, opts)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn write_context(ctx: &AppContext, content: &str, output: &str) -> anyhow::Result<()> {
    context::write(&ctx.root, content, output).map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn project_rules(ctx: &AppContext) -> anyhow::Result<String> {
    let decisions = ctx
        .structured
        .list_decisions()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let worktrees = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut out = String::new();
    push_header(&mut out);
    push_hard_rules(&mut out, &ctx.config);
    push_workflow(&mut out);
    push_decisions(&mut out, &decisions);
    push_worktrees(&mut out, &worktrees);
    push_dev_workflow(&mut out);
    Ok(out)
}

fn push_header(out: &mut String) {
    out.push_str("# spec-store — Project Rules\n\n");
    out.push_str(
        "`spec-store` is a Rust workspace that maintains a codebase specification registry \
         with semantic search, quality gates, and multi-agent worktree coordination.\n\n",
    );
}

fn push_hard_rules(out: &mut String, cfg: &Config) {
    out.push_str("## Hard Rules (enforced by gates — do not bypass)\n\n");
    out.push_str("| Rule | Limit |\n|------|-------|\n");
    let q = &cfg.quality;
    let c = &cfg.coverage;
    let r = &cfg.reuse;
    out.push_str(&format!(
        "| Lines per file (code) | {} |\n",
        q.max_file_lines
    ));
    out.push_str(&format!("| Lines per function | {} |\n", q.max_fn_lines));
    out.push_str(&format!(
        "| Functions per file | {} |\n",
        q.max_fns_per_file
    ));
    out.push_str(&format!(
        "| Cyclomatic complexity | {} |\n",
        q.max_fn_complexity
    ));
    out.push_str(&format!(
        "| Parameters per function | {} |\n",
        q.max_fn_params
    ));
    out.push_str(&format!(
        "| Test coverage per file | {:.0}% |\n",
        c.min_per_file
    ));
    out.push_str(&format!(
        "| Similarity to existing fn | < {:.2} (blocked), < {:.2} (warn) |\n\n",
        r.similarity_block, r.similarity_warn
    ));
}

fn push_workflow(out: &mut String) {
    out.push_str("## Before Writing Any New Function\n\n");
    out.push_str("1. Use the `search` tool with a description of what you intend to write\n");
    out.push_str("2. If similarity >= 0.85, extend the existing function instead\n");
    out.push_str("3. Use `reuse_check` to verify your new function won't be blocked\n\n");
    out.push_str("## Workspace Structure\n\n```\ncrates/\n");
    out.push_str("  spec-store-core/   — library: stores, scanners, coverage, quality gates\n");
    out.push_str("  spec-store-cli/    — thin CLI for CI/hooks (binary: spec-store)\n");
    out.push_str("  spec-store-mcp/    — MCP server for agent integration\n```\n\n");
}

fn push_decisions(out: &mut String, decisions: &[Decision]) {
    if decisions.is_empty() {
        return;
    }
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

fn push_worktrees(out: &mut String, worktrees: &[Worktree]) {
    if worktrees.is_empty() {
        return;
    }
    out.push_str("## Active Worktrees (do not modify files owned by others)\n\n");
    for wt in worktrees {
        let owner = wt.owner.as_deref().unwrap_or("unassigned");
        out.push_str(&format!("- `{}` ({})", wt.branch, owner));
        if let Some(c) = &wt.contract {
            out.push_str(&format!(" → `{c}`"));
        }
        out.push('\n');
    }
    out.push('\n');
}

fn push_dev_workflow(out: &mut String) {
    out.push_str("## Development Workflow\n\n");
    out.push_str("1. `search` — check for existing code before writing\n");
    out.push_str("2. Write code — respect the hard rules above\n");
    out.push_str("3. Write tests — target 85%+ per file\n");
    out.push_str("4. `check_quality` — verify quality gates pass\n");
    out.push_str("5. `catchup` — find any unregistered functions\n");
    out.push_str("6. `check_coverage` — verify coverage gates pass before pushing\n\n");
    out.push_str("## Running Tests\n\n```bash\n");
    out.push_str("cargo test --workspace              # all tests\n");
    out.push_str("cargo test -p spec-store-core       # core library only\n");
    out.push_str("cargo llvm-cov --lcov --output-path lcov.info  # generate coverage\n```\n");
}
