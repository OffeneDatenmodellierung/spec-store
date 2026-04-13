---
name: spec-store
description: Use when working in repos with a .spec-store/ directory. Enforces quality gates, coverage thresholds, function registry, and worktree coordination. Read this before writing or modifying code.
---

# spec-store

spec-store is a codebase specification registry that enforces quality gates,
tracks test coverage, manages function registration, and coordinates multi-agent
worktree access. It runs as a CLI tool — you call it via shell commands.

**Important**: spec-store does NOT generate descriptions or do AI work. You (the
agent) provide descriptions when registering functions. spec-store only stores,
searches, and enforces gates.

## When to activate this skill

- The project has a `.spec-store/` directory
- You're about to write new functions, modify existing code, or push changes
- You need to check quality gates, coverage, or find existing functions

## Workflow

### Before writing any new function

```bash
spec-store search "<what you intend to write>"
```

If similarity >= 0.85 exists, extend the existing function instead of writing a new one.

### Before pushing

```bash
# 1. Run tests with coverage (generates lcov.info — required by coverage check)
cargo llvm-cov --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'

# 2. Run gates
spec-store quality check --staged
spec-store catchup --staged --fail-on-missing
spec-store coverage check
spec-store worktree verify
```

### After finishing a feature

```bash
spec-store catchup --path src/  # find unregistered functions
# Then register each with a description YOU write:
spec-store register fn --name "validate_stake" --file "src/risk.rs" --line 42 --desc "Validates betting stake against configured limit"
```

### Every change must update CHANGELOG.md

**MANDATORY**: When making any user-visible change (feature, fix, removal), add an
entry to `CHANGELOG.md` under the `[Unreleased]` section before committing. Follow
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Categories:
`Added`, `Fixed`, `Changed`, `Removed`, `Deprecated`, `Security`.

See [references/releasing.md](references/releasing.md) for release process.

## Hard rules

See [references/quality-gates.md](references/quality-gates.md) for the enforced limits.

## Command reference

See [references/commands.md](references/commands.md) for full CLI usage.

## Test tracking

See [references/test-tracking.md](references/test-tracking.md) for test inventory,
function-to-test mapping, and per-function coverage.

## Configuration

See [references/configuration.md](references/configuration.md) for coverage thresholds,
quality limits, exclusion patterns, and `--json` output.

## CI / GitHub Action

See [references/ci-setup.md](references/ci-setup.md) for setting up spec-store
in CI pipelines using the reusable GitHub Action.

## Architecture

See [references/architecture.md](references/architecture.md) for module map and
key decisions.
