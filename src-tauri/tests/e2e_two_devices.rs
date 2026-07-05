use ai_session_manager_lib::gitcli::run_git_in;
use ai_session_manager_lib::model::SessionSource;
use ai_session_manager_lib::paths::{remap_path, PathMapping};
use ai_session_manager_lib::restore::{restore_claude, restore_codex};
use ai_session_manager_lib::scan_claude;
use ai_session_manager_lib::scan_codex;
use ai_session_manager_lib::sync::SyncEngine;
use std::fs;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn two_device_upload_and_restore_round_trip() {
    let tmp = tempfile::tempdir().unwrap();

    run_git_in(tmp.path(), &["init", "--bare", "-b", "main", "remote.git"]).unwrap();
    let remote = tmp.path().join("remote.git").to_string_lossy().replace('\\', "/");

    let mut sessions = scan_claude::scan(&fixtures().join("claude")).unwrap();
    sessions.extend(scan_codex::scan(&fixtures().join("codex")).unwrap());
    assert_eq!(sessions.len(), 2);

    let engine_a = SyncEngine::connect(&remote, &tmp.path().join("mirrorA")).unwrap();
    let n = engine_a
        .upload(&sessions, "dev-a", "2026-07-05T10:00:00.000Z")
        .unwrap();
    assert_eq!(n, 2);

    let claude_id = "11111111-aaaa-bbbb-cccc-000000000001";
    let mut meta = engine_a.repo().read_meta(claude_id).unwrap();
    meta.title = "登录 bug 修复".into();
    meta.starred = true;
    meta.updated_at = "2026-07-05T10:30:00.000Z".into();
    engine_a.update_meta(&meta).unwrap();

    let engine_b = SyncEngine::connect(&remote, &tmp.path().join("mirrorB")).unwrap();
    engine_b.refresh().unwrap();
    let metas = engine_b.repo().list_metas().unwrap();
    assert_eq!(metas.len(), 2);
    let claude_meta = metas.iter().find(|m| m.session_id == claude_id).unwrap();
    assert_eq!(claude_meta.title, "登录 bug 修复");
    assert!(claude_meta.starred);

    let mappings = vec![PathMapping {
        from_prefix: r"E:\fake".into(),
        to_prefix: "/Users/b/work".into(),
    }];
    let target = remap_path(&claude_meta.origin_cwd, &mappings).unwrap();
    assert_eq!(target, "/Users/b/work/proj");

    let home_b = tmp.path().join("homeB");
    let out_claude = restore_claude(
        &engine_b.repo().transcript_path(claude_id),
        claude_meta,
        &target,
        &home_b.join(".claude"),
    )
    .unwrap();
    assert!(out_claude
        .to_string_lossy()
        .replace('\\', "/")
        .contains("/projects/-Users-b-work-proj/"));
    assert!(fs::read_to_string(&out_claude)
        .unwrap()
        .contains("/Users/b/work/proj"));

    let codex_meta = metas
        .iter()
        .find(|m| matches!(m.source, SessionSource::Codex))
        .unwrap();
    let out_codex = restore_codex(
        &engine_b.repo().transcript_path(&codex_meta.session_id),
        codex_meta,
        &target,
        &home_b.join(".codex"),
    )
    .unwrap();
    assert!(out_codex
        .to_string_lossy()
        .replace('\\', "/")
        .contains("/sessions/2026/06/21/"));

    assert!(restore_claude(
        &engine_b.repo().transcript_path(claude_id),
        claude_meta,
        &target,
        &home_b.join(".claude"),
    )
    .is_err());
}
