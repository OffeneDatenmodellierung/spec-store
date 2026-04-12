# CLAUDE.md — spec-store

This file is read automatically by Claude Code and other AI agents.
Do not modify it manually — regenerate with `spec-store context generate`.

## Project

`spec-store` is a Rust CLI tool that maintains a codebase specification registry
with semantic search, quality gates, and multi-agent worktree coordination.
It dogfoods its own gates: all files must pass the rules it enforces on others.

## Hard Rules (enforced by gates — do not bypass)

| Rule | Limit |
|------|-------|
| Lines per file (code) | 300 |
| Lines per function | 50 |
| Functions per file | 15 |
| Cyclomatic complexity | 10 |
| Parameters per function | 5 |
| Test coverage per file | 85% |
| Similarity to existing fn | < 0.95 (blocked), < 0.85 (warn) |

## Before Writing Any New Function

```bash
spec-store search "<what you intend to write>"
```

If a similarity ≥ 0.85 exists, extend the existing function or record a reason:
```bash
spec-store reuse acknowledge <id> --reason "..."
```

## Module Map

```
src/
  main.rs          — entry point only, <20 lines
  error.rs         — shared error enum
  config.rs        — load/save .spec-store/config.toml
  cli/
    commands.rs    — clap structs (no logic)
    dispatch.rs    — command routing and AppContext
  store/
    structured.rs  — SQLite: decisions, features, worktrees, functions
    baseline.rs    — coverage baseline ratchet (JSON-backed)
    vector.rs      — cosine similarity vector store (JSON-backed)
  scanner/
    regex_scanner.rs — Rust/Python/TS function extractor
    quality.rs       — file/fn length, complexity, param gates
  coverage/
    lcov.rs        — lcov format parser
    checker.rs     — threshold + ratchet enforcement
    reporter.rs    — coloured terminal output
  ai/
    provider.rs    — LlmProvider trait + NoneProvider
    claude.rs      — Anthropic API impl
    lightllm.rs    — OpenAI-compat impl
  hooks/
    installer.rs   — writes .githooks/, sets core.hooksPath
  interview/
    session.rs     — multi-turn AI interview
  context/
    generator.rs   — produces AGENTS.md from store state
  reuse/
    enforcer.rs    — similarity gate with tiered thresholds
```

## Key Architectural Decisions

- [2026-04-07] Word-bag local embeddings by default; no API calls for vector search
- [2026-04-07] Qdrant is optional (future); LocalVectorStore covers most repos
- [2026-04-07] Baselines ratchet upward only — coverage can never regress
- [2026-04-07] Per-file coverage threshold, not project-wide, to prevent masking
- [2026-04-07] Git hooks committed to `.githooks/` + `core.hooksPath` for team enforcement
- [2026-04-07] async-trait used for LlmProvider; NoneProvider makes CLI work offline

## Running Tests

```bash
cargo test                          # all tests
cargo test --lib coverage           # coverage module only
cargo llvm-cov --lcov --output-path lcov.info  # generate coverage report
spec-store coverage check           # enforce gates
```

## Adding a New Feature

1. `spec-store search "<intent>"` — check for existing code
2. `spec-store worktree claim <branch> --contract <path>`
3. Write code — keep files < 300 lines, functions < 50 lines
4. Write tests — target 85%+ per file
5. `spec-store quality check --path src/<your module>/`
6. `spec-store catchup --staged` — register new functions
7. Push — `pre-push` hook checks coverage and worktree conflicts
