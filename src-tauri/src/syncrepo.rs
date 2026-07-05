use crate::model::{DeviceInfo, SessionMeta};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SyncRepo {
    pub root: PathBuf,
}

impl SyncRepo {
    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.root.join("sessions").join(session_id)
    }

    pub fn write_session(&self, transcript_src: &Path, meta: &SessionMeta) -> Result<()> {
        let dir = self.session_dir(&meta.session_id);
        fs::create_dir_all(&dir)?;
        fs::copy(transcript_src, dir.join("transcript.jsonl"))
            .with_context(|| format!("copy {}", transcript_src.display()))?;
        self.write_meta(meta)
    }

    pub fn write_meta(&self, meta: &SessionMeta) -> Result<()> {
        let dir = self.session_dir(&meta.session_id);
        fs::create_dir_all(&dir)?;
        let path = dir.join("meta.json");
        if path.exists() {
            if let Ok(existing) = self.read_meta(&meta.session_id) {
                if existing.updated_at.as_str() >= meta.updated_at.as_str() {
                    return Ok(());
                }
            }
        }
        fs::write(&path, serde_json::to_string_pretty(meta)?)?;
        Ok(())
    }

    pub fn read_meta(&self, session_id: &str) -> Result<SessionMeta> {
        let text = fs::read_to_string(self.session_dir(session_id).join("meta.json"))?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn transcript_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("transcript.jsonl")
    }

    pub fn list_metas(&self) -> Result<Vec<SessionMeta>> {
        let mut out = Vec::new();
        let sessions = self.root.join("sessions");
        let Ok(entries) = fs::read_dir(&sessions) else {
            return Ok(out);
        };
        for e in entries.flatten() {
            if let Some(id) = e.file_name().to_str() {
                if let Ok(m) = self.read_meta(id) {
                    out.push(m);
                }
            }
        }
        out.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        Ok(out)
    }

    pub fn write_device_info(&self, info: &DeviceInfo) -> Result<()> {
        let dir = self.root.join("devices");
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join(format!("{}.json", info.device_id)),
            serde_json::to_string_pretty(info)?,
        )?;
        Ok(())
    }

    pub fn read_device_info(&self, device_id: &str) -> Result<Option<DeviceInfo>> {
        let path = self.root.join("devices").join(format!("{}.json", device_id));
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&text)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionMeta, SessionSource};
    use std::fs;

    fn sample_meta(updated_at: &str) -> SessionMeta {
        SessionMeta {
            schema_version: 1,
            session_id: "s-1".into(),
            source: SessionSource::Codex,
            title: "t".into(),
            description: String::new(),
            tags: vec![],
            starred: false,
            origin_device_id: "dev-a".into(),
            origin_cwd: r"E:\fake\proj".into(),
            relative_store_path: "2026/06/21/rollout-x.jsonl".into(),
            started_at: "2026-06-21T02:54:52.130Z".into(),
            last_active_at: "2026-06-21T02:55:10.000Z".into(),
            updated_at: updated_at.into(),
        }
    }

    #[test]
    fn write_session_copies_transcript_and_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("orig.jsonl");
        fs::write(&src, "{\"line\":1}\n").unwrap();

        let repo = SyncRepo {
            root: tmp.path().join("repo"),
        };
        repo.write_session(&src, &sample_meta("2026-07-05T00:00:00Z"))
            .unwrap();

        let dir = repo.root.join("sessions/s-1");
        assert_eq!(
            fs::read_to_string(dir.join("transcript.jsonl")).unwrap(),
            "{\"line\":1}\n"
        );
        let metas = repo.list_metas().unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].session_id, "s-1");
    }

    #[test]
    fn meta_lww_older_write_is_rejected_newer_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = SyncRepo {
            root: tmp.path().to_path_buf(),
        };

        repo.write_meta(&sample_meta("2026-07-05T10:00:00Z"))
            .unwrap();
        let mut older = sample_meta("2026-07-05T09:00:00Z");
        older.title = "旧标题".into();
        repo.write_meta(&older).unwrap();
        assert_eq!(repo.read_meta("s-1").unwrap().title, "t");
        let mut newer = sample_meta("2026-07-05T11:00:00Z");
        newer.title = "新标题".into();
        repo.write_meta(&newer).unwrap();
        assert_eq!(repo.read_meta("s-1").unwrap().title, "新标题");
    }

    #[test]
    fn device_info_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = SyncRepo {
            root: tmp.path().to_path_buf(),
        };
        let info = crate::model::DeviceInfo {
            device_id: "dev-a".into(),
            hostname: "PC-A".into(),
            os: "windows".into(),
            path_mappings: vec![crate::paths::PathMapping {
                from_prefix: "/Users/x/code".into(),
                to_prefix: r"E:\fromGithub".into(),
            }],
            last_sync_at: "2026-07-05T00:00:00Z".into(),
        };
        repo.write_device_info(&info).unwrap();
        let back = repo.read_device_info("dev-a").unwrap().unwrap();
        assert_eq!(back.path_mappings.len(), 1);
        assert!(repo.read_device_info("dev-zzz").unwrap().is_none());
    }
}
