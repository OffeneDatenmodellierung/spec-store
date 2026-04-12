//! Git helpers for spec-store.

use std::path::Path;

/// Return list of staged files (relative paths) from `git diff --cached --name-only`.
pub fn staged_files(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect(),
        _ => vec![],
    }
}

/// Check if any staged files overlap with claimed worktree contracts.
pub fn staged_files_conflict_with_worktrees(
    staged: &[String],
    worktrees: &[crate::store::structured::Worktree],
    current_branch: Option<&str>,
) -> Vec<String> {
    let mut conflicts = Vec::new();
    for wt in worktrees {
        // Skip our own worktree
        if current_branch == Some(&wt.branch) {
            continue;
        }
        if let Some(contract) = &wt.contract {
            for file in staged {
                if file.starts_with(contract) || contract.starts_with(file) {
                    conflicts.push(format!(
                        "{} conflicts with worktree '{}' ({})",
                        file,
                        wt.branch,
                        wt.owner.as_deref().unwrap_or("unassigned")
                    ));
                }
            }
        }
    }
    conflicts
}

/// Get the current git branch name.
pub fn current_branch(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::structured::Worktree;

    #[test]
    fn no_conflicts_when_no_worktrees() {
        let staged = vec!["src/foo.rs".into()];
        let conflicts = staged_files_conflict_with_worktrees(&staged, &[], None);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn detects_conflict_with_contract() {
        let staged = vec!["src/auth/login.rs".into()];
        let worktrees = vec![Worktree {
            branch: "feat/auth".into(),
            contract: Some("src/auth".into()),
            owner: Some("agent-1".into()),
            claimed_at: String::new(),
        }];
        let conflicts = staged_files_conflict_with_worktrees(&staged, &worktrees, None);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].contains("feat/auth"));
    }

    #[test]
    fn skips_own_worktree() {
        let staged = vec!["src/auth/login.rs".into()];
        let worktrees = vec![Worktree {
            branch: "feat/auth".into(),
            contract: Some("src/auth".into()),
            owner: Some("agent-1".into()),
            claimed_at: String::new(),
        }];
        let conflicts =
            staged_files_conflict_with_worktrees(&staged, &worktrees, Some("feat/auth"));
        assert!(conflicts.is_empty());
    }
}
