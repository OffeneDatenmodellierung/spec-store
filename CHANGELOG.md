# Changelog

All notable changes to spec-store will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-12

### Added

- **Coverage report grouped by folder** with readable filenames instead of truncated paths
- **`--json` flag** on `spec-store coverage report` for machine-readable output
- **`references/configuration.md`** in skill documenting all config.toml settings
- **`references/releasing.md`** in skill documenting release process and CHANGELOG discipline
- **158 tests** (up from 122) with 75.3% overall coverage
- **15 architectural decisions** recorded in spec-store's own registry

### Fixed

- **Block line counting** — `count_block_lines` now uses brace-depth tracking instead of
  indentation heuristics, fixing false positives where function line counts bled into
  subsequent functions
- **Complexity measurement** — limited to function's actual line range instead of 200-line
  window, fixing inflated scores from neighbouring functions
- **Quality gates skip test functions** — only production code is gated for count, lines,
  complexity, and parameters
- **Coverage false regressions** — floating point epsilon comparison fixed

### Removed

- **MCP server** (`spec-store-mcp` crate) — the skill + CLI covers all agent integration.
  Agents call `spec-store` via shell commands; no MCP server needed
- **`AiConfig`**, **`EmbeddingsConfig`**, **`LightllmConfig`** structs — last remnants of
  the deleted AI provider layer. Old config files with `[ai]` sections still parse fine
- `rmcp`, `schemars`, `tracing`, `tracing-subscriber` dependencies

### Changed

- **Workspace reduced** to two crates: `spec-store-core` (library) + `spec-store-cli` (binary)
- **`register_fn`** takes `RegisterFnInput` struct instead of 6 positional params
- **`print_report`** takes `ReportInput` struct instead of 5 params
- **`ops/mod.rs`** split into `coverage_ops.rs`, `worktree_ops.rs`, `context_ops.rs`
- **`regex_scanner.rs`** block measurement extracted to `block_measure.rs`
- **`tools.rs`** param structs extracted to `params.rs` (then removed with MCP crate)
- Release workflow packages single `spec-store` binary (not two)

## [0.2.0] - 2026-04-12

### Added

- **Workspace architecture**: Split into three crates — `spec-store-core` (library),
  `spec-store-cli` (CLI binary), `spec-store-mcp` (MCP server binary)
- **Test tracking**: Functions tagged `is_test` via language-specific detection
  - Rust: `#[test]`, `#[tokio::test]`, `#[rstest]`, `#[cfg(test)]` modules
  - Python: `test_` prefix, `@pytest.mark` decorators
  - TypeScript: `test_` prefix, `.test.ts`/`.spec.ts` files, `__tests__/` dirs
- **Function-to-test mapping**: Name and file heuristics link tests to production code
- **Per-function coverage**: LCOV `DA:` line parsing cross-referenced with function ranges
- **`--staged` flag**: `catchup` and `quality check` can scan only git-staged files
- **`--auto-register` flag**: `catchup` can auto-register unregistered functions
- **Worktree verify**: Checks staged files against worktree contract claims
- **Git worktree detection**: `worktree list` shows both spec-store claims and `git worktree` entries
- **`--version` flag** on `spec-store` binary
- **Agent skill**: `.agents/skills/spec-store/` with SKILL.md and reference docs
- **Ops layer**: shared business logic for CLI
- **Shared `is_excluded`**: Deduplicated into `util.rs`
- **`git.rs` module**: `staged_files()`, `current_branch()`, conflict detection

### Fixed

- `worktree verify` was a stub — now checks staged files against worktree contracts
- `catchup` ignored `--staged`, `--auto-register` flags — now implemented
- `quality check` ignored `--staged` flag — now scans only staged files
- `assert_no_blocks` error showed `100` instead of configured threshold
- `context/generator.rs` hardcoded `85.0%` threshold — now reads from config
- `fail_on_regression` config field was declared but never used — now wired into checker
- `is_excluded` duplicated with divergent logic — unified
- Post-checkout hook referenced wrong command — corrected

### Removed

- AI provider layer — AI is the caller's responsibility
- Interview session — agent conversation replaces terminal interview
- `--ai-describe`, `--dry-run` flags
- `Feature` struct and `features` table — never used
- `glob`, `hex`, `reqwest`, `async-trait` dependencies
