//! Worktree coordination operations.

use crate::{store::structured::Worktree, AppContext};

pub fn claim_worktree(
    ctx: &mut AppContext,
    branch: &str,
    contract: Option<&str>,
    owner: Option<&str>,
) -> anyhow::Result<()> {
    ctx.structured
        .claim_worktree(branch, contract, owner)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn release_worktree(ctx: &mut AppContext, branch: &str) -> anyhow::Result<()> {
    ctx.structured
        .release_worktree(branch)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn list_worktrees(ctx: &AppContext) -> anyhow::Result<Vec<Worktree>> {
    let mut claimed = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let claimed_branches: std::collections::HashSet<String> =
        claimed.iter().map(|w| w.branch.clone()).collect();

    if let Ok(git_worktrees) = detect_git_worktrees(&ctx.root) {
        for gw in git_worktrees {
            if !claimed_branches.contains(&gw.branch) {
                claimed.push(gw);
            }
        }
    }
    Ok(claimed)
}

fn detect_git_worktrees(root: &std::path::Path) -> anyhow::Result<Vec<Worktree>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    Ok(parse_git_worktree_output(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

/// Parse `git worktree list --porcelain` output into Worktree entries.
fn parse_git_worktree_output(stdout: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if let Some(branch_ref) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch_ref.to_string());
        } else if line.is_empty() {
            if let Some(branch) = current_branch.take() {
                worktrees.push(Worktree {
                    branch,
                    contract: None,
                    owner: Some("git-worktree".into()),
                    claimed_at: String::new(),
                });
            }
        }
    }
    if let Some(branch) = current_branch {
        worktrees.push(Worktree {
            branch,
            contract: None,
            owner: Some("git-worktree".into()),
            claimed_at: String::new(),
        });
    }
    worktrees
}

/// Verify no staged files conflict with other worktree claims.
pub fn verify_worktrees(ctx: &AppContext) -> anyhow::Result<Vec<String>> {
    let worktrees = ctx
        .structured
        .list_worktrees()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let staged = crate::git::staged_files(&ctx.root);
    let current = crate::git::current_branch(&ctx.root);
    Ok(crate::git::staged_files_conflict_with_worktrees(
        &staged,
        &worktrees,
        current.as_deref(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, store::{BaselineStore, LocalVectorStore, StructuredStore}};

    fn test_ctx() -> AppContext {
        AppContext {
            root: std::path::PathBuf::from("."),
            config: config::Config::default(),
            structured: StructuredStore::open_in_memory().unwrap(),
            baseline: BaselineStore::new_empty(),
            vectors: LocalVectorStore::new_empty(),
        }
    }

    #[test]
    fn claim_and_release() {
        let mut ctx = test_ctx();
        claim_worktree(&mut ctx, "feat/x", Some("src/x"), Some("agent")).unwrap();
        let wts = list_worktrees(&ctx).unwrap();
        assert!(wts.iter().any(|w| w.branch == "feat/x"));
        release_worktree(&mut ctx, "feat/x").unwrap();
        let wts = list_worktrees(&ctx).unwrap();
        assert!(!wts.iter().any(|w| w.branch == "feat/x"));
    }

    #[test]
    fn verify_worktrees_empty() {
        let ctx = test_ctx();
        let conflicts = verify_worktrees(&ctx).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_porcelain_output() {
        let output = "worktree /path/to/main\nHEAD abc123\nbranch refs/heads/main\n\nworktree /path/to/feat\nHEAD def456\nbranch refs/heads/feat/auth\n\n";
        let wts = parse_git_worktree_output(output);
        assert_eq!(wts.len(), 2);
        assert_eq!(wts[0].branch, "main");
        assert_eq!(wts[1].branch, "feat/auth");
        assert_eq!(wts[0].owner, Some("git-worktree".into()));
    }

    #[test]
    fn parse_porcelain_empty() {
        assert!(parse_git_worktree_output("").is_empty());
    }

    #[test]
    fn parse_porcelain_no_trailing_newline() {
        let output = "worktree /path\nHEAD abc\nbranch refs/heads/main";
        let wts = parse_git_worktree_output(output);
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].branch, "main");
    }

    #[test]
    fn detect_in_non_git_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = detect_git_worktrees(dir.path()).unwrap();
        assert!(result.is_empty());
    }
}
