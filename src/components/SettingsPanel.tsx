import { useEffect, useState } from "react";
import { api, PathMapping } from "../api";

export function SettingsPanel() {
  const [deviceId, setDeviceId] = useState("");
  const [repoUrl, setRepoUrl] = useState("");
  const [mappings, setMappings] = useState<PathMapping[]>([]);
  const [status, setStatus] = useState("");

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        setDeviceId(s.device_id);
        setRepoUrl(s.repo_url ?? "");
        setMappings(s.path_mappings);
      })
      .catch((e) => setStatus(`读取失败：${e}`));
  }, []);

  const save = async () => {
    setStatus("保存中");
    try {
      await api.saveSettings(
        repoUrl || null,
        mappings.filter((m) => m.from_prefix && m.to_prefix),
      );
      setStatus("已保存");
    } catch (e) {
      setStatus(`保存失败：${e}`);
    }
  };

  const setMapping = (i: number, key: keyof PathMapping, value: string) => {
    const next = mappings.slice();
    next[i] = { ...next[i], [key]: value };
    setMappings(next);
  };

  return (
    <section className="panel settings">
      <div className="panel-bar">
        <h2>设置</h2>
        <button className="primary" onClick={save}>
          保存
        </button>
      </div>
      <label className="field">
        <span>设备 ID</span>
        <input value={deviceId} readOnly />
      </label>
      <label className="field">
        <span>同步仓库</span>
        <input
          value={repoUrl}
          onChange={(e) => setRepoUrl(e.target.value)}
          placeholder="git@github.com:me/ai-sessions.git"
        />
      </label>
      <div className="mapping-header">
        <h3>路径映射</h3>
        <button onClick={() => setMappings([...mappings, { from_prefix: "", to_prefix: "" }])}>
          添加
        </button>
      </div>
      <div className="mapping-list">
        {mappings.map((m, i) => (
          <div className="mapping-row" key={i}>
            <input
              value={m.from_prefix}
              onChange={(e) => setMapping(i, "from_prefix", e.target.value)}
              placeholder="E:\fromGithub"
            />
            <span>→</span>
            <input
              value={m.to_prefix}
              onChange={(e) => setMapping(i, "to_prefix", e.target.value)}
              placeholder="/Users/me/code"
            />
            <button
              className="icon-button"
              aria-label="删除映射"
              onClick={() => setMappings(mappings.filter((_, j) => j !== i))}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
