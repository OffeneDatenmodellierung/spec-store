# spec-store

A codebase specification registry with semantic search, quality gates, and multi-agent worktree coordination. Designed to prevent code duplication, enforce coverage, and help AI agents (and junior developers) work safely in parallel on a shared codebase.

## Install

```bash
cargo install --path .
spec-store init
```

`init` writes `.githooks/` and sets `git config core.hooksPath .githooks/` so every team member gets the hooks automatically on clone.

## Core Commands

```bash
# Semantic search before writing anything new
spec-store search "validate a bet stake against customer limits"

# Register a function
spec-store register fn --name validate_stake --file src/risk.rs --line 42 --desc "Validates stake vs limit"

# Record an architectural decision
spec-store decision add "Use HMAC-SHA256 for all webhook tokens, no exceptions" --tags security,tokens

# Scan for unregistered functions
spec-store catchup --dry-run
spec-store catchup --ai-describe   # use Claude to auto-generate descriptions

# Coverage gates
spec-store coverage report         # full per-file report
spec-store coverage check          # enforce 85% floor + ratchet (used by pre-push hook)
spec-store coverage baseline       # set baseline from current lcov.info

# Quality gates
spec-store quality check --staged  # check staged files (used by pre-commit hook)
spec-store quality report          # full project quality summary

# Multi-agent worktree coordination
spec-store worktree claim feat/spend --contract src/contracts/spend.yaml --owner agent-2
spec-store worktree list
spec-store worktree release feat/spend

# Generate just-in-time agent context
spec-store context generate --worktree feat/spend --output AGENTS.md

# Interactive intent capture
spec-store interview --scope project   # new project bootstrap
spec-store interview --scope feature   # before starting a new feature

# Overall health
spec-store status
```

## Configuration

`.spec-store/config.toml` is created by `init` with sensible defaults:

```toml
[coverage]
min_per_file      = 85.0      # per-file floor
ratchet           = true      # baselines can only increase
fail_on_regression = true

[quality]
max_file_lines    = 300
max_fn_lines      = 50
max_fn_complexity = 10
max_fns_per_file  = 15
max_fn_params     = 5

[reuse]
similarity_warn   = 0.85      # yellow warning
similarity_block  = 0.95      # blocks commit

[ai]
provider          = "claude"  # claude | lightllm | none
model             = "claude-sonnet-4-20250514"
```

Set `ANTHROPIC_API_KEY` for Claude features. For local models, set `provider = "lightllm"` and configure the `[ai.lightllm]` section.

## Git Hooks

| Hook | Action |
|------|--------|
| `pre-commit` | Quality gates on staged files + catchup check |
| `pre-push` | Coverage gates + worktree conflict detection |
| `post-merge` | Auto-register new functions + update baselines |
| `post-checkout` | Regenerate `AGENTS.md` for the new branch |

Hooks live in `.githooks/` (committed). New team members activate them with `spec-store init`.

## AI Integration

spec-store works offline — search and quality gates need no API. Claude (or any OpenAI-compatible model via lightllm) is used for:

- `catchup --ai-describe` — auto-generate function descriptions
- `interview` — guided Q&A that populates decisions and feature records
- Future: suggest splits for oversized files

## Dogfooding

spec-store enforces its own gates on itself. If a PR breaks any rule, the pre-push hook blocks it. Coverage is tracked per-file and cannot regress.

## Architecture

See `CLAUDE.md` for the full module map and architectural decisions.
