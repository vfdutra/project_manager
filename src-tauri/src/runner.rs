use crate::detect::CommandSpec;
use crate::ports::{extract_url, strip_ansi};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RunnerEvent {
    Started { project: String, command: String },
    Log { project: String, command: String, line: String },
    Url { project: String, command: String, url: String },
    Exited { project: String, command: String, code: Option<i32> },
}

pub type Emit = Arc<dyn Fn(RunnerEvent) + Send + Sync>;
type Key = (String, String);

#[derive(Clone)]
struct Proc {
    pid: u32,
    cwd: PathBuf,
    stop: Option<String>,
    url: Option<String>,
    stopping: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct RunningInfo {
    pub project: String,
    pub command: String,
    pub url: Option<String>,
}

pub struct Runner {
    procs: Arc<Mutex<HashMap<Key, Proc>>>,
    emit: Emit,
}

fn shell(cmd: &str, cwd: &Path) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C").raw_arg(cmd).current_dir(cwd).creation_flags(CREATE_NO_WINDOW);
    c
}

impl Runner {
    pub fn new(emit: Emit) -> Self {
        Self { procs: Default::default(), emit }
    }

    pub fn start(&self, project: &str, spec: &CommandSpec) -> Result<(), String> {
        let key: Key = (project.to_string(), spec.name.clone());
        if self.procs.lock().unwrap().contains_key(&key) {
            return Ok(());
        }
        let cwd = Path::new(project).join(&spec.cwd);
        let mut child = shell(&spec.cmd, &cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{}: {e}", spec.cmd))?;
        self.procs
            .lock()
            .unwrap()
            .insert(key.clone(), Proc { pid: child.id(), cwd, stop: spec.stop.clone(), url: None, stopping: false });
        (self.emit)(RunnerEvent::Started { project: key.0.clone(), command: key.1.clone() });

        let out = self.spawn_reader(key.clone(), child.stdout.take().unwrap());
        let err = self.spawn_reader(key.clone(), child.stderr.take().unwrap());
        let (procs, emit) = (self.procs.clone(), self.emit.clone());
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            // ponytail: waits for pipes to close; a detached grandchild holding them delays `exited`.
            let _ = out.join();
            let _ = err.join();
            // A user-requested stop is not a failure: report it without an exit code.
            let stopped = procs.lock().unwrap().remove(&key).is_some_and(|p| p.stopping);
            emit(RunnerEvent::Exited { project: key.0, command: key.1, code: if stopped { None } else { code } });
        });
        Ok(())
    }

    fn spawn_reader(&self, key: Key, src: impl Read + Send + 'static) -> JoinHandle<()> {
        let (procs, emit) = (self.procs.clone(), self.emit.clone());
        std::thread::spawn(move || {
            let mut reader = BufReader::new(src);
            let mut buf = Vec::new();
            // read_until + lossy: Windows tools often print non-UTF-8 (cp850/cp1252).
            while reader.read_until(b'\n', &mut buf).map(|n| n > 0).unwrap_or(false) {
                let line = strip_ansi(String::from_utf8_lossy(&buf).trim_end());
                buf.clear();
                if let Some(url) = extract_url(&line) {
                    if let Some(p) = procs.lock().unwrap().get_mut(&key) {
                        if p.url.is_none() {
                            p.url = Some(url.clone());
                            emit(RunnerEvent::Url { project: key.0.clone(), command: key.1.clone(), url });
                        }
                    }
                }
                emit(RunnerEvent::Log { project: key.0.clone(), command: key.1.clone(), line });
            }
        })
    }

    pub fn stop(&self, project: &str, command: &str) {
        let key: Key = (project.into(), command.into());
        let Some(p) = self.procs.lock().unwrap().get_mut(&key).map(|p| {
            p.stopping = true;
            p.clone()
        }) else {
            return;
        };
        if let Some(stop) = &p.stop {
            let _ = shell(stop, &p.cwd).stdout(Stdio::null()).stderr(Stdio::null()).status();
        }
        let _ = Command::new("taskkill")
            .args(["/PID", &p.pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    pub fn stop_project(&self, project: &str) {
        let names: Vec<String> =
            self.procs.lock().unwrap().keys().filter(|k| k.0 == project).map(|k| k.1.clone()).collect();
        for n in names {
            self.stop(project, &n);
        }
    }

    pub fn stop_all(&self) {
        let keys: Vec<Key> = self.procs.lock().unwrap().keys().cloned().collect();
        for (p, c) in keys {
            self.stop(&p, &c);
        }
    }

    pub fn running(&self) -> Vec<RunningInfo> {
        self.procs
            .lock()
            .unwrap()
            .iter()
            .map(|((p, c), proc)| RunningInfo { project: p.clone(), command: c.clone(), url: proc.url.clone() })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    type Events = Arc<Mutex<Vec<RunnerEvent>>>;

    fn runner() -> (Runner, Events) {
        let events: Events = Default::default();
        let sink = events.clone();
        (Runner::new(Arc::new(move |e| sink.lock().unwrap().push(e))), events)
    }

    fn wait_for(events: &Events, pred: impl Fn(&RunnerEvent) -> bool) -> bool {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(10) {
            if events.lock().unwrap().iter().any(&pred) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn spec(cmd: &str, stop: Option<&str>) -> CommandSpec {
        CommandSpec { name: "t".into(), cwd: "".into(), cmd: cmd.into(), stop: stop.map(Into::into) }
    }

    #[test]
    fn captures_logs_url_and_exit() {
        let (r, ev) = runner();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().to_string_lossy().into_owned();
        r.start(&project, &spec("echo Local: http://localhost:5173/", None)).unwrap();
        assert!(wait_for(&ev, |e| matches!(e, RunnerEvent::Exited { code: Some(0), .. })));
        let ev = ev.lock().unwrap();
        assert!(matches!(&ev[0], RunnerEvent::Started { .. }));
        assert!(ev.iter().any(|e| matches!(e, RunnerEvent::Url { url, .. } if url == "http://localhost:5173")));
        assert!(ev.iter().any(|e| matches!(e, RunnerEvent::Log { line, .. } if line.contains("Local:"))));
        assert!(r.running().is_empty());
    }

    #[test]
    fn stop_runs_stop_command_and_kills_tree() {
        let (r, ev) = runner();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().to_string_lossy().into_owned();
        r.start(&project, &spec("ping -n 30 127.0.0.1", Some("echo x> stopped.txt"))).unwrap();
        assert_eq!(r.running().len(), 1);
        r.start(&project, &spec("ping -n 30 127.0.0.1", None)).unwrap(); // already running → no-op
        assert_eq!(r.running().len(), 1);
        r.stop(&project, "t");
        assert!(wait_for(&ev, |e| matches!(e, RunnerEvent::Exited { code: None, .. })));
        assert!(dir.path().join("stopped.txt").exists());
        assert!(r.running().is_empty());
    }
}
