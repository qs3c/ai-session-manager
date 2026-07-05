use crate::model::SessionMeta;
use crate::paths::munge_claude_project_dir;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    fs::create_dir_all(path.parent().context("no parent")?)?;
    fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

pub fn restore_claude(
    src: &Path,
    meta: &SessionMeta,
    target_cwd: &str,
    claude_home: &Path,
) -> Result<PathBuf> {
    let dest = claude_home
        .join("projects")
        .join(munge_claude_project_dir(target_cwd))
        .join(format!("{}.jsonl", meta.session_id));
    if dest.exists() {
        bail!("target already exists: {}", dest.display());
    }

    let text = fs::read_to_string(src)?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(mut v) => {
                if v.get("cwd").is_some() {
                    v["cwd"] = json!(target_cwd);
                }
                out.push(serde_json::to_string(&v)?);
            }
            Err(_) => out.push(line.to_string()),
        }
    }
    write_lines(&dest, &out)?;
    Ok(dest)
}

pub fn restore_codex(
    src: &Path,
    meta: &SessionMeta,
    target_cwd: &str,
    codex_home: &Path,
) -> Result<PathBuf> {
    let dest = codex_home.join("sessions").join(&meta.relative_store_path);
    if dest.exists() {
        bail!("target already exists: {}", dest.display());
    }

    let text = fs::read_to_string(src)?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(mut v) => {
                let typ = v.get("type").and_then(Value::as_str).unwrap_or("").to_string();
                if typ == "session_meta" || typ == "turn_context" {
                    if v["payload"].get("cwd").is_some() {
                        v["payload"]["cwd"] = json!(target_cwd);
                    }
                    if v["payload"].get("workspace_roots").is_some() {
                        v["payload"]["workspace_roots"] = json!([target_cwd]);
                    }
                }
                out.push(serde_json::to_string(&v)?);
            }
            Err(_) => out.push(line.to_string()),
        }
    }
    write_lines(&dest, &out)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionMeta, SessionSource};
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(rel)
    }

    fn claude_meta() -> SessionMeta {
        SessionMeta {
            schema_version: 1,
            session_id: "11111111-aaaa-bbbb-cccc-000000000001".into(),
            source: SessionSource::ClaudeCode,
            title: "t".into(),
            description: String::new(),
            tags: vec![],
            starred: false,
            origin_device_id: "dev-a".into(),
            origin_cwd: r"E:\fake\proj".into(),
            relative_store_path: "E--fake-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl".into(),
            started_at: "".into(),
            last_active_at: "".into(),
            updated_at: "2026-07-05T00:00:00Z".into(),
        }
    }

    #[test]
    fn restore_claude_rewrites_cwd_and_munges_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_home = tmp.path().join(".claude");
        let src = fixture("claude/E--fake-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl");

        let out = restore_claude(&src, &claude_meta(), "/Users/sybil/code/proj", &claude_home)
            .unwrap();
        assert!(out.ends_with(std::path::Path::new(
            "projects/-Users-sybil-code-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl"
        )));

        let text = fs::read_to_string(&out).unwrap();
        let mut saw_message = false;
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            if let Some(cwd) = v.get("cwd") {
                assert_eq!(cwd.as_str().unwrap(), "/Users/sybil/code/proj");
                saw_message = true;
            }
            if v.get("uuid").and_then(Value::as_str) == Some("u-1") {
                assert_eq!(v["message"]["content"].as_str().unwrap(), "帮我修 bug");
            }
        }
        assert!(saw_message);
    }

    #[test]
    fn restore_claude_refuses_to_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_home = tmp.path().join(".claude");
        let src = fixture("claude/E--fake-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl");
        restore_claude(&src, &claude_meta(), "/x", &claude_home).unwrap();
        assert!(restore_claude(&src, &claude_meta(), "/x", &claude_home).is_err());
    }

    #[test]
    fn restore_codex_rewrites_session_meta_and_turn_context() {
        let tmp = tempfile::tempdir().unwrap();
        let codex_home = tmp.path().join(".codex");
        let src = fixture(
            "codex/2026/06/21/rollout-2026-06-21T10-54-52-22222222-aaaa-bbbb-cccc-000000000002.jsonl",
        );
        let mut meta = claude_meta();
        meta.session_id = "22222222-aaaa-bbbb-cccc-000000000002".into();
        meta.source = SessionSource::Codex;
        meta.relative_store_path =
            "2026/06/21/rollout-2026-06-21T10-54-52-22222222-aaaa-bbbb-cccc-000000000002.jsonl"
                .into();

        let out = restore_codex(&src, &meta, "/Users/sybil/code/proj", &codex_home).unwrap();
        assert!(out.to_string_lossy().replace('\\', "/").ends_with(
            "sessions/2026/06/21/rollout-2026-06-21T10-54-52-22222222-aaaa-bbbb-cccc-000000000002.jsonl"
        ));

        let text = fs::read_to_string(&out).unwrap();
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            match v.get("type").and_then(Value::as_str) {
                Some("session_meta") => {
                    assert_eq!(v["payload"]["cwd"].as_str().unwrap(), "/Users/sybil/code/proj");
                }
                Some("turn_context") => {
                    assert_eq!(v["payload"]["cwd"].as_str().unwrap(), "/Users/sybil/code/proj");
                    assert_eq!(
                        v["payload"]["workspace_roots"][0].as_str().unwrap(),
                        "/Users/sybil/code/proj"
                    );
                }
                _ => {}
            }
        }
    }
}
