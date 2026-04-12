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

    let stdout = String::from_utf8_lossy(&output.stdout);
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
    Ok(worktrees)
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
