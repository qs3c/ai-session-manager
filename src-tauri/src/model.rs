use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionSource {
    Codex,
    ClaudeCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub source: SessionSource,
    pub cwd: String,
    pub transcript_path: String,
    pub relative_store_path: String,
    pub first_message: String,
    pub message_count: u32,
    pub started_at: String,
    pub last_active_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub schema_version: u32,
    pub session_id: String,
    pub source: SessionSource,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub starred: bool,
    pub origin_device_id: String,
    pub origin_cwd: String,
    pub relative_store_path: String,
    pub started_at: String,
    pub last_active_at: String,
    pub updated_at: String,
}

impl SessionMeta {
    pub fn from_summary(s: &SessionSummary, device_id: &str, now_iso: &str) -> SessionMeta {
        SessionMeta {
            schema_version: 1,
            session_id: s.session_id.clone(),
            source: s.source,
            title: s.first_message.clone(),
            description: String::new(),
            tags: vec![],
            starred: false,
            origin_device_id: device_id.to_string(),
            origin_cwd: s.cwd.clone(),
            relative_store_path: s.relative_store_path.clone(),
            started_at: s.started_at.clone(),
            last_active_at: s.last_active_at.clone(),
            updated_at: now_iso.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub hostname: String,
    pub os: String,
    #[serde(default)]
    pub path_mappings: Vec<crate::paths::PathMapping>,
    pub last_sync_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_json_round_trip() {
        let meta = SessionMeta {
            schema_version: 1,
            session_id: "abc-123".into(),
            source: SessionSource::ClaudeCode,
            title: "修复登录 bug".into(),
            description: String::new(),
            tags: vec!["work".into()],
            starred: true,
            origin_device_id: "dev-1".into(),
            origin_cwd: r"E:\fromGithub\bkcrab".into(),
            relative_store_path: r"E--fromGithub-bkcrab/abc-123.jsonl".into(),
            started_at: "2026-07-05T02:46:20.453Z".into(),
            last_active_at: "2026-07-05T03:00:00.000Z".into(),
            updated_at: "2026-07-05T03:00:01.000Z".into(),
        };
        let json = serde_json::to_string_pretty(&meta).unwrap();
        assert!(json.contains("\"claude-code\""));
        let back: SessionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "abc-123");
        assert!(back.starred);
    }

    #[test]
    fn meta_missing_optional_fields_defaults() {
        let json = r#"{
            "schema_version": 1, "session_id": "x", "source": "codex",
            "title": "t", "origin_device_id": "d", "origin_cwd": "/a",
            "relative_store_path": "p.jsonl",
            "started_at": "2026-01-01T00:00:00Z", "last_active_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let m: SessionMeta = serde_json::from_str(json).unwrap();
        assert_eq!(m.tags.len(), 0);
        assert!(!m.starred);
    }
}
