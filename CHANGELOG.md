# Changelog

All notable changes to spec-store will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-04-12

### Added

- **Workspace architecture**: Split into three crates — `spec-store-core` (library),
  `spec-store-cli` (CLI binary), `spec-store-mcp` (MCP server binary)
- **MCP server** with 17 tools and 2 resources for agent integration:
  - `search`, `register_fn`, `add_decision`, `list_decisions`
  - `check_coverage`, `check_quality`, `scan_functions`, `catchup`
  - `claim_worktree`, `release_worktree`, `list_worktrees`
  - `reuse_check`, `init`, `status`, `project_rules`
  - `list_tests`, `function_coverage`, `test_mappings`
  - Resources: `spec-store://rules`, `spec-store://config`
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
- **`project_rules` tool/resource**: Dynamic project rules from live config and store state
- **`--version` flag** on both `spec-store` and `spec-store-mcp` binaries
- **Agent skill**: `.agents/skills/spec-store/` with SKILL.md and reference docs
- **Ops layer**: `ops/mod.rs` + `ops/test_tracking.rs` — shared business logic for CLI and MCP
- **Shared `is_excluded`**: Deduplicated into `util.rs`, used by quality and coverage checkers
- **`git.rs` module**: `staged_files()`, `current_branch()`, conflict detection

### Fixed

- `worktree verify` was a stub that always printed success — now actually checks conflicts
- `catchup` ignored `--staged`, `--auto-register` flags — now implemented
- `quality check` ignored `--staged` flag — now scans only staged files
- `init --ai` flag did nothing — removed (AI is the caller's responsibility)
- `assert_no_blocks` error showed `100` instead of configured threshold — fixed
- `context/generator.rs` hardcoded `85.0%` threshold — now reads from config
- `fail_on_regression` config field was declared but never used — now wired into checker
- `is_excluded` duplicated with divergent logic in quality.rs and checker.rs — unified
- Post-checkout hook referenced `spec-store context generate` — corrected to `spec-store context`
- Reporter `truncate_path` test used byte length for multi-byte `…` character — fixed to char count

### Removed

- AI provider layer (`ai/provider.rs`, `ai/claude.rs`, `ai/lightllm.rs`) — AI is handled
  by the calling agent (Claude, Zed, etc.), not spec-store
- Interview session (`interview/session.rs`) — agent conversation replaces terminal interview
- `--ai-describe` flag on catchup — descriptions are the caller's responsibility
- `--dry-run` flag on catchup — default behavior already reports without registering
- `Feature` struct and `features` table — defined but never used
- `glob` and `hex` dependencies — never used
- `reqwest` and `async-trait` dependencies — only needed by deleted AI layer

### Changed

- `AiConfig` and `EmbeddingsConfig` marked legacy with `#[serde(default)]` for backward
  compatibility with existing config.toml files
- `register_fn` MCP tool description clarifies the agent provides descriptions
- `FunctionInfo` now includes `is_test: bool` field
- SQLite schema migrated to v2: `is_test` column, `test_mappings` table, `function_coverage` table
- Vector store payload includes `is_test` flag for search results
