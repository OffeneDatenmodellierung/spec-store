# Architecture

## Workspace structure

```
crates/
  spec-store-core/     Library — all business logic, no I/O formatting
    src/
      lib.rs           AppContext (root, config, stores)
      config.rs        .spec-store/config.toml loader
      error.rs         SpecStoreError enum
      git.rs           staged_files, current_branch, conflict detection
      util.rs          shared helpers (is_excluded)
      ops/
        mod.rs         Core operations (search, register, catchup, etc.)
        test_tracking.rs  list_tests, function_coverage, test_mappings
      store/
        structured.rs  SQLite: decisions, worktrees, functions
        baseline.rs    Coverage baseline ratchet (JSON-backed)
        vector.rs      Cosine similarity vector store (JSON-backed)
      scanner/
        regex_scanner.rs  Rust/Python/TS function extractor
        quality.rs        File/fn length, complexity, param gates
        test_detect.rs    #[test], #[cfg(test)], pytest detection
        test_mapper.rs    Function-to-test mapping heuristics
      coverage/
        lcov.rs           LCOV parser (file-level + DA line-level)
        checker.rs        Threshold + ratchet enforcement
        fn_coverage.rs    Per-function coverage from DA lines
      reuse/
        enforcer.rs       Similarity gate with tiered thresholds
      hooks/
        installer.rs      Git hook file writer
      context/
        generator.rs      AGENTS.md generation

  spec-store-cli/      Thin CLI binary (name: spec-store)
    src/
      main.rs          Entry point
      commands.rs      Clap argument structs
      dispatch.rs      Command routing, coloured output
      reporter.rs      Coverage/quality terminal tables
```

## Key decisions

- **Word-bag local embeddings** — no API calls for vector search. SHA256 hash into 64 buckets, cosine similarity.
- **Baselines ratchet upward only** — coverage can never regress below a recorded baseline.
- **Per-file coverage threshold** — not project-wide, to prevent masking.
- **Git hooks in `.githooks/`** — committed to repo with `core.hooksPath` for team enforcement.
- **AI is the caller's responsibility** — spec-store stores and enforces, it does not generate descriptions or call LLM APIs.
- **CLI + skill for agent integration** — agents call `spec-store` via shell commands. The skill (`.agents/skills/spec-store/`) teaches agents the workflow.

## Data storage

All state lives in `.spec-store/`:
- `config.toml` — quality/coverage/reuse thresholds
- `store.db` — SQLite (decisions, functions, worktrees, test mappings, function coverage)
- `baselines.json` — per-file coverage baselines (ratchet)
- `vectors.json` — function embeddings for semantic search
