import { useEffect, useState } from "react";
import { api } from "./api";
import { reduce, type Runtime } from "./runtime";

export function useRuntime(): Runtime {
  const [rt, setRt] = useState<Runtime>({});
  useEffect(() => {
    const unlisten = api.onEvent((e) => setRt((r) => reduce(r, e)));
    // Restore processes that survived a webview reload.
    api.running().then((list) =>
      setRt((r) => {
        for (const i of list) {
          r = reduce(r, { type: "started", project: i.project, command: i.command });
          if (i.url) r = reduce(r, { type: "url", project: i.project, command: i.command, url: i.url });
        }
        return r;
      }),
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  return rt;
}
