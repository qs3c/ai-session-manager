pub mod config;
pub mod gitcli;
pub mod model;
pub mod paths;
pub mod restore;
pub mod scan_claude;
pub mod scan_codex;
pub mod sync;
pub mod syncrepo;

use model::{DeviceInfo, SessionMeta, SessionSummary};
use paths::PathMapping;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sync::SyncEngine;
use tauri::Manager;

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into())
}

struct AppDirs {
    config_dir: PathBuf,
    mirror_dir: PathBuf,
    claude_projects: PathBuf,
    codex_sessions: PathBuf,
}

fn app_dirs(app: &tauri::AppHandle) -> Result<AppDirs, String> {
    let home = dirs::home_dir().ok_or("no home dir")?;
    Ok(AppDirs {
        config_dir: app.path().app_config_dir().map_err(|e| e.to_string())?,
        mirror_dir: app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("mirror"),
        claude_projects: home.join(".claude").join("projects"),
        codex_sessions: home.join(".codex").join("sessions"),
    })
}

fn engine(app: &tauri::AppHandle) -> Result<(SyncEngine, config::AppConfig), String> {
    let dirs = app_dirs(app)?;
    let cfg = config::AppConfig::load_or_init(&dirs.config_dir).map_err(|e| e.to_string())?;
    let url = cfg.repo_url.clone().ok_or("repo_url not configured")?;
    let eng = SyncEngine::connect(&url, &dirs.mirror_dir).map_err(|e| e.to_string())?;
    Ok((eng, cfg))
}

#[derive(Serialize, Deserialize)]
struct Settings {
    device_id: String,
    repo_url: Option<String>,
    path_mappings: Vec<PathMapping>,
}

#[tauri::command]
fn list_local_sessions(app: tauri::AppHandle) -> Result<Vec<SessionSummary>, String> {
    let dirs = app_dirs(&app)?;
    let mut out = scan_claude::scan(&dirs.claude_projects).map_err(|e| e.to_string())?;
    out.extend(scan_codex::scan(&dirs.codex_sessions).map_err(|e| e.to_string())?);
    out.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
    Ok(out)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let dirs = app_dirs(&app)?;
    let cfg = config::AppConfig::load_or_init(&dirs.config_dir).map_err(|e| e.to_string())?;
    let path_mappings = match engine(&app) {
        Ok((eng, _)) => eng
            .repo()
            .read_device_info(&cfg.device_id)
            .ok()
            .flatten()
            .map(|d| d.path_mappings)
            .unwrap_or_default(),
        Err(_) => vec![],
    };
    Ok(Settings {
        device_id: cfg.device_id,
        repo_url: cfg.repo_url,
        path_mappings,
    })
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    repo_url: Option<String>,
    path_mappings: Vec<PathMapping>,
) -> Result<(), String> {
    let dirs = app_dirs(&app)?;
    let mut cfg = config::AppConfig::load_or_init(&dirs.config_dir).map_err(|e| e.to_string())?;
    cfg.repo_url = repo_url;
    cfg.save(&dirs.config_dir).map_err(|e| e.to_string())?;
    if cfg.repo_url.is_some() {
        let (eng, cfg) = engine(&app)?;
        eng.refresh().map_err(|e| e.to_string())?;
        let info = DeviceInfo {
            device_id: cfg.device_id.clone(),
            hostname: hostname(),
            os: std::env::consts::OS.into(),
            path_mappings,
            last_sync_at: now_iso(),
        };
        eng.repo()
            .write_device_info(&info)
            .map_err(|e| e.to_string())?;
        eng.commit_and_push("devices: update mappings")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn list_repo_sessions(app: tauri::AppHandle) -> Result<Vec<SessionMeta>, String> {
    let (eng, _) = engine(&app)?;
    eng.refresh().map_err(|e| e.to_string())?;
    eng.repo().list_metas().map_err(|e| e.to_string())
}

#[tauri::command]
fn upload_sessions(app: tauri::AppHandle, session_ids: Vec<String>) -> Result<usize, String> {
    let (eng, cfg) = engine(&app)?;
    let all = list_local_sessions(app.clone())?;
    let selected: Vec<_> = all
        .into_iter()
        .filter(|s| session_ids.contains(&s.session_id))
        .collect();
    eng.upload(&selected, &cfg.device_id, &now_iso())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_session_meta(app: tauri::AppHandle, mut meta: SessionMeta) -> Result<(), String> {
    let (eng, _) = engine(&app)?;
    meta.updated_at = now_iso();
    eng.update_meta(&meta).map_err(|e| e.to_string())
}

#[tauri::command]
fn suggest_restore_path(app: tauri::AppHandle, session_id: String) -> Result<Option<String>, String> {
    let (eng, cfg) = engine(&app)?;
    let meta = eng.repo().read_meta(&session_id).map_err(|e| e.to_string())?;
    let mappings = eng
        .repo()
        .read_device_info(&cfg.device_id)
        .ok()
        .flatten()
        .map(|d| d.path_mappings)
        .unwrap_or_default();
    Ok(paths::remap_path(&meta.origin_cwd, &mappings))
}

#[tauri::command]
fn restore_session(
    app: tauri::AppHandle,
    session_id: String,
    target_cwd: String,
) -> Result<String, String> {
    let (eng, _) = engine(&app)?;
    eng.refresh().map_err(|e| e.to_string())?;
    let meta = eng.repo().read_meta(&session_id).map_err(|e| e.to_string())?;
    let src = eng.repo().transcript_path(&session_id);
    let home = dirs::home_dir().ok_or("no home dir")?;
    let dest = match meta.source {
        model::SessionSource::ClaudeCode => {
            restore::restore_claude(&src, &meta, &target_cwd, &home.join(".claude"))
        }
        model::SessionSource::Codex => {
            restore::restore_codex(&src, &meta, &target_cwd, &home.join(".codex"))
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_local_sessions,
            get_settings,
            save_settings,
            list_repo_sessions,
            upload_sessions,
            update_session_meta,
            suggest_restore_path,
            restore_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
