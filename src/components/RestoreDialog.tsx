import { useEffect, useState } from "react";
import { api, SessionMeta } from "../api";

export function RestoreDialog({
  meta,
  onClose,
  onDone,
}: {
  meta: SessionMeta;
  onClose: () => void;
  onDone: () => void;
}) {
  const [target, setTarget] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setTarget("");
    api
      .suggestRestorePath(meta.session_id)
      .then((p) => setTarget(p ?? meta.origin_cwd))
      .catch(() => setTarget(meta.origin_cwd));
  }, [meta.session_id, meta.origin_cwd]);

  const restore = async () => {
    setBusy(true);
    try {
      await api.restoreSession(meta.session_id, target);
      onDone();
    } catch (e) {
      window.alert(`恢复失败：${e}`);
      setBusy(false);
    }
  };

  return (
    <div className="dialog-backdrop" role="presentation">
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="restore-title">
        <header>
          <h2 id="restore-title">恢复会话</h2>
          <button className="icon-button" aria-label="关闭" onClick={onClose} disabled={busy}>
            ×
          </button>
        </header>
        <p className="dialog-title">{meta.title}</p>
        <label className="field">
          <span>原路径</span>
          <input value={meta.origin_cwd} readOnly />
        </label>
        <label className="field">
          <span>本机项目路径</span>
          <input value={target} onChange={(e) => setTarget(e.target.value)} />
        </label>
        <footer>
          <button onClick={onClose} disabled={busy}>
            取消
          </button>
          <button className="primary" onClick={restore} disabled={!target || busy}>
            恢复
          </button>
        </footer>
      </div>
    </div>
  );
}
