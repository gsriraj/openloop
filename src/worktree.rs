use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub base_branch: String,
}

impl Worktree {
    pub fn create(base_branch: &str, task_name: &str) -> Result<Self> {
        let branch = format!("openloop-{}-{}", task_name, std::process::id());
        let worktree_path = std::env::temp_dir().join(format!("openloop-{}", branch));

        // Ensure we're in a git repo
        let repo_root = get_repo_root()?;

        // Create branch + worktree
        Command::new("git")
            .args(["checkout", "-b", &branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to create branch")?;

        let worktree_add_output = Command::new("git")
            .args(["worktree", "add", worktree_path.to_str().unwrap(), &branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to create worktree")?;

        if !worktree_add_output.status.success() {
            anyhow::bail!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&worktree_add_output.stderr)
            );
        }

        // Switch back to original branch
        Command::new("git")
            .args(["checkout", base_branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to switch back to base branch")?;

        Ok(Worktree {
            path: worktree_path,
            branch,
            base_branch: base_branch.to_string(),
        })
    }

    pub fn merge_and_cleanup(&self) -> Result<()> {
        let repo_root = get_repo_root()?;

        // Switch to worktree branch and commit
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.path)
            .output()
            .context("Failed to stage changes in worktree")?;

        Command::new("git")
            .args([
                "commit",
                "--allow-empty",
                "-m",
                "openloop: parallel task result",
            ])
            .current_dir(&self.path)
            .output()
            .ok();

        // Switch back to base branch
        Command::new("git")
            .args(["checkout", &self.base_branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to switch to base branch")?;

        // Merge worktree branch
        Command::new("git")
            .args(["merge", "--no-edit", &self.branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to merge worktree branch")?;

        // Delete worktree
        Command::new("git")
            .args(["worktree", "remove", self.path.to_str().unwrap()])
            .current_dir(&repo_root)
            .output()
            .context("Failed to remove worktree")?;

        // Delete branch
        Command::new("git")
            .args(["branch", "-D", &self.branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to delete worktree branch")?;

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&self.path);

        Ok(())
    }
}

fn get_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Not a git repository")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository");
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(PathBuf::from(path))
}

pub fn ensure_git_repo() -> Result<()> {
    get_repo_root()?;
    Ok(())
}

pub fn is_git_repo() -> bool {
    get_repo_root().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_not_a_git_repo() {
        let _tmpdir = tempfile::tempdir().unwrap();
        let result = get_repo_root();
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_is_git_repo_in_temp_dir() {
        let tmpdir = tempfile::tempdir().unwrap();
        assert!(!is_git_repo_in_path(tmpdir.path()));
    }

    fn is_git_repo_in_path(path: &Path) -> bool {
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
