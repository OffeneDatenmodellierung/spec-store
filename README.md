# spec-store

A codebase specification registry with semantic search, quality gates, test tracking,
and multi-agent worktree coordination. Prevents code duplication, enforces coverage,
and helps AI agents and developers work safely in parallel.

## Install

```bash
cargo install --path crates/spec-store-cli
spec-store init
```

`init` creates `.spec-store/config.toml`, installs git hooks to `.githooks/`, and sets
`core.hooksPath` so every team member gets the hooks on clone.

## Core Commands

```bash
# Search before writing anything new
spec-store search "validate a bet stake against customer limits"

# Register a function (YOU provide the description)
spec-store register fn --name validate_stake --file src/risk.rs --line 42 --desc "Validates stake vs limit"

# Record an architectural decision
spec-store decision add "Use HMAC-SHA256 for all webhook tokens" --tags security,tokens

# Scan for unregistered functions
spec-store catchup --path src/
spec-store catchup --staged --fail-on-missing   # staged files only (for hooks)
spec-store catchup --auto-register              # auto-register all found

# Coverage gates (requires lcov.info — run tests first)
cargo llvm-cov --lcov --output-path lcov.info
spec-store coverage report              # grouped by folder
spec-store coverage report --json       # machine-readable JSON
spec-store coverage check               # enforce 85% floor + ratchet
spec-store coverage baseline            # set baseline from current lcov.info

# Quality gates
spec-store quality check --staged       # staged files only (pre-commit hook)
spec-store quality check --path src/    # all source files
spec-store quality report               # full project summary

# Multi-agent worktree coordination
spec-store worktree claim feat/spend --contract src/spend --owner agent-2
spec-store worktree list                # shows spec-store claims + git worktrees
spec-store worktree verify              # check staged files against claims
spec-store worktree release feat/spend

# Generate agent context
spec-store context --output AGENTS.md

# Overall health
spec-store status
```

## CI / GitHub Action

Add spec-store gates to any repo's CI pipeline:

```yaml
- name: spec-store gates
  uses: OffeneDatenmodellierung/spec-store/.github/actions/check@v0.3.0
  with:
    version: '0.3.0'
    lcov-path: lcov.info
    quality-path: src/
```

The action downloads a pinned release binary, runs quality gates, coverage checks,
and catchup. No Rust toolchain needed — just your `lcov.info` from a prior test step.

See `.agents/skills/spec-store/references/ci-setup.md` for full setup guide.

## Agent Integration

spec-store integrates with AI agents via a **skill** (`.agents/skills/spec-store/`),
not an MCP server. The skill teaches agents the workflow; agents call `spec-store` via
shell commands. Works with Claude Code, Zed, and any agent that supports
[skillsmcp](https://github.com/aviddiviner/skillsmcp) or reads `.agents/skills/`.

**spec-store does NOT do AI** — it stores, searches, and enforces. The calling agent
provides function descriptions when registering.

## Configuration

`.spec-store/config.toml` is created by `init` with sensible defaults:

```toml
[coverage]
min_per_file       = 85.0     # per-file floor
ratchet            = true     # baselines can only increase
fail_on_regression = true     # treat regressions as errors

[quality]
max_file_lines     = 300
max_fn_lines       = 50
max_fn_complexity  = 10
max_fns_per_file   = 15       # production functions only (tests excluded)
max_fn_params      = 5

[reuse]
similarity_warn    = 0.85     # yellow warning
similarity_block   = 0.95     # blocks commit
```

## Git Hooks

| Hook | Action |
|------|--------|
| `pre-commit` | Quality gates on staged files + catchup check |
| `pre-push` | Coverage gates + worktree conflict detection |
| `post-merge` | Auto-register new functions + update baselines |
| `post-checkout` | Regenerate `AGENTS.md` for the new branch |

Hooks live in `.githooks/` (committed). Activated by `spec-store init`.

## Test Tracking

Functions are tagged `is_test` via language-specific detection:
- **Rust**: `#[test]`, `#[tokio::test]`, `#[rstest]`, `#[cfg(test)]` modules
- **Python**: `test_` prefix, `@pytest.mark` decorators
- **TypeScript**: `test_` prefix, `.test.ts`/`.spec.ts` files

Quality gates only apply to production functions — test functions are excluded from
line count, complexity, function count, and parameter checks.

## Architecture

Two-crate workspace:
- `spec-store-core` — library with all business logic (stores, scanners, coverage, quality gates)
- `spec-store-cli` — thin CLI binary with coloured output

See `.agents/skills/spec-store/references/architecture.md` for the full module map.
