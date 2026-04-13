//! Detect whether functions are tests based on language-specific conventions.

/// Find line ranges (1-indexed, inclusive) of `#[cfg(test)]` modules in Rust source.
pub fn find_cfg_test_ranges(source: &str) -> Vec<(usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut ranges = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed == "#[cfg(test)]" {
            // Look for the next `mod` line
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && lines[j].trim().starts_with("mod ") {
                let start = i + 1; // 1-indexed, includes #[cfg(test)] line
                let end = find_block_end(&lines, j);
                ranges.push((start, end));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    ranges
}

/// Find the closing brace of a block starting at `start_idx` (0-indexed).
fn find_block_end(lines: &[&str], start_idx: usize) -> usize {
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
            }
            if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return i + 1; // 1-indexed
                }
            }
        }
    }
    lines.len()
}

fn is_test_attribute(line: &str) -> bool {
    let t = line.trim();
    t == "#[test]" || t.starts_with("#[tokio::test") || t.starts_with("#[rstest")
}

fn in_cfg_test_range(fn_line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|&(start, end)| fn_line >= start && fn_line <= end)
}

/// Check if a Rust function at `fn_line` (1-indexed) is a test.
pub fn is_test_rust(source: &str, fn_line: usize, cfg_test_ranges: &[(usize, usize)]) -> bool {
    if in_cfg_test_range(fn_line, cfg_test_ranges) {
        return true;
    }
    has_test_attribute_above(source, fn_line)
}

fn has_test_attribute_above(source: &str, fn_line: usize) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let fn_idx = fn_line.saturating_sub(1);
    let scan_start = fn_idx.saturating_sub(5);

    for i in (scan_start..fn_idx).rev() {
        let Some(line) = lines.get(i) else { continue };
        if is_test_attribute(line) {
            return true;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "}" {
            break;
        }
    }
    false
}

/// Check if a Python function is a test.
pub fn is_test_python(name: &str, source: &str, fn_line: usize) -> bool {
    if name.starts_with("test_") {
        return true;
    }

    // Check for pytest decorators above the def line
    let lines: Vec<&str> = source.lines().collect();
    let start = fn_line.saturating_sub(1).saturating_sub(3);
    let end = fn_line.saturating_sub(1);

    for i in start..end {
        if i >= lines.len() {
            break;
        }
        let trimmed = lines[i].trim();
        if trimmed.starts_with("@pytest.mark") || trimmed.starts_with("@pytest.fixture") {
            return true;
        }
    }
    false
}

/// Check if a TypeScript function is a test.
pub fn is_test_typescript(name: &str, file: &str) -> bool {
    name.starts_with("test_")
        || file.contains("__tests__")
        || file.ends_with(".test.ts")
        || file.ends_with(".test.tsx")
        || file.ends_with(".spec.ts")
        || file.ends_with(".spec.tsx")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_test_attribute() {
        let source = "fn prod() {}\n\n#[test]\nfn test_thing() {}\n";
        let ranges = find_cfg_test_ranges(source);
        assert!(!is_test_rust(source, 1, &ranges)); // prod()
        assert!(is_test_rust(source, 4, &ranges)); // test_thing()
    }

    #[test]
    fn detects_tokio_test() {
        let source = "#[tokio::test]\nasync fn test_async() {}\n";
        let ranges = find_cfg_test_ranges(source);
        assert!(is_test_rust(source, 2, &ranges));
    }

    #[test]
    fn detects_cfg_test_module() {
        let source = "\
fn prod_fn() {}

#[cfg(test)]
mod tests {
    fn helper() {}
    #[test]
    fn test_a() {}
}
";
        let ranges = find_cfg_test_ranges(source);
        assert_eq!(ranges.len(), 1);
        assert!(!is_test_rust(source, 1, &ranges)); // prod_fn
        assert!(is_test_rust(source, 5, &ranges)); // helper inside cfg(test)
        assert!(is_test_rust(source, 7, &ranges)); // test_a inside cfg(test)
    }

    #[test]
    fn non_test_function_not_detected() {
        let source = "pub fn validate(x: i32) -> bool { x > 0 }\n";
        let ranges = find_cfg_test_ranges(source);
        assert!(!is_test_rust(source, 1, &ranges));
    }

    #[test]
    fn python_test_prefix() {
        assert!(is_test_python("test_validate", "", 1));
        assert!(!is_test_python("validate", "", 1));
        assert!(!is_test_python("helper", "", 1));
    }

    #[test]
    fn python_pytest_decorator() {
        let source = "@pytest.mark.parametrize('x', [1,2])\ndef check(x):\n    pass\n";
        assert!(is_test_python("check", source, 2));
    }

    #[test]
    fn typescript_test_conventions() {
        assert!(is_test_typescript("test_render", "src/app.ts"));
        assert!(is_test_typescript("render", "src/app.test.ts"));
        assert!(is_test_typescript("render", "src/__tests__/app.ts"));
        assert!(is_test_typescript("render", "src/app.spec.tsx"));
        assert!(!is_test_typescript("render", "src/app.ts"));
    }
}
