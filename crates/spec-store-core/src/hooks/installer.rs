use crate::error::{Result, SpecStoreError};
use std::{fs, path::Path};

const PRE_COMMIT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
echo "spec-store: running quality gates on staged files..."
spec-store quality check --staged || exit 1
spec-store catchup --staged --fail-on-missing || exit 1
"#;

const PRE_PUSH: &str = r#"#!/usr/bin/env bash
set -euo pipefail
echo "spec-store: checking worktree conflicts..."
spec-store worktree verify || exit 1
if [ -f lcov.info ]; then
  echo "spec-store: checking coverage gates..."
  spec-store coverage check || exit 1
else
  echo "spec-store: lcov.info not found, skipping coverage check (run cargo llvm-cov to generate)"
fi
"#;

const POST_MERGE: &str = r#"#!/usr/bin/env bash
set -euo pipefail
echo "spec-store: updating registry after merge..."
spec-store catchup --auto-register
spec-store coverage baseline --update
"#;

const POST_CHECKOUT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
PREV=$1; NEW=$2; IS_BRANCH=$3
if [ "$IS_BRANCH" = "1" ] && [ "$PREV" != "$NEW" ]; then
  echo "spec-store: regenerating agent context for $(git branch --show-current)..."
  spec-store context --output AGENTS.md
fi
"#;

pub struct HookSet {
    pub pre_commit: &'static str,
    pub pre_push: &'static str,
    pub post_merge: &'static str,
    pub post_checkout: &'static str,
}

impl Default for HookSet {
    fn default() -> Self {
        Self {
            pre_commit: PRE_COMMIT,
            pre_push: PRE_PUSH,
            post_merge: POST_MERGE,
            post_checkout: POST_CHECKOUT,
        }
    }
}

pub fn install(root: &Path, hooks: &HookSet) -> Result<()> {
    let hooks_dir = root.join(".githooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| SpecStoreError::HookInstall(e.to_string()))?;

    write_hook(&hooks_dir.join("pre-commit"), hooks.pre_commit)?;
    write_hook(&hooks_dir.join("pre-push"), hooks.pre_push)?;
    write_hook(&hooks_dir.join("post-merge"), hooks.post_merge)?;
    write_hook(&hooks_dir.join("post-checkout"), hooks.post_checkout)?;

    configure_git_hooks_path(root, &hooks_dir)
}

fn write_hook(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)
        .map_err(|e| SpecStoreError::HookInstall(format!("{}: {e}", path.display())))?;
    set_executable(path)
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| SpecStoreError::HookInstall(e.to_string()))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).map_err(|e| SpecStoreError::HookInstall(e.to_string()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn configure_git_hooks_path(root: &Path, hooks_dir: &Path) -> Result<()> {
    let rel = hooks_dir.strip_prefix(root).unwrap_or(hooks_dir);
    let status = std::process::Command::new("git")
        .args(["config", "core.hooksPath", &rel.to_string_lossy()])
        .current_dir(root)
        .status()
        .map_err(|e| SpecStoreError::HookInstall(format!("git config failed: {e}")))?;
    if !status.success() {
        return Err(SpecStoreError::HookInstall(
            "git config core.hooksPath failed — is this a git repo?".into(),
        ));
    }
    Ok(())
}

pub fn verify_hooks_installed(root: &Path) -> bool {
    let hooks_dir = root.join(".githooks");
    ["pre-commit", "pre-push", "post-merge", "post-checkout"]
        .iter()
        .all(|h| hooks_dir.join(h).exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    #[test]
    fn install_creates_hook_files() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        let hooks = HookSet::default();
        install(dir.path(), &hooks).unwrap();
        assert!(verify_hooks_installed(dir.path()));
    }

    #[test]
    fn hook_files_contain_spec_store_calls() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        install(dir.path(), &HookSet::default()).unwrap();
        let content = fs::read_to_string(dir.path().join(".githooks/pre-commit")).unwrap();
        assert!(content.contains("spec-store"));
    }

    #[test]
    fn verify_returns_false_when_not_installed() {
        let dir = TempDir::new().unwrap();
        assert!(!verify_hooks_installed(dir.path()));
    }

    #[test]
    fn verify_returns_true_after_install() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        install(dir.path(), &HookSet::default()).unwrap();
        assert!(verify_hooks_installed(dir.path()));
    }

    #[test]
    fn pre_push_hook_checks_coverage() {
        assert!(PRE_PUSH.contains("coverage check"));
        assert!(PRE_PUSH.contains("worktree verify"));
    }

    #[test]
    fn post_checkout_hook_regenerates_agents_md() {
        assert!(POST_CHECKOUT.contains("AGENTS.md"));
    }
}
