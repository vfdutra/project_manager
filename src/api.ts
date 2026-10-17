import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type CommandSpec = { name: string; cwd: string; cmd: string; stop?: string | null };
export type Project = {
  name: string;
  path: string;
  kind: string;
  commands: CommandSpec[];
  url: string | null;
  customized: boolean;
  hidden: boolean;
  hidden_commands: string[];
};
export type Family = { name: string; path: string; projects: Project[] };
export type ProjectOverride = { commands: CommandSpec[] | null; url: string | null };
export type RunnerEvent =
  | { type: "started"; project: string; command: string }
  | { type: "log"; project: string; command: string; line: string }
  | { type: "url"; project: string; command: string; url: string }
  | { type: "exited"; project: string; command: string; code: number | null };
export type RunningInfo = { project: string; command: string; url: string | null };

/** Key used in the hidden list for one sub-project (command) of a project. */
export const subKey = (projectPath: string, command: string) => `${projectPath}::${command}`;

export const api = {
  families: () => invoke<Family[]>("get_families"),
  addFamily: (path: string) => invoke<void>("add_family", { path }),
  removeFamily: (path: string) => invoke<void>("remove_family", { path }),
  saveOverride: (project: string, value: ProjectOverride) => invoke<void>("save_override", { project, value }),
  setHidden: (project: string, hidden: boolean) => invoke<void>("set_hidden", { project, hidden }),
  start: (project: string) => invoke<void>("start_project", { project }),
  stop: (project: string) => invoke<void>("stop_project", { project }),
  startCommand: (project: string, command: string) => invoke<void>("start_command", { project, command }),
  stopCommand: (project: string, command: string) => invoke<void>("stop_command", { project, command }),
  running: () => invoke<RunningInfo[]>("running"),
  onEvent: (cb: (e: RunnerEvent) => void) => listen<RunnerEvent>("proc", (e) => cb(e.payload)),
};
