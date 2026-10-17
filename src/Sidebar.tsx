import { useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { subKey, type Family, type Project } from "./api";
import { type Runtime, cmdStatus, commandUrl, projectRunning, projectStatus, projectUrl, portOf } from "./runtime";
import { EyeIcon, EyeOffIcon, FolderIcon } from "./icons";

/** Selected project, and optionally one of its commands (sub-project). */
export type Selection = { path: string; tab: string | null };

type Props = {
  families: Family[];
  rt: Runtime;
  selected: Selection | null;
  onSelect: (path: string, tab?: string | null) => void;
  onHide: (key: string, hidden: boolean) => void;
  onAdd: () => void;
  onRemove: (path: string) => void;
  onRefresh: () => void;
};

const HideButton = ({ hidden, onClick }: { hidden: boolean; onClick: () => void }) => {
  const label = hidden ? "Mostrar na lista" : "Ocultar da lista";
  return (
    <button className="row-action" title={label} aria-label={label} onClick={onClick}>
      {hidden ? <EyeIcon size={14} /> : <EyeOffIcon size={14} />}
    </button>
  );
};

export function Sidebar({ families, rt, selected, onSelect, onHide, onAdd, onRemove, onRefresh }: Props) {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [showHidden, setShowHidden] = useState(false);
  const hiddenCount = families.reduce(
    (n, f) => n + f.projects.reduce((m, p) => m + (p.commands.length > 1 ? p.hidden_commands.length : p.hidden ? 1 : 0), 0),
    0,
  );
  const remove = async (f: Family) => {
    if (await ask(`Remover a família "${f.name}"? A pasta não é apagada.`, { title: "Remover família" })) onRemove(f.path);
  };

  // Running things stay visible even when hidden, so nothing runs out of sight.
  const visibleSubs = (p: Project) =>
    p.commands.filter((c) => showHidden || !p.hidden_commands.includes(c.name) || rt[p.path]?.[c.name]?.running);
  const isVisible = (p: Project) =>
    p.commands.length > 1
      ? visibleSubs(p).length > 0
      : showHidden || !p.hidden || projectRunning(rt, p.path);

  const projectRow = (p: Project) => {
    const url = projectUrl(rt, p);
    const multi = p.commands.length > 1;
    const isSel = selected?.path === p.path;
    return (
      <div key={p.path}>
        <div className={`project-row ${!multi && p.hidden ? "is-hidden" : ""}`}>
          <button
            className={`project ${isSel && (!multi || selected?.tab === null) ? "active" : ""}`}
            onClick={() => onSelect(p.path)}
          >
            <span className={`dot ${projectStatus(rt, p.path)}`} />
            <span className="name">{p.name}</span>
            {url && <span className="port">:{portOf(url)}</span>}
          </button>
          {!multi && <HideButton hidden={p.hidden} onClick={() => onHide(p.path, !p.hidden)} />}
        </div>
        {multi &&
          visibleSubs(p).map((c) => {
            const cUrl = commandUrl(rt, p.path, c.name);
            const hidden = p.hidden_commands.includes(c.name);
            return (
              <div key={c.name} className={`project-row ${hidden ? "is-hidden" : ""}`}>
                <button
                  className={`project sub ${isSel && selected?.tab === c.name ? "active" : ""}`}
                  onClick={() => onSelect(p.path, c.name)}
                >
                  <span className={`dot ${cmdStatus(rt[p.path]?.[c.name])}`} />
                  <span className="name">{c.name}</span>
                  {cUrl && <span className="port">:{portOf(cUrl)}</span>}
                </button>
                <HideButton hidden={hidden} onClick={() => onHide(subKey(p.path, c.name), !hidden)} />
              </div>
            );
          })}
      </div>
    );
  };

  const toggleLabel = showHidden ? "Esconder projetos ocultos" : `Mostrar projetos ocultos (${hiddenCount})`;

  return (
    <aside className="sidebar">
      <header>
        <span>Projetos</span>
        <span className="header-actions">
          <button
            className={`toggle ${showHidden ? "on" : ""}`}
            title={toggleLabel}
            aria-label={toggleLabel}
            aria-pressed={showHidden}
            onClick={() => setShowHidden(!showHidden)}
          >
            {showHidden ? <EyeIcon /> : <EyeOffIcon />}
            {hiddenCount > 0 && <span className="count">{hiddenCount}</span>}
          </button>
          <button title="Recarregar pastas" onClick={onRefresh}>⟳</button>
          <button title="Adicionar família" onClick={onAdd}>+</button>
        </span>
      </header>
      {families.length === 0 && <p className="hint">Adicione uma família (ex.: a pasta Node) com o botão +.</p>}
      {families.map((f) => {
        const list = f.projects.filter(isVisible);
        return (
          <section key={f.path} className="family-group">
            <div className="family">
              <button className="family-name" onClick={() => setCollapsed({ ...collapsed, [f.path]: !collapsed[f.path] })}>
                <span className="chevron">{collapsed[f.path] ? "▸" : "▾"}</span>
                <FolderIcon />
                <span className="name">{f.name}</span>
                <span className="count">{list.length}</span>
              </button>
              <button title="Remover família" onClick={() => remove(f)}>×</button>
            </div>
            {!collapsed[f.path] && <div className="family-projects">{list.map(projectRow)}</div>}
          </section>
        );
      })}
    </aside>
  );
}
