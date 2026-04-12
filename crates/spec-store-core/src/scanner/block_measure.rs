//! Block measurement: line counting and complexity for function bodies.

use super::regex_scanner::complexity_re;

pub fn measure_block(lines: &[&str], start_line: usize) -> (usize, usize) {
    let idx = start_line.saturating_sub(1);
    let line_count = count_block_lines(lines, idx);
    let snippet: String = lines
        .iter()
        .skip(idx)
        .take(line_count)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let complexity = complexity_re().find_iter(&snippet).count().min(50) + 1;
    (line_count, complexity)
}

fn count_brace_block(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut found_open = false;
    let mut count = 0;
    for line in lines.iter().skip(start) {
        count += 1;
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                found_open = true;
            } else if ch == '}' {
                depth -= 1;
                if found_open && depth == 0 {
                    return Some(count);
                }
            }
        }
        if count > 200 {
            return Some(count);
        }
    }
    if found_open {
        Some(count)
    } else {
        None
    }
}

fn count_indent_block(lines: &[&str], start: usize) -> usize {
    let base = lines
        .get(start)
        .map(|l| l.chars().take_while(|c| *c == ' ').count())
        .unwrap_or(0);
    let mut count: usize = 0;
    for line in lines.iter().skip(start) {
        count += 1;
        if count > 1 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let depth = line.chars().take_while(|c| *c == ' ').count();
            if depth <= base && !trimmed.starts_with('#') {
                return count.saturating_sub(1).max(1);
            }
        }
        if count > 200 {
            break;
        }
    }
    count
}

fn count_block_lines(lines: &[&str], start: usize) -> usize {
    count_brace_block(lines, start).unwrap_or_else(|| count_indent_block(lines, start))
}
