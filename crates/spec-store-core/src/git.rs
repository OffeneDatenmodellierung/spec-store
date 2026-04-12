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

    #[test]
    fn staged_files_returns_empty_in_non_git_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(staged_files(dir.path()).is_empty());
    }

    #[test]
    fn current_branch_returns_none_in_non_git_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(current_branch(dir.path()).is_none());
    }

    #[test]
    fn staged_files_works_in_git_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        // No staged files in empty repo
        assert!(staged_files(dir.path()).is_empty());
    }

    #[test]
    fn current_branch_works_in_git_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init", "--quiet", "-b", "main"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        // Need at least one commit for branch to exist
        std::fs::write(dir.path().join("README"), "test").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init", "--no-gpg-sign"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert_eq!(current_branch(dir.path()), Some("main".into()));
    }

    #[test]
    fn no_conflict_when_worktree_has_no_contract() {
        let staged = vec!["src/foo.rs".into()];
        let worktrees = vec![Worktree {
            branch: "feat/x".into(),
            contract: None,
            owner: None,
            claimed_at: String::new(),
        }];
        let conflicts = staged_files_conflict_with_worktrees(&staged, &worktrees, None);
        assert!(conflicts.is_empty());
    }
}
