import { useState } from "react";
import { api, SessionMeta, SessionSummary } from "../api";

function sourceBadge(source: string) {
  return (
    <span className={`source-badge ${source === "codex" ? "codex" : "claude"}`}>
      <span aria-hidden className="source-dot" />
      {source === "codex" ? "Codex" : "Claude"}
    </span>
  );
}

function shortDate(value: string) {
  return value ? value.slice(0, 16).replace("T", " ") : "unknown";
}

export function LocalList({
  sessions,
  onUploaded,
}: {
  sessions: SessionSummary[];
  onUploaded: () => void;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  const toggle = (id: string) => {
    const next = new Set(selected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelected(next);
  };

  const upload = async () => {
    setBusy(true);
    try {
      await api.uploadSessions([...selected]);
      setSelected(new Set());
      onUploaded();
    } catch (e) {
      window.alert(`上传失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="panel">
      <div className="panel-bar">
        <h2>本机会话</h2>
        <button className="primary" disabled={selected.size === 0 || busy} onClick={upload}>
          上传选中
          <span>{selected.size}</span>
        </button>
      </div>
      <ul className="session-list">
        {sessions.map((s) => (
          <li key={s.session_id} className="session-row">
            <label className="check-cell">
              <input
                type="checkbox"
                checked={selected.has(s.session_id)}
                onChange={() => toggle(s.session_id)}
              />
            </label>
            <div className="session-main">
              <div className="row-top">
                {sourceBadge(s.source)}
                <strong>{s.first_message || "(无标题)"}</strong>
              </div>
              <div className="muted path-line">{s.cwd}</div>
            </div>
            <div className="session-meta">
              <span>{s.message_count} 条</span>
              <span>{shortDate(s.last_active_at)}</span>
            </div>
          </li>
        ))}
      </ul>
      {sessions.length === 0 && <p className="empty">暂无本机会话</p>}
    </section>
  );
}

export function RepoList({
  metas,
  onChanged,
  onRestore,
}: {
  metas: SessionMeta[];
  onChanged: () => void;
  onRestore: (meta: SessionMeta) => void;
}) {
  const [savingId, setSavingId] = useState<string | null>(null);

  const save = async (meta: SessionMeta, patch: Partial<SessionMeta>) => {
    setSavingId(meta.session_id);
    try {
      await api.updateSessionMeta({ ...meta, ...patch });
      onChanged();
    } catch (e) {
      window.alert(`保存失败：${e}`);
    } finally {
      setSavingId(null);
    }
  };

  const saveTags = (meta: SessionMeta, value: string) => {
    const tags = value
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean);
    if (tags.join(",") !== meta.tags.join(",")) {
      save(meta, { tags });
    }
  };

  return (
    <section className="panel">
      <div className="panel-bar">
        <h2>仓库会话</h2>
        <button onClick={onChanged}>刷新</button>
      </div>
      <ul className="repo-list">
        {metas.map((m) => (
          <li key={m.session_id} className="repo-row">
            <button
              className="icon-button"
              aria-label={m.starred ? "取消星标" : "加星"}
              disabled={savingId === m.session_id}
              onClick={() => save(m, { starred: !m.starred })}
            >
              {m.starred ? "★" : "☆"}
            </button>
            <div className="repo-fields">
              <div className="row-top">
                {sourceBadge(m.source)}
                <input
                  className="title-input"
                  defaultValue={m.title}
                  onBlur={(e) => e.target.value !== m.title && save(m, { title: e.target.value })}
                />
              </div>
              <input
                className="plain-input"
                placeholder="描述"
                defaultValue={m.description}
                onBlur={(e) =>
                  e.target.value !== m.description && save(m, { description: e.target.value })
                }
              />
              <input
                className="plain-input"
                placeholder="标签，用逗号分隔"
                defaultValue={m.tags.join(", ")}
                onBlur={(e) => saveTags(m, e.target.value)}
              />
              <div className="muted path-line">{m.origin_cwd}</div>
            </div>
            <div className="repo-actions">
              <span className="muted">{shortDate(m.last_active_at)}</span>
              <button onClick={() => onRestore(m)}>恢复</button>
            </div>
          </li>
        ))}
      </ul>
      {metas.length === 0 && <p className="empty">暂无仓库会话</p>}
    </section>
  );
}
