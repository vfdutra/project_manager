import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, subKey, type Project } from "./api";
import { type Runtime, cmdStatus, commandUrl, projectRunning, projectUrl, portOf } from "./runtime";
import { CommandEditor } from "./CommandEditor";
import { ExternalIcon, EyeIcon, EyeOffIcon, PlayIcon, StopIcon } from "./icons";

type Props = {
  project: Project;
  rt: Runtime;
  run: (p: Promise<unknown>) => void;
  tab: string | null;
  onTab: (name: string) => void;
};

export function ProjectPanel({ project: p, rt, run, tab, onTab }: Props) {
  const [editing, setEditing] = useState(false);
  const states = rt[p.path] ?? {};
  const multi = p.commands.length > 1;
  const anyRunning = projectRunning(rt, p.path);
  const allRunning = p.commands.length > 0 && p.commands.every((c) => states[c.name]?.running);
  const url = projectUrl(rt, p);
  const current = p.commands.find((c) => c.name === tab) ?? p.commands[0];
  const cur = current ? states[current.name] : undefined;
  const curUrl = current ? commandUrl(rt, p.path, current.name) : null;
  const startLabel = multi ? "Iniciar todos" : "Iniciar";
  const stopLabel = multi ? "Parar todos" : "Parar";
  const hideLabel = p.hidden ? "Mostrar na lista" : "Ocultar da lista";
  const curHidden = current ? p.hidden_commands.includes(current.name) : false;
  const curHideLabel = curHidden ? "Mostrar na lista" : "Ocultar da lista";

  return (
    <div className="panel">
      <header>
        <div>
          <h1>
            {p.name} <span className="badge">{p.kind}</span>
          </h1>
          <small>{p.path}</small>
        </div>
        <div className="actions">
          {url && <span className="port">Porta {portOf(url)}</span>}
          <button className="icon-text" disabled={!url} onClick={() => url && run(openUrl(url))}>
            <ExternalIcon /> Abrir no navegador
          </button>
          {anyRunning && (
            <button className="icon stop" title={stopLabel} aria-label={stopLabel} onClick={() => run(api.stop(p.path))}>
              <StopIcon />
            </button>
          )}
          {!allRunning && (
            <button
              className="icon play"
              title={startLabel}
              aria-label={startLabel}
              disabled={p.commands.length === 0}
              onClick={() => run(api.start(p.path))}
            >
              <PlayIcon />
            </button>
          )}
          {!multi && (
            <button className="icon" title={hideLabel} aria-label={hideLabel} onClick={() => run(api.setHidden(p.path, !p.hidden))}>
              {p.hidden ? <EyeIcon /> : <EyeOffIcon />}
            </button>
          )}
          <button onClick={() => setEditing(!editing)}>{editing ? "Fechar" : "Configurar"}</button>
        </div>
      </header>

      {editing && (
        <CommandEditor
          project={p}
          onSave={(v) => {
            setEditing(false);
            run(api.saveOverride(p.path, v));
          }}
        />
      )}

      {p.commands.length === 0 && !editing && (
        <p className="hint">Nenhum comando detectado. Clique em Configurar para definir os comandos de inicialização.</p>
      )}

      {current && (
        <>
          <nav className="tabs">
            {p.commands.map((c) => (
              <button key={c.name} className={`${c.name === current.name ? "active" : ""} ${p.hidden_commands.includes(c.name) ? "is-hidden" : ""}`} onClick={() => onTab(c.name)}>
                <span className={`dot ${cmdStatus(states[c.name])}`} /> {c.name}
              </button>
            ))}
          </nav>
          <div className="cmdbar">
            <code className="cmdline">
              {current.cwd ? `${current.cwd}> ` : "> "}
              {current.cmd}
            </code>
            {multi && (
              <span className="actions">
                <button
                  className="icon"
                  title={curHideLabel}
                  aria-label={curHideLabel}
                  onClick={() => run(api.setHidden(subKey(p.path, current.name), !curHidden))}
                >
                  {curHidden ? <EyeIcon size={14} /> : <EyeOffIcon size={14} />}
                </button>
                {curUrl && (
                  <button className="icon-text" onClick={() => run(openUrl(curUrl))}>
                    <ExternalIcon size={14} /> :{portOf(curUrl)}
                  </button>
                )}
                {cur?.running ? (
                  <button
                    className="icon stop"
                    title={`Parar ${current.name}`}
                    aria-label={`Parar ${current.name}`}
                    onClick={() => run(api.stopCommand(p.path, current.name))}
                  >
                    <StopIcon size={14} />
                  </button>
                ) : (
                  <button
                    className="icon play"
                    title={`Iniciar ${current.name}`}
                    aria-label={`Iniciar ${current.name}`}
                    onClick={() => run(api.startCommand(p.path, current.name))}
                  >
                    <PlayIcon size={14} />
                  </button>
                )}
              </span>
            )}
          </div>
          <Logs lines={cur?.logs ?? []} />
        </>
      )}
    </div>
  );
}

function Logs({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLPreElement>(null);
  const stick = useRef(true);
  useEffect(() => {
    if (stick.current && ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines]);
  return (
    <pre
      ref={ref}
      className="logs"
      onScroll={(e) => {
        const el = e.currentTarget;
        stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
      }}
    >
      {lines.join("\n")}
    </pre>
  );
}
