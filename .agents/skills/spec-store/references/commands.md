# CLI Command Reference

## Initialisation

```bash
spec-store init
```

Creates `.spec-store/config.toml`, SQLite store, and installs git hooks to `.githooks/`.

## Search

```bash
spec-store search "<natural language query>" [--limit 5]
```

Semantic search across registered functions and decisions. Always run this before
writing new functions to check for existing code.

## Function Registration

```bash
# Register a function (description provided by you, NOT auto-generated)
spec-store register fn --name "validate_stake" --file "src/risk.rs" --line 42 --desc "Validates stake against limit"

# Register a decision
spec-store register decision "Use HMAC-SHA256 for all tokens" --tags security,auth
```

## Catchup (find unregistered functions)

```bash
# Scan all source files
spec-store catchup --path src/

# Scan only staged files
spec-store catchup --staged

# Auto-register everything (for hooks)
spec-store catchup --auto-register

# Fail if any unregistered (for CI)
spec-store catchup --path src/ --fail-on-missing
```

## Decisions

```bash
# Add an architectural decision
spec-store decision add "Use JWT for auth" --tags auth,security

# List all decisions
spec-store decision list
```

## Coverage

**Prerequisite**: Coverage commands require an `lcov.info` file. Generate it first:

```bash
# Generate coverage data (MUST run before any coverage command)
cargo llvm-cov --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'
```

```bash
# Full coverage report
spec-store coverage report

# Check coverage gates (exits non-zero on failure)
spec-store coverage check

# Use a specific lcov file
spec-store coverage check --from path/to/lcov.info

# Update baselines (ratchet up only)
spec-store coverage baseline --from lcov.info
```

## Quality

```bash
# Check quality gates on all src/
spec-store quality check --path src/

# Check only staged files
spec-store quality check --staged

# Check a single file
spec-store quality check --file src/risk.rs

# Full report (no exit code)
spec-store quality report
```

## Worktrees

```bash
# Claim a branch for exclusive development
spec-store worktree claim feat/auth --contract src/auth --owner agent-1

# Release a claim
spec-store worktree release feat/auth

# List all worktrees (spec-store claims + git worktrees)
spec-store worktree list

# Verify no staged files conflict with other claims
spec-store worktree verify
```

## Context Generation

```bash
# Generate AGENTS.md with current store state
spec-store context --output AGENTS.md

# Scope to a specific worktree
spec-store context --worktree feat/auth --output AGENTS.md
```

## Status

```bash
spec-store status
```

Shows function count, decision count, active worktrees, coverage baselines, and hook status.
