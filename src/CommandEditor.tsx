import { useState } from "react";
import type { CommandSpec, Project, ProjectOverride } from "./api";

type Props = { project: Project; onSave: (v: ProjectOverride) => void };
const blank: CommandSpec = { name: "", cwd: "", cmd: "", stop: null };

export function CommandEditor({ project, onSave }: Props) {
  const [rows, setRows] = useState<CommandSpec[]>(
    project.commands.length ? project.commands : [{ ...blank, name: "dev" }],
  );
  const [url, setUrl] = useState(project.url ?? "");
  const set = (i: number, patch: Partial<CommandSpec>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const names = rows.map((r) => r.name.trim());
  const invalid = rows.some((r) => !r.name.trim() || !r.cmd.trim()) || new Set(names).size !== names.length;

  return (
    <div className="editor">
      <p className="hint">
        Cada linha roda em um processo separado (como um terminal por comando). "Pasta" é relativa ao projeto. Nomes
        precisam ser únicos.
      </p>
      {rows.map((r, i) => (
        <div className="row" key={i}>
          <input placeholder="nome" value={r.name} onChange={(e) => set(i, { name: e.target.value })} />
          <input placeholder="pasta (opcional)" value={r.cwd} onChange={(e) => set(i, { cwd: e.target.value })} />
          <input
            className="grow"
            placeholder="comando, ex.: npm run dev"
            value={r.cmd}
            onChange={(e) => set(i, { cmd: e.target.value })}
          />
          <input
            placeholder="ao parar (opcional)"
            value={r.stop ?? ""}
            onChange={(e) => set(i, { stop: e.target.value || null })}
          />
          <button title="Remover comando" onClick={() => setRows(rows.filter((_, j) => j !== i))}>×</button>
        </div>
      ))}
      <div>
        <button onClick={() => setRows([...rows, { ...blank }])}>+ Comando</button>
      </div>
      <label>
        URL fixa (opcional, quando a porta não aparece no log):
        <input value={url} placeholder="http://localhost:8080" onChange={(e) => setUrl(e.target.value)} />
      </label>
      <div className="actions">
        <button
          className="primary"
          disabled={invalid}
          onClick={() => onSave({ commands: rows.map((r) => ({ ...r, name: r.name.trim() })), url: url.trim() || null })}
        >
          Salvar
        </button>
        {(project.customized || project.url) && (
          <button onClick={() => onSave({ commands: null, url: null })}>Restaurar detecção automática</button>
        )}
      </div>
    </div>
  );
}
