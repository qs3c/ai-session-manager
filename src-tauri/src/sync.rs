use crate::gitcli::GitRepo;
use crate::model::{SessionMeta, SessionSummary};
use crate::syncrepo::SyncRepo;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct SyncEngine {
    git: GitRepo,
    repo: SyncRepo,
}

impl SyncEngine {
    pub fn connect(remote_url: &str, mirror_dir: &Path) -> Result<SyncEngine> {
        let git = GitRepo::ensure_clone(remote_url, mirror_dir)?;
        let repo = SyncRepo {
            root: git.dir.clone(),
        };
        Ok(SyncEngine { git, repo })
    }

    pub fn repo(&self) -> &SyncRepo {
        &self.repo
    }

    pub fn refresh(&self) -> Result<()> {
        self.git.pull_rebase()
    }

    pub fn upload(
        &self,
        sessions: &[SessionSummary],
        device_id: &str,
        now_iso: &str,
    ) -> Result<usize> {
        self.refresh()?;
        let mut written = 0usize;
        for s in sessions {
            let repo_transcript = self.repo.transcript_path(&s.session_id);
            let local_bytes = fs::read(&s.transcript_path)
                .with_context(|| format!("read {}", s.transcript_path))?;
            let unchanged = repo_transcript.exists()
                && fs::read(&repo_transcript)
                    .map(|b| b == local_bytes)
                    .unwrap_or(false);
            let meta_exists = self.repo.read_meta(&s.session_id).is_ok();
            if unchanged && meta_exists {
                continue;
            }
            if !meta_exists {
                let meta = SessionMeta::from_summary(s, device_id, now_iso);
                self.repo
                    .write_session(Path::new(&s.transcript_path), &meta)?;
            } else {
                fs::create_dir_all(repo_transcript.parent().context("transcript has no parent")?)?;
                fs::write(&repo_transcript, &local_bytes)?;
                let mut meta = self.repo.read_meta(&s.session_id)?;
                meta.last_active_at = s.last_active_at.clone();
                meta.updated_at = now_iso.to_string();
                self.repo.write_meta(&meta)?;
            }
            written += 1;
        }
        if written > 0 {
            let msg = format!("sync: upload {} session(s) from {}", written, device_id);
            if self.git.commit_all(&msg)? {
                self.git.push()?;
            }
        }
        Ok(written)
    }

    pub fn update_meta(&self, meta: &SessionMeta) -> Result<()> {
        self.refresh()?;
        self.repo.write_meta(meta)?;
        if self
            .git
            .commit_all(&format!("meta: update {}", meta.session_id))?
        {
            self.git.push()?;
        }
        Ok(())
    }

    pub fn commit_and_push(&self, message: &str) -> Result<()> {
        if self.git.commit_all(message)? {
            self.git.push()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitcli::run_git_in;
    use crate::model::{SessionSource, SessionSummary};
    use std::fs;

    fn make_bare_remote(dir: &std::path::Path) -> String {
        run_git_in(dir, &["init", "--bare", "-b", "main", "remote.git"]).unwrap();
        dir.join("remote.git").to_string_lossy().replace('\\', "/")
    }

    fn sample_summary(transcript: &std::path::Path) -> SessionSummary {
        SessionSummary {
            session_id: "s-1".into(),
            source: SessionSource::ClaudeCode,
            cwd: r"E:\fake\proj".into(),
            transcript_path: transcript.to_string_lossy().into_owned(),
            relative_store_path: "E--fake-proj/s-1.jsonl".into(),
            first_message: "帮我修 bug".into(),
            message_count: 3,
            started_at: "2026-07-05T02:46:20.453Z".into(),
            last_active_at: "2026-07-05T02:50:00.000Z".into(),
        }
    }

    #[test]
    fn upload_then_second_device_sees_session() {
        let tmp = tempfile::tempdir().unwrap();
        let remote = make_bare_remote(tmp.path());
        let transcript = tmp.path().join("orig.jsonl");
        fs::write(&transcript, "{\"a\":1}\n").unwrap();

        let engine_a = SyncEngine::connect(&remote, &tmp.path().join("mirrorA")).unwrap();
        let n = engine_a
            .upload(
                &[sample_summary(&transcript)],
                "dev-a",
                "2026-07-05T10:00:00Z",
            )
            .unwrap();
        assert_eq!(n, 1);

        let engine_b = SyncEngine::connect(&remote, &tmp.path().join("mirrorB")).unwrap();
        engine_b.refresh().unwrap();
        let metas = engine_b.repo().list_metas().unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].title, "帮我修 bug");
        assert_eq!(metas[0].origin_device_id, "dev-a");
    }

    #[test]
    fn reupload_longer_transcript_overwrites_but_keeps_meta_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let remote = make_bare_remote(tmp.path());
        let transcript = tmp.path().join("orig.jsonl");
        fs::write(&transcript, "{\"a\":1}\n").unwrap();

        let engine = SyncEngine::connect(&remote, &tmp.path().join("mirror")).unwrap();
        engine
            .upload(
                &[sample_summary(&transcript)],
                "dev-a",
                "2026-07-05T10:00:00Z",
            )
            .unwrap();

        let mut meta = engine.repo().read_meta("s-1").unwrap();
        meta.title = "我的重要会话".into();
        meta.updated_at = "2026-07-05T11:00:00Z".into();
        engine.repo().write_meta(&meta).unwrap();

        fs::write(&transcript, "{\"a\":1}\n{\"a\":2}\n").unwrap();
        let n = engine
            .upload(
                &[sample_summary(&transcript)],
                "dev-a",
                "2026-07-05T12:00:00Z",
            )
            .unwrap();
        assert_eq!(n, 1);

        let text = fs::read_to_string(engine.repo().transcript_path("s-1")).unwrap();
        assert!(text.contains("{\"a\":2}"));
        assert_eq!(
            engine.repo().read_meta("s-1").unwrap().title,
            "我的重要会话"
        );
    }

    #[test]
    fn upload_unchanged_transcript_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let remote = make_bare_remote(tmp.path());
        let transcript = tmp.path().join("orig.jsonl");
        fs::write(&transcript, "{\"a\":1}\n").unwrap();

        let engine = SyncEngine::connect(&remote, &tmp.path().join("mirror")).unwrap();
        engine
            .upload(
                &[sample_summary(&transcript)],
                "dev-a",
                "2026-07-05T10:00:00Z",
            )
            .unwrap();
        let n = engine
            .upload(
                &[sample_summary(&transcript)],
                "dev-a",
                "2026-07-05T10:05:00Z",
            )
            .unwrap();
        assert_eq!(n, 0);
    }
}
