# Quality Gates

These limits are enforced by `spec-store quality check` and cannot be bypassed.
Values come from `.spec-store/config.toml` — defaults shown below.

| Rule | Default | Config key |
|------|---------|------------|
| Lines per file (code, excluding blanks/comments) | 300 | `quality.max_file_lines` |
| Lines per function | 50 | `quality.max_fn_lines` |
| Functions per file | 15 | `quality.max_fns_per_file` |
| Cyclomatic complexity per function | 10 | `quality.max_fn_complexity` |
| Parameters per function | 5 | `quality.max_fn_params` |
| Test coverage per file | 85% | `coverage.min_per_file` |
| Similarity to existing function | < 0.95 (blocked), < 0.85 (warn) | `reuse.similarity_block`, `reuse.similarity_warn` |

## Coverage enforcement

- Coverage is **per-file**, not project-wide — prevents high-coverage files from masking low ones
- Baselines **ratchet upward only** — coverage can never regress below a recorded baseline
- `fail_on_regression` controls whether regressions are errors or warnings

## Checking gates

```bash
# Check quality on all src/ files
spec-store quality check --path src/

# Check only staged files (for pre-commit hooks)
spec-store quality check --staged

# Check a single file
spec-store quality check --file src/risk.rs

# Full quality report (no exit code)
spec-store quality report

# Check coverage gates
spec-store coverage check

# Check coverage from a specific lcov file
spec-store coverage check --from path/to/lcov.info
```

## What to do when gates fail

- **File too long**: Split into submodules. Move related functions into a new file.
- **Function too long**: Extract helper functions. Break up large match blocks.
- **Too many functions**: Group related functions into a submodule.
- **Complexity too high**: Simplify branching. Extract conditions into named booleans. Use early returns.
- **Too many parameters**: Use a config/options struct.
- **Coverage too low**: Write tests. Target the uncovered lines specifically.
- **Similarity too high**: Extend the existing function instead. Use `spec-store search` to find it.
