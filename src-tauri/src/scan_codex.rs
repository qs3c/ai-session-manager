use crate::model::{SessionSource, SessionSummary};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn input_text(payload: &Value) -> String {
    payload
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

pub fn parse_session_file(path: &Path, sessions_root: &Path) -> Result<SessionSummary> {
    let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = String::new();
    let mut cwd = String::new();
    let mut started_at = String::new();
    let mut first_message = String::new();
    let mut message_count: u32 = 0;
    let mut last_active_at = String::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let typ = v.get("type").and_then(Value::as_str).unwrap_or("");
        let Some(payload) = v.get("payload") else {
            continue;
        };

        match typ {
            "session_meta" => {
                session_id = payload
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                cwd = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                started_at = payload
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
            }
            "response_item" => {
                if payload.get("type").and_then(Value::as_str) != Some("message") {
                    continue;
                }
                let role = payload.get("role").and_then(Value::as_str).unwrap_or("");
                if role != "user" && role != "assistant" {
                    continue;
                }
                message_count += 1;
                if let Some(ts) = v.get("timestamp").and_then(Value::as_str) {
                    last_active_at = ts.to_string();
                }
                if first_message.is_empty() && role == "user" {
                    let text = input_text(payload);
                    if !text.is_empty() && !text.starts_with('<') {
                        first_message = truncate_chars(&text, 200);
                    }
                }
            }
            _ => {}
        }
    }

    anyhow::ensure!(!session_id.is_empty(), "no session_meta in {}", path.display());
    let relative_store_path = path
        .strip_prefix(sessions_root)
        .with_context(|| "path not under sessions root")?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(SessionSummary {
        session_id,
        source: SessionSource::Codex,
        cwd,
        transcript_path: path.to_string_lossy().into_owned(),
        relative_store_path,
        first_message,
        message_count,
        started_at,
        last_active_at,
    })
}

pub fn scan(sessions_root: &Path) -> Result<Vec<SessionSummary>> {
    let mut out = Vec::new();
    walk(sessions_root, sessions_root, &mut out);
    Ok(out)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<SessionSummary>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, root, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
            if let Ok(s) = parse_session_file(&p, root) {
                out.push(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex")
    }

    #[test]
    fn parse_fixture_rollout() {
        let root = fixtures_root();
        let path = root.join(
            "2026/06/21/rollout-2026-06-21T10-54-52-22222222-aaaa-bbbb-cccc-000000000002.jsonl",
        );
        let s = parse_session_file(&path, &root).unwrap();
        assert_eq!(s.session_id, "22222222-aaaa-bbbb-cccc-000000000002");
        assert_eq!(s.cwd, r"E:\fake\proj");
        assert_eq!(s.first_message, "帮我重构这个函数");
        assert_eq!(s.message_count, 3);
        assert_eq!(s.started_at, "2026-06-21T02:54:52.130Z");
        assert_eq!(s.last_active_at, "2026-06-21T02:55:10.000Z");
        assert_eq!(
            s.relative_store_path,
            "2026/06/21/rollout-2026-06-21T10-54-52-22222222-aaaa-bbbb-cccc-000000000002.jsonl"
        );
        assert!(matches!(s.source, crate::model::SessionSource::Codex));
    }

    #[test]
    fn scan_walks_date_tree() {
        let sessions = scan(&fixtures_root()).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn scan_missing_dir_returns_empty() {
        assert!(scan(&PathBuf::from("Z:/nope")).unwrap().is_empty());
    }
}
