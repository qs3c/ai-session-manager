import { invoke } from "@tauri-apps/api/core";

export type SessionSource = "codex" | "claude-code";

export interface SessionSummary {
  session_id: string;
  source: SessionSource;
  cwd: string;
  transcript_path: string;
  relative_store_path: string;
  first_message: string;
  message_count: number;
  started_at: string;
  last_active_at: string;
}

export interface SessionMeta {
  schema_version: number;
  session_id: string;
  source: SessionSource;
  title: string;
  description: string;
  tags: string[];
  starred: boolean;
  origin_device_id: string;
  origin_cwd: string;
  relative_store_path: string;
  started_at: string;
  last_active_at: string;
  updated_at: string;
}

export interface PathMapping {
  from_prefix: string;
  to_prefix: string;
}

export interface Settings {
  device_id: string;
  repo_url: string | null;
  path_mappings: PathMapping[];
}

export const api = {
  listLocalSessions: () => invoke<SessionSummary[]>("list_local_sessions"),
  listRepoSessions: () => invoke<SessionMeta[]>("list_repo_sessions"),
  uploadSessions: (sessionIds: string[]) =>
    invoke<number>("upload_sessions", { sessionIds }),
  updateSessionMeta: (meta: SessionMeta) =>
    invoke<void>("update_session_meta", { meta }),
  suggestRestorePath: (sessionId: string) =>
    invoke<string | null>("suggest_restore_path", { sessionId }),
  restoreSession: (sessionId: string, targetCwd: string) =>
    invoke<string>("restore_session", { sessionId, targetCwd }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (repoUrl: string | null, pathMappings: PathMapping[]) =>
    invoke<void>("save_settings", { repoUrl, pathMappings }),
};
