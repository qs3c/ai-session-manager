use crate::model::{SessionSource, SessionSummary};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn content_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| {
                if b.get("type").and_then(Value::as_str) == Some("text") {
                    b.get("text").and_then(Value::as_str).map(str::to_string)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

pub fn parse_session_file(path: &Path) -> Result<SessionSummary> {
    let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("bad filename")?
        .to_string();
    let project_dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .context("no parent dir")?
        .to_string();

    let mut cwd = String::new();
    let mut first_message = String::new();
    let mut message_count: u32 = 0;
    let mut started_at = String::new();
    let mut last_active_at = String::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let typ = v.get("type").and_then(Value::as_str).unwrap_or("");
        if typ != "user" && typ != "assistant" {
            continue;
        }
        if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(message) = v.get("message") else {
            continue;
        };
        message_count += 1;

        if cwd.is_empty() {
            if let Some(c) = v.get("cwd").and_then(Value::as_str) {
                cwd = c.to_string();
            }
        }
        if let Some(ts) = v.get("timestamp").and_then(Value::as_str) {
            if started_at.is_empty() {
                started_at = ts.to_string();
            }
            last_active_at = ts.to_string();
        }
        if first_message.is_empty() && typ == "user" {
            if let Some(content) = message.get("content") {
                let text = content_text(content);
                if !text.is_empty() && !text.starts_with('<') {
                    first_message = truncate_chars(&text, 200);
                }
            }
        }
    }

    anyhow::ensure!(message_count > 0, "no messages in {}", path.display());
    Ok(SessionSummary {
        session_id: session_id.clone(),
        source: SessionSource::ClaudeCode,
        cwd,
        transcript_path: path.to_string_lossy().into_owned(),
        relative_store_path: format!("{}/{}.jsonl", project_dir, session_id),
        first_message,
        message_count,
        started_at,
        last_active_at,
    })
}

pub fn scan(projects_root: &Path) -> Result<Vec<SessionSummary>> {
    let mut out = Vec::new();
    let Ok(projects) = fs::read_dir(projects_root) else {
        return Ok(out);
    };
    for proj in projects.flatten() {
        if !proj.path().is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(proj.path()) else {
            continue;
        };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                if let Ok(s) = parse_session_file(&p) {
                    out.push(s);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude")
    }

    #[test]
    fn parse_fixture_session() {
        let path =
            fixtures_root().join("E--fake-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl");
        let s = parse_session_file(&path).unwrap();
        assert_eq!(s.session_id, "11111111-aaaa-bbbb-cccc-000000000001");
        assert_eq!(s.cwd, r"E:\fake\proj");
        assert_eq!(s.first_message, "帮我修 bug");
        assert_eq!(s.message_count, 3);
        assert_eq!(s.started_at, "2026-07-05T02:46:20.453Z");
        assert_eq!(s.last_active_at, "2026-07-05T02:50:00.000Z");
        assert_eq!(
            s.relative_store_path,
            "E--fake-proj/11111111-aaaa-bbbb-cccc-000000000001.jsonl"
        );
        assert!(matches!(s.source, crate::model::SessionSource::ClaudeCode));
    }

    #[test]
    fn scan_dir_finds_all_sessions() {
        let sessions = scan(&fixtures_root()).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn scan_missing_dir_returns_empty() {
        let sessions = scan(&PathBuf::from("Z:/definitely/not/here")).unwrap();
        assert!(sessions.is_empty());
    }
}
