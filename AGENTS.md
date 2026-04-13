# Agent Context — spec-store

> For full workflow, activate the `spec-store` skill or read
> `.agents/skills/spec-store/SKILL.md`.

## Quick Reference

```bash
spec-store search "<what you intend to write>"   # ALWAYS run before writing new functions
spec-store quality check --staged                 # check quality gates
spec-store catchup --staged --fail-on-missing     # find unregistered functions
spec-store coverage check                         # enforce coverage (needs lcov.info)
spec-store worktree verify                        # check for worktree conflicts
spec-store status                                 # overall health
spec-store decision list                          # architectural decisions
spec-store coverage report --json                 # machine-readable coverage
```

## Rules

- Max 300 code lines per file, 50 per function, 15 production functions per file
- Max complexity 10, max 5 params per function
- 85% test coverage per file, ratchet-only baselines
- Similarity >= 0.95 blocks, >= 0.85 warns
- Every change must update `CHANGELOG.md`
- You provide descriptions when registering — spec-store does NOT generate them

## Before Pushing

```bash
cargo llvm-cov --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'
spec-store quality check --staged
spec-store catchup --staged --fail-on-missing
spec-store coverage check
spec-store worktree verify
```
