# CLAUDE.md — spec-store

Read by Claude Code automatically. For full details, activate the spec-store skill
or read `.agents/skills/spec-store/SKILL.md`.

## Project

`spec-store` is a Rust workspace (two crates: `spec-store-core` library +
`spec-store-cli` binary) that maintains a codebase specification registry
with semantic search, quality gates, test tracking, and multi-agent worktree
coordination. It dogfoods its own gates.

## Hard Rules (enforced by gates)

| Rule | Limit |
|------|-------|
| Lines per file (code) | 300 |
| Lines per function | 50 |
| Functions per file (production only) | 15 |
| Cyclomatic complexity | 10 |
| Parameters per function | 5 |
| Test coverage per file | 85% |
| Similarity to existing fn | < 0.95 (blocked), < 0.85 (warn) |

## Before Writing Any New Function

```bash
spec-store search "<what you intend to write>"
```

## Before Pushing

```bash
cargo llvm-cov --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'
spec-store quality check --staged
spec-store catchup --staged --fail-on-missing
spec-store coverage check
spec-store worktree verify
```

## CHANGELOG.md is mandatory

Every user-visible change must add an entry to `CHANGELOG.md` under `[Unreleased]`.

## Running Tests

```bash
cargo test --workspace
cargo test -p spec-store-core
cargo llvm-cov --lcov --output-path lcov.info  # generate coverage
spec-store coverage check                       # enforce gates
```

## Module Map

```
crates/
  spec-store-core/src/
    lib.rs, config.rs, error.rs, git.rs, util.rs
    ops/           — search, register, catchup, coverage, quality, worktrees
    store/         — SQLite, baselines (JSON), vector store (JSON)
    scanner/       — regex function extractor, quality gates, test detection, test mapping
    coverage/      — LCOV parser, checker, per-function coverage
    reuse/         — similarity gate
    hooks/         — git hook installer
    context/       — AGENTS.md generator
  spec-store-cli/src/
    main.rs, commands.rs, dispatch.rs, reporter.rs
```
