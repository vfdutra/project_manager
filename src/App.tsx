import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Family } from "./api";
import { useRuntime } from "./useRuntime";
import { Sidebar, type Selection } from "./Sidebar";
import { ProjectPanel } from "./ProjectPanel";
import "./App.css";

export default function App() {
  const [families, setFamilies] = useState<Family[]>([]);
  const [selected, setSelected] = useState<Selection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rt = useRuntime();

  const refresh = () => api.families().then(setFamilies).catch((e) => setError(String(e)));
  useEffect(() => {
    refresh();
  }, []);

  const run = (p: Promise<unknown>) => {
    p.then(refresh).catch((e) => setError(String(e)));
  };
  const addFamily = async () => {
    const dir = await open({ directory: true, title: "Escolha a pasta da família (ex.: Node)" });
    if (typeof dir === "string") run(api.addFamily(dir));
  };
  const project = families.flatMap((f) => f.projects).find((p) => p.path === selected?.path) ?? null;

  return (
    <div className="app">
      <Sidebar
        families={families}
        rt={rt}
        selected={selected}
        onSelect={(path, tab = null) => setSelected({ path, tab })}
        onHide={(path, hidden) => run(api.setHidden(path, hidden))}
        onAdd={addFamily}
        onRemove={(path) => run(api.removeFamily(path))}
        onRefresh={refresh}
      />
      <main>
        {error && (
          <div className="error" title="Clique para fechar" onClick={() => setError(null)}>
            {error}
          </div>
        )}
        {project ? (
          <ProjectPanel
            key={project.path}
            project={project}
            rt={rt}
            run={run}
            tab={selected?.tab ?? null}
            onTab={(tab) => setSelected({ path: project.path, tab })}
          />
        ) : (
          <p className="empty">Selecione um projeto na barra lateral.</p>
        )}
      </main>
    </div>
  );
}
