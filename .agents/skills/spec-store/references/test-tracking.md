# Test Tracking

spec-store tracks tests alongside production code. Every scanned function is tagged
`is_test: true/false` based on language-specific conventions.

## How tests are detected

**Rust**: `#[test]`, `#[tokio::test]`, `#[rstest]` attributes, or functions inside
`#[cfg(test)]` modules.

**Python**: Functions with `test_` prefix, or `@pytest.mark`/`@pytest.fixture` decorators.

**TypeScript**: Functions with `test_` prefix, or files matching `*.test.ts`,
`*.spec.ts`, or in `__tests__/` directories.

## Listing tests

```bash
# Via CLI — catchup shows [test] tags
spec-store catchup --path src/

# Via MCP tool
list_tests(path?)
```

## Function-to-test mapping

spec-store maps tests to production functions using two heuristics:

1. **Name match**: `test_validate_stake` maps to `validate_stake` (highest confidence)
2. **File match**: Tests in the same file as production code map to all production
   functions in that file (lower confidence, used when no name match found)

```bash
# Via MCP tool
test_mappings(function_name?: "validate_stake")
```

## Per-function coverage

When an `lcov.info` file is available, spec-store cross-references LCOV `DA:` lines
with function line ranges to compute per-function coverage percentages.

```bash
# Via MCP tool
function_coverage(from?: "lcov.info", function_name?: "validate_stake")
```

Returns for each function: name, file, percentage, lines found, lines hit.

## Workflow for agents

1. Before modifying a function, check its test mappings to know which tests to update
2. After modifying a function, re-run tests and check per-function coverage
3. When writing new functions, write tests alongside — coverage gates require 85%+
