use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_git_in(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.email=ai-session-manager@local",
            "-c",
            "user.name=ai-session-manager",
        ])
        .args(args)
        .output()
        .context("failed to spawn git; is git installed and on PATH?")?;
    if !output.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub struct GitRepo {
    pub dir: PathBuf,
}

impl GitRepo {
    pub fn ensure_clone(url: &str, dest: &Path) -> Result<GitRepo> {
        if dest.join(".git").exists() {
            let repo = GitRepo {
                dir: dest.to_path_buf(),
            };
            match run_git_in(&repo.dir, &["remote", "get-url", "origin"]) {
                Ok(existing) if existing.trim() != url => {
                    run_git_in(&repo.dir, &["remote", "set-url", "origin", url])?;
                }
                Err(_) => {
                    run_git_in(&repo.dir, &["remote", "add", "origin", url])?;
                }
                _ => {}
            }
            return Ok(repo);
        }
        let parent = dest.parent().context("dest has no parent")?;
        std::fs::create_dir_all(parent)?;
        let dest_name = dest.file_name().context("bad dest")?.to_string_lossy();
        run_git_in(parent, &["clone", url, &dest_name])?;
        let repo = GitRepo {
            dir: dest.to_path_buf(),
        };
        let _ = run_git_in(&repo.dir, &["checkout", "-B", "main"]);
        Ok(repo)
    }

    pub fn commit_all(&self, message: &str) -> Result<bool> {
        run_git_in(&self.dir, &["add", "-A"])?;
        let status = run_git_in(&self.dir, &["status", "--porcelain"])?;
        if status.trim().is_empty() {
            return Ok(false);
        }
        run_git_in(&self.dir, &["commit", "-m", message])?;
        Ok(true)
    }

    pub fn push(&self) -> Result<()> {
        run_git_in(&self.dir, &["push", "-u", "origin", "main"])?;
        Ok(())
    }

    pub fn pull_rebase(&self) -> Result<()> {
        match run_git_in(&self.dir, &["pull", "--rebase", "origin", "main"]) {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("couldn't find remote ref")
                    || msg.contains("couldn't find remote ref main")
                    || msg.contains("fatal: couldn't find remote ref")
                {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_bare_remote(dir: &std::path::Path) -> String {
        let bare = dir.join("remote.git");
        run_git_in(dir, &["init", "--bare", "-b", "main", "remote.git"]).unwrap();
        bare.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn clone_commit_push_pull_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let remote_url = make_bare_remote(tmp.path());

        let a_dir = tmp.path().join("deviceA");
        let repo_a = GitRepo::ensure_clone(&remote_url, &a_dir).unwrap();
        fs::write(a_dir.join("hello.txt"), "v1").unwrap();
        assert!(repo_a.commit_all("add hello").unwrap());
        repo_a.push().unwrap();

        assert!(!repo_a.commit_all("nothing").unwrap());

        let b_dir = tmp.path().join("deviceB");
        let repo_b = GitRepo::ensure_clone(&remote_url, &b_dir).unwrap();
        assert_eq!(fs::read_to_string(b_dir.join("hello.txt")).unwrap(), "v1");

        fs::write(a_dir.join("hello.txt"), "v2").unwrap();
        repo_a.commit_all("update").unwrap();
        repo_a.push().unwrap();
        repo_b.pull_rebase().unwrap();
        assert_eq!(fs::read_to_string(b_dir.join("hello.txt")).unwrap(), "v2");

        let again = GitRepo::ensure_clone(&remote_url, &a_dir).unwrap();
        assert_eq!(again.dir, a_dir);
    }

    #[test]
    fn empty_remote_clone_then_first_push() {
        let tmp = tempfile::tempdir().unwrap();
        let remote_url = make_bare_remote(tmp.path());
        let dir = tmp.path().join("w");
        let repo = GitRepo::ensure_clone(&remote_url, &dir).unwrap();
        fs::write(dir.join("a.txt"), "x").unwrap();
        repo.commit_all("first").unwrap();
        repo.push().unwrap();
    }

    #[test]
    fn ensure_clone_updates_existing_origin_url() {
        let tmp = tempfile::tempdir().unwrap();
        let a_root = tmp.path().join("a");
        let b_root = tmp.path().join("b");
        fs::create_dir_all(&a_root).unwrap();
        fs::create_dir_all(&b_root).unwrap();
        let remote_a = make_bare_remote(&a_root);
        let remote_b = make_bare_remote(&b_root);
        let dir = tmp.path().join("mirror");

        GitRepo::ensure_clone(&remote_a, &dir).unwrap();
        GitRepo::ensure_clone(&remote_b, &dir).unwrap();

        let origin = run_git_in(&dir, &["remote", "get-url", "origin"]).unwrap();
        assert_eq!(origin.trim(), remote_b);
    }
}
