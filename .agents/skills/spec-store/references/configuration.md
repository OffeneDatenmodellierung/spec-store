# Configuration

All settings live in `.spec-store/config.toml`. Created by `spec-store init`.

## Coverage

```toml
[coverage]
min_per_file = 85.0          # Per-file coverage threshold (%)
lcov_path = "lcov.info"      # Default lcov file path
lcov_max_age_mins = 60       # Max age of lcov file before warning
ratchet = true               # Baselines can only go up, never down
fail_on_regression = true    # Treat regressions as errors (false = warnings)

# Files excluded from coverage checks entirely
exclude = [
    "src/generated/**",
    "tests/**",
    "benches/**",
]

# Files tested by e2e/integration tests (not in lcov.info)
# These show as "⊘ e2e" in reports instead of failures
e2e_tested = [
    "crates/spec-store-cli/src/dispatch.rs",
    "crates/spec-store-cli/src/reporter.rs",
]
```

## Quality

```toml
[quality]
max_file_lines = 300         # Max code lines per file (excluding blanks/comments)
max_fn_lines = 50            # Max lines per function
max_fn_complexity = 10       # Max cyclomatic complexity per function
max_fns_per_file = 15        # Max production functions per file (tests excluded)
max_fn_params = 5            # Max parameters per function (warning, not error)
warn_only = false            # If true, violations are warnings not errors

# Files excluded from quality checks
exclude = [
    "src/generated/**",
    "crates/spec-store-mcp/**",
]
```

## Reuse / Similarity

```toml
[reuse]
similarity_warn = 0.85       # Warn if new function is >= 85% similar to existing
similarity_block = 0.95      # Block if new function is >= 95% similar
```

## Output formats

Coverage report supports `--json` for machine-readable output:

```bash
# Human-readable (grouped by folder)
spec-store coverage report

# JSON output (for CI, scripts, agents)
spec-store coverage report --json
```
