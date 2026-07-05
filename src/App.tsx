import { useEffect, useState } from "react";
import { api, SessionMeta, SessionSummary } from "./api";
import "./App.css";
import { RestoreDialog } from "./components/RestoreDialog";
import { LocalList, RepoList } from "./components/SessionList";
import { SettingsPanel } from "./components/SettingsPanel";

type Tab = "local" | "repo" | "settings";

function App() {
  const [tab, setTab] = useState<Tab>("local");
  const [local, setLocal] = useState<SessionSummary[]>([]);
  const [repo, setRepo] = useState<SessionMeta[]>([]);
  const [restoring, setRestoring] = useState<SessionMeta | null>(null);
  const [error, setError] = useState("");

  const loadLocal = () =>
    api
      .listLocalSessions()
      .then((sessions) => {
        setLocal(sessions);
        setError("");
      })
      .catch((e) => setError(String(e)));

  const loadRepo = () =>
    api
      .listRepoSessions()
      .then((metas) => {
        setRepo(metas);
        setError("");
      })
      .catch((e) => setError(String(e)));

  useEffect(() => {
    loadLocal();
  }, []);

  useEffect(() => {
    if (tab === "repo") {
      loadRepo();
    }
  }, [tab]);

  const onUploaded = () => {
    loadLocal();
    loadRepo();
  };

  return (
    <main className="app-shell">
      <aside>
        <div className="brand">
          <strong>AI Session Manager</strong>
          <span>同步闭环</span>
        </div>
        <nav>
          <button className={tab === "local" ? "active" : ""} onClick={() => setTab("local")}>
            本机
          </button>
          <button className={tab === "repo" ? "active" : ""} onClick={() => setTab("repo")}>
            仓库
          </button>
          <button
            className={tab === "settings" ? "active" : ""}
            onClick={() => setTab("settings")}
          >
            设置
          </button>
        </nav>
      </aside>
      <section className="content">
        {error && <p className="error">{error}</p>}
        {tab === "local" && <LocalList sessions={local} onUploaded={onUploaded} />}
        {tab === "repo" && (
          <RepoList metas={repo} onChanged={loadRepo} onRestore={setRestoring} />
        )}
        {tab === "settings" && <SettingsPanel />}
      </section>
      {restoring && (
        <RestoreDialog
          meta={restoring}
          onClose={() => setRestoring(null)}
          onDone={() => {
            setRestoring(null);
            loadLocal();
            loadRepo();
          }}
        />
      )}
    </main>
  );
}

export default App;
