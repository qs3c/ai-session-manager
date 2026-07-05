use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathMapping {
    pub from_prefix: String,
    pub to_prefix: String,
}

pub fn munge_claude_project_dir(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn remap_path(path: &str, mappings: &[PathMapping]) -> Option<String> {
    let best = mappings
        .iter()
        .filter(|m| path.starts_with(m.from_prefix.as_str()))
        .max_by_key(|m| m.from_prefix.len())?;
    let rest = &path[best.from_prefix.len()..];
    let target_sep = if best.to_prefix.get(1..3) == Some(":\\")
        || best.to_prefix.get(1..3) == Some(":/")
    {
        '\\'
    } else {
        '/'
    };
    let rest_converted: String = rest
        .chars()
        .map(|c| {
            if c == '\\' || c == '/' {
                target_sep
            } else {
                c
            }
        })
        .collect();
    Some(format!("{}{}", best.to_prefix, rest_converted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn munge_windows_path() {
        assert_eq!(
            munge_claude_project_dir(r"E:\fromGithub\ai-session-manager"),
            "E--fromGithub-ai-session-manager"
        );
        assert_eq!(
            munge_claude_project_dir(r"C:\Users\27499\.understand-anything-repo"),
            "C--Users-27499--understand-anything-repo"
        );
    }

    #[test]
    fn munge_posix_path_preserves_case() {
        assert_eq!(
            munge_claude_project_dir("/Users/sybil/code/App"),
            "-Users-sybil-code-App"
        );
    }

    #[test]
    fn remap_longest_prefix_wins_and_converts_separators() {
        let maps = vec![
            PathMapping {
                from_prefix: r"E:\fromGithub".into(),
                to_prefix: "/Users/sybil/code".into(),
            },
            PathMapping {
                from_prefix: r"E:\fromGithub\special".into(),
                to_prefix: "/opt/special".into(),
            },
        ];
        assert_eq!(
            remap_path(r"E:\fromGithub\special\x", &maps).unwrap(),
            "/opt/special/x"
        );
        assert_eq!(
            remap_path(r"E:\fromGithub\bkcrab\sub", &maps).unwrap(),
            "/Users/sybil/code/bkcrab/sub"
        );
    }

    #[test]
    fn remap_posix_to_windows() {
        let maps = vec![PathMapping {
            from_prefix: "/Users/sybil/code".into(),
            to_prefix: r"E:\fromGithub".into(),
        }];
        assert_eq!(
            remap_path("/Users/sybil/code/app", &maps).unwrap(),
            r"E:\fromGithub\app"
        );
    }

    #[test]
    fn remap_no_match_returns_none() {
        assert!(remap_path("/nowhere", &[]).is_none());
    }
}
