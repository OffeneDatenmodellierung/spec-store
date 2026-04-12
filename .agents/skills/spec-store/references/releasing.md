# Releasing

## CHANGELOG discipline

Every user-visible change MUST have an entry in `CHANGELOG.md` before committing.

### Format

```markdown
## [Unreleased]

### Added
- New feature description

### Fixed
- Bug fix description

### Changed
- Behaviour change description

### Removed
- Removed feature description
```

### Rules

1. Add entries under `[Unreleased]` during development
2. Use past tense ("Added", "Fixed") not imperative ("Add", "Fix")
3. Group by category: Added, Fixed, Changed, Removed, Deprecated, Security
4. Reference the module or command affected
5. One line per change — link to issues if applicable

## Release process

1. Ensure all tests pass: `cargo test --workspace`
2. Ensure clean lint: `cargo clippy --workspace --all-targets`
3. Ensure formatted: `cargo fmt --check`
4. Run security audit: `cargo audit`
5. Run spec-store gates:
   ```bash
   spec-store quality check --path crates/
   spec-store catchup --path crates/ --fail-on-missing
   spec-store coverage check
   ```
6. Update version in root `Cargo.toml` workspace section
7. Move `[Unreleased]` entries to `[X.Y.Z] - YYYY-MM-DD` in CHANGELOG.md
8. Add empty `[Unreleased]` section at top
9. Commit: `git commit -m "release: vX.Y.Z"`
10. Tag: `git tag vX.Y.Z`
11. Build release binaries: `cargo build --release --workspace`
12. Push: `git push && git push --tags`

## Versioning

- **Major** (X.0.0): Breaking changes to CLI flags or config format
- **Minor** (0.X.0): New features, new CLI commands
- **Patch** (0.0.X): Bug fixes, documentation, internal refactoring
