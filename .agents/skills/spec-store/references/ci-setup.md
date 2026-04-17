# CI Setup

spec-store provides a reusable GitHub Action that downloads a pinned release
binary and runs quality gates, coverage checks, and catchup in your CI pipeline.

## Quick start

Add to your workflow after generating `lcov.info`:

```yaml
- name: spec-store gates
  uses: OffeneDatenmodellierung/spec-store/.github/actions/check@v0.4.0
  with:
    version: '0.4.0'
    lcov-path: lcov.info
```

## Full example workflows

The spec-store gate step is identical regardless of language — only the
coverage-generation step changes.

### Rust

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: taiki-e/install-action@cargo-llvm-cov
      - name: Run tests with coverage
        run: |
          cargo llvm-cov \
            --workspace --lcov --output-path lcov.info \
            --ignore-filename-regex 'main\.rs'
      - name: spec-store gates
        uses: OffeneDatenmodellierung/spec-store/.github/actions/check@v0.4.0
        with:
          version: '0.4.0'
          lcov-path: lcov.info
          quality-path: src/
          catchup-path: src/
```

### Python (coverage.py + pytest)

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: '3.12' }
      - run: pip install -e ".[test]" coverage pytest
      - name: Run tests with coverage
        run: |
          coverage run -m pytest
          coverage lcov -o lcov.info
      - name: spec-store gates
        uses: OffeneDatenmodellierung/spec-store/.github/actions/check@v0.4.0
        with:
          version: '0.4.0'
          lcov-path: lcov.info
          quality-path: src/
          catchup-path: src/
```

### TypeScript / JavaScript (vitest or jest+nyc)

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - run: npm ci
      - name: Run tests with coverage
        run: npx vitest run --coverage --coverage.reporter=lcov
        # or with jest + nyc:  npx nyc --reporter=lcovonly npm test
      - name: Move LCOV to repo root
        run: cp coverage/lcov.info ./lcov.info
      - name: spec-store gates
        uses: OffeneDatenmodellierung/spec-store/.github/actions/check@v0.4.0
        with:
          version: '0.4.0'
          lcov-path: lcov.info
          quality-path: src/
          catchup-path: src/
```

## Inputs

| Input | Default | Description |
|-------|---------|-------------|
| `version` | **required** | spec-store release version to download |
| `lcov-path` | `lcov.info` | Path to coverage file |
| `quality-path` | `src/` | Path for quality gate checks |
| `catchup-path` | `src/` | Path to scan for unregistered functions |
| `check-quality` | `true` | Enable quality gate checks |
| `check-coverage` | `true` | Enable coverage gate checks |
| `check-catchup` | `true` | Enable catchup check |
| `fail-on-missing` | `true` | Fail if unregistered functions found |
| `skip-download` | `false` | Skip binary download (use spec-store already on PATH) |

## Outputs

| Output | Description |
|--------|-------------|
| `quality-passed` | Whether quality gates passed |
| `coverage-passed` | Whether coverage gates passed |
| `catchup-passed` | Whether all functions are registered |

## What it does

1. Downloads the `spec-store` binary from the pinned release
2. Initialises `.spec-store/` if not present
3. Runs `spec-store quality check` on the specified path
4. Resets baselines from CI's lcov data, then checks coverage (85% threshold)
5. Checks for unregistered functions with `catchup --fail-on-missing`

## Setting up a new project

```bash
# Install spec-store
cargo install spec-store

# Initialise in your project root
cd your-project
spec-store init

# Register existing functions
spec-store catchup --path src/ --auto-register

# Commit the spec-store state
git add .spec-store/ .githooks/
git commit -m "chore: add spec-store quality gates"
```

## For repos that build spec-store from source

Use `skip-download: true` when the binary is already installed earlier in the
workflow (e.g. spec-store's own CI):

```yaml
- name: Install from source
  run: cargo install --path crates/spec-store-cli --debug

- name: spec-store gates
  uses: ./.github/actions/check
  with:
    version: '0.4.0'
    skip-download: 'true'
    lcov-path: lcov.info
```

## Coverage notes

CI resets `baselines.json` before checking coverage to avoid cross-machine
ratchet conflicts. Coverage percentages can vary slightly between local and CI
due to compiler instrumentation differences. The 85% per-file threshold is
enforced; ratchet baselines are for local development only until cross-run
baseline persistence is implemented (see issue #2).
