import type { Project, RunnerEvent } from "./api";

export type CmdState = { running: boolean; url: string | null; logs: string[]; code: number | null };
/** project path → command name → state */
export type Runtime = Record<string, Record<string, CmdState>>;

export const MAX_LOG = 2000;
const empty = (): CmdState => ({ running: false, url: null, logs: [], code: null });

export function reduce(rt: Runtime, e: RunnerEvent): Runtime {
  const proj = rt[e.project] ?? {};
  const prev = proj[e.command] ?? empty();
  let next: CmdState;
  switch (e.type) {
    case "started":
      next = { ...empty(), running: true };
      break;
    case "log":
      // ponytail: copies up to MAX_LOG lines per event; switch to a ring buffer if huge logs lag.
      next = { ...prev, logs: [...prev.logs.slice(-(MAX_LOG - 1)), e.line] };
      break;
    case "url":
      next = { ...prev, url: e.url };
      break;
    case "exited":
      next = { ...prev, running: false, code: e.code };
      break;
  }
  return { ...rt, [e.project]: { ...proj, [e.command]: next } };
}

export const projectRunning = (rt: Runtime, path: string) =>
  Object.values(rt[path] ?? {}).some((c) => c.running);

/** URL to open: the user's fixed URL, else the first one a running command printed. */
export const projectUrl = (rt: Runtime, p: Project): string | null => {
  if (!projectRunning(rt, p.path)) return null;
  return p.url ?? Object.values(rt[p.path] ?? {}).find((c) => c.running && c.url)?.url ?? null;
};

export const portOf = (url: string) => {
  try {
    const u = new URL(url);
    return u.port || (u.protocol === "https:" ? "443" : "80");
  } catch {
    return "?";
  }
};

export type Status = "on" | "err" | "";

/** Dot for one command: running, crashed (non-zero exit not caused by Stop), or idle. */
export const cmdStatus = (s?: CmdState): Status => (s?.running ? "on" : s?.code ? "err" : "");

/** Dot for a whole project: any command running → on, else any crashed → err. */
export const projectStatus = (rt: Runtime, path: string): Status => {
  const all = Object.values(rt[path] ?? {}).map(cmdStatus);
  return all.includes("on") ? "on" : all.includes("err") ? "err" : "";
};

/** URL printed by one running command. */
export const commandUrl = (rt: Runtime, path: string, command: string): string | null => {
  const s = rt[path]?.[command];
  return s?.running ? s.url : null;
};
