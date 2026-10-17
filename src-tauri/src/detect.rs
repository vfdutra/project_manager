use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandSpec {
    pub name: String,
    /// Relative to the project dir; "" = project root.
    pub cwd: String,
    pub cmd: String,
    /// Run before killing the process tree (e.g. `docker compose stop`).
    #[serde(default)]
    pub stop: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Detected {
    pub kind: String,
    pub commands: Vec<CommandSpec>,
}

pub fn detect(dir: &Path) -> Detected {
    if let Some((kind, cmd, stop)) = detect_one(dir) {
        return Detected { kind: kind.into(), commands: vec![spec(kind.into(), "", cmd, stop)] };
    }
    // Monorepo-ish layout (e.g. minha-loja/frontend + backend): look one level down.
    let mut kind = "unknown".to_string();
    let mut commands = vec![];
    for sub in subdirs(dir) {
        if let Some((k, cmd, stop)) = detect_one(&sub) {
            if commands.is_empty() {
                kind = k.into();
            }
            let name = dir_name(&sub);
            commands.push(spec(name.clone(), &name, cmd, stop));
        }
    }
    Detected { kind, commands }
}

fn detect_one(dir: &Path) -> Option<(&'static str, String, Option<String>)> {
    let has = |f: &str| dir.join(f).exists();
    for f in ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"] {
        if has(f) {
            return Some(("docker", "docker compose up".into(), Some("docker compose stop".into())));
        }
    }
    if let Some(script) = npm_script(&dir.join("package.json")) {
        let pm = if has("pnpm-lock.yaml") {
            "pnpm"
        } else if has("yarn.lock") {
            "yarn"
        } else if has("bun.lockb") || has("bun.lock") {
            "bun"
        } else {
            "npm"
        };
        return Some(("node", format!("{pm} run {script}"), None));
    }
    if has("artisan") {
        return Some(("php", "php artisan serve".into(), None));
    }
    if has("composer.json") || has("index.php") {
        let root = if has("public") { " -t public" } else { "" };
        return Some(("php", format!("php -S localhost:8000{root}"), None));
    }
    let py = python(dir);
    if has("manage.py") {
        return Some(("python", format!("{py} manage.py runserver"), None));
    }
    for f in ["main.py", "app.py"] {
        if has(f) {
            return Some(("python", format!("{py} {f}"), None));
        }
    }
    None
}

fn npm_script(pkg: &Path) -> Option<&'static str> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(pkg).ok()?).ok()?;
    ["dev", "start"].into_iter().find(|s| v["scripts"][*s].is_string())
}

fn python(dir: &Path) -> String {
    for venv in [".venv", "venv"] {
        if dir.join(venv).join("Scripts").join("python.exe").exists() {
            return format!(r"{venv}\Scripts\python");
        }
    }
    "python".into()
}

fn spec(name: String, cwd: &str, cmd: String, stop: Option<String>) -> CommandSpec {
    CommandSpec { name, cwd: cwd.into(), cmd, stop }
}

pub fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !dir_name(p).starts_with('.') && dir_name(p) != "node_modules")
        .collect();
    v.sort();
    v
}

pub fn dir_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    fn cmds(d: &Detected) -> Vec<(&str, &str)> {
        d.commands.iter().map(|c| (c.cwd.as_str(), c.cmd.as_str())).collect()
    }

    #[test]
    fn node_dev_script_with_npm() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "package.json", r#"{"scripts":{"dev":"vite","start":"node ."}}"#);
        let d = detect(t.path());
        assert_eq!(d.kind, "node");
        assert_eq!(cmds(&d), [("", "npm run dev")]);
        assert_eq!(d.commands[0].name, "node");
    }

    #[test]
    fn node_start_fallback_and_pnpm_lockfile() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "package.json", r#"{"scripts":{"start":"node ."}}"#);
        write(t.path(), "pnpm-lock.yaml", "");
        assert_eq!(cmds(&detect(t.path())), [("", "pnpm run start")]);
    }

    #[test]
    fn compose_wins_and_has_stop_command() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "docker-compose.yml", "services: {}");
        write(t.path(), "package.json", r#"{"scripts":{"dev":"vite"}}"#);
        let d = detect(t.path());
        assert_eq!(d.kind, "docker");
        assert_eq!(cmds(&d), [("", "docker compose up")]);
        assert_eq!(d.commands[0].stop.as_deref(), Some("docker compose stop"));
    }

    #[test]
    fn django_uses_local_venv() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "manage.py", "");
        write(t.path(), ".venv/Scripts/python.exe", "");
        let d = detect(t.path());
        assert_eq!(d.kind, "python");
        assert_eq!(cmds(&d), [("", r".venv\Scripts\python manage.py runserver")]);
    }

    #[test]
    fn laravel_and_plain_php() {
        let a = tempfile::tempdir().unwrap();
        write(a.path(), "artisan", "");
        write(a.path(), "composer.json", "{}");
        assert_eq!(cmds(&detect(a.path())), [("", "php artisan serve")]);

        let b = tempfile::tempdir().unwrap();
        write(b.path(), "composer.json", "{}");
        write(b.path(), "public/index.php", "");
        assert_eq!(cmds(&detect(b.path())), [("", "php -S localhost:8000 -t public")]);
    }

    #[test]
    fn monorepo_scans_one_level_down() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "front/package.json", r#"{"scripts":{"dev":"vite"}}"#);
        write(t.path(), "api/main.py", "");
        write(t.path(), ".git/config", "");
        write(t.path(), "node_modules/package.json", r#"{"scripts":{"dev":"x"}}"#);
        let d = detect(t.path());
        assert_eq!(d.kind, "python");
        assert_eq!(cmds(&d), [("api", "python main.py"), ("front", "npm run dev")]);
        let names: Vec<_> = d.commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["api", "front"]);
    }

    #[test]
    fn empty_dir_is_unknown() {
        let t = tempfile::tempdir().unwrap();
        let d = detect(t.path());
        assert_eq!(d.kind, "unknown");
        assert!(d.commands.is_empty());
    }
}
