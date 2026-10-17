# Gerenciador de Projetos — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** App desktop (estilo Docker Desktop) que lista projetos agrupados por "família" (pasta), detecta como rodá-los (Node/Docker/Python/PHP), inicia/para cada um pela UI, mostra logs, porta e abre no navegador.

**Architecture:** Tauri 2 (backend Rust + webview React/TS). O Rust escaneia as pastas das famílias, detecta comandos, guarda config em JSON e gerencia processos (`cmd /C <comando>`, um processo por comando, logs via evento `proc`). O React só renderiza: sidebar família → projetos (igual à lista pastas+sessões do Claude Code) e um painel com Iniciar/Parar, porta, "Abrir no navegador", abas de log por comando e editor de comandos.

**Tech Stack:** Rust 1.91 (MSVC), Tauri 2, `tauri-plugin-dialog`, `tauri-plugin-opener`, `regex`, `serde_json`; React 18 + TypeScript + Vite (template `create-tauri-app` react-ts); testes Rust com `cargo test` + `tempfile`; teste do reducer TS com `node --test` (Node 24, type stripping nativo).

## Global Constraints

- Plataforma alvo: **Windows apenas** (usa `std::os::windows::process::CommandExt`, `cmd /C`, `taskkill`).
- Raiz do repo: `C:\Users\dutra\Documents\Projetos\Gerenciador de Projetos`. Remoto: `https://github.com/vfdutra/project_manager.git`, branch `main`.
- Textos da UI em **português**. Identificadores de código em inglês.
- Família = uma pasta escolhida pelo usuário; projetos = subpastas imediatas (ignorando as que começam com `.` e `node_modules`).
- Cada comando de um projeto roda em **processo separado** (equivalente a um terminal por comando).
- Comandos detectados podem ser sobrescritos pelo usuário (override por caminho do projeto, salvo em `%APPDATA%\com.vfdutra.projectmanager\config.json`).
- Fechar o app **mata todos os processos filhos** (sem servidores órfãos).
- Nenhuma dependência nova além das listadas no Tech Stack.
- **Commit Gate (CLAUDE.md do usuário):** os passos "Checkpoint de commit" NÃO autorizam commit. Ao chegar neles, mostre `git status`/`git diff --stat`, proponha a mensagem e **pergunte** ao humano. Só rode `git commit` com autorização explícita ("commita", "pode commitar"). `git push` exige autorização separada.

---

## Mapa de arquivos

| Arquivo | Responsabilidade |
|---|---|
| `src-tauri/src/detect.rs` | `CommandSpec`, `detect(dir)` → tipo + comandos sugeridos; `subdirs`, `dir_name` |
| `src-tauri/src/ports.rs` | `strip_ansi`, `extract_url(line)` (porta a partir do log) |
| `src-tauri/src/config.rs` | `Config` (famílias + overrides), load/save JSON, monta `Family`/`Project` |
| `src-tauri/src/runner.rs` | Spawna/para processos, lê stdout/stderr, emite `RunnerEvent` |
| `src-tauri/src/lib.rs` | Comandos Tauri + estado + limpeza ao sair |
| `src/api.ts` | Tipos espelhando o Rust + wrappers `invoke`/`listen` |
| `src/runtime.ts` | Reducer puro do estado de execução (testável no Node) |
| `src/useRuntime.ts` | Hook React que liga eventos → reducer |
| `src/App.tsx` / `src/App.css` | Layout e tema escuro |
| `src/Sidebar.tsx` | Famílias recolhíveis + projetos com status/porta |
| `src/ProjectPanel.tsx` | Ações, porta, abas de log |
| `src/CommandEditor.tsx` | Editar comandos/URL do projeto |
| `tests/runtime.test.ts` | Teste do reducer (`node --test`) |

---

### Task 1: Scaffold Tauri + git

**Files:**
- Create: todo o template `create-tauri-app` (react-ts) na raiz do repo
- Modify: `src-tauri/Cargo.toml` (deps), `src-tauri/tauri.conf.json` (janela), `package.json` (script test)

**Interfaces:**
- Produces: crate lib `project_manager_lib` em `src-tauri/`, `src-tauri/src/lib.rs` com `pub fn run()`, plugins opener + dialog registrados, script `npm test`.

- [ ] **Step 1: git init + remoto**

```bash
cd "/c/Users/dutra/Documents/Projetos/Gerenciador de Projetos"
git init -b main
git remote add origin https://github.com/vfdutra/project_manager.git
```

- [ ] **Step 2: Gerar o template em subpasta e mover para a raiz**

(O nome da pasta raiz tem espaços/acentos, por isso geramos em `project-manager/` e subimos.)

```bash
cd "/c/Users/dutra/Documents/Projetos/Gerenciador de Projetos"
npx --yes create-tauri-app@4.8.0 project-manager --template react-ts --manager npm --identifier com.vfdutra.projectmanager --tauri-version 2 -y
shopt -s dotglob && mv project-manager/* . && rmdir project-manager
npm install
```

Expected: raiz contém `package.json`, `src/`, `src-tauri/`, `index.html`, `.gitignore`.

- [ ] **Step 3: Adicionar plugin dialog e crates**

```bash
npm run tauri add dialog
cargo add regex --manifest-path src-tauri/Cargo.toml
cargo add tempfile --dev --manifest-path src-tauri/Cargo.toml
```

Expected: `src-tauri/capabilities/default.json` contém `"dialog:default"` e `"opener:default"`. Se `dialog:default` não aparecer, adicione-o à lista `permissions`.

- [ ] **Step 4: Configurar janela**

Em `src-tauri/tauri.conf.json`, defina `"productName": "Gerenciador de Projetos"` e substitua o objeto em `app.windows[0]` por:

```json
{ "title": "Gerenciador de Projetos", "width": 1200, "height": 760, "minWidth": 800, "minHeight": 500 }
```

- [ ] **Step 5: Script de teste do frontend**

Em `package.json`, dentro de `"scripts"`, adicione:

```json
"test": "node --test tests/"
```

- [ ] **Step 6: Verificar que tudo compila**

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: `vite build` termina sem erro; `cargo test` compila (primeira vez demora alguns minutos) e mostra `test result: ok. 0 passed`.

- [ ] **Step 7: Checkpoint de commit** (ver Commit Gate)

Mensagem proposta: `chore: scaffold tauri 2 + react-ts app`

---

### Task 2: Detecção de projetos (`detect.rs`)

**Files:**
- Create: `src-tauri/src/detect.rs`
- Modify: `src-tauri/src/lib.rs` (adicionar `mod detect;` na primeira linha)

**Interfaces:**
- Produces:
  - `pub struct CommandSpec { pub name: String, pub cwd: String, pub cmd: String, pub stop: Option<String> }` (Serialize/Deserialize/Clone/Debug/PartialEq; `cwd` relativo ao projeto, `""` = raiz)
  - `pub struct Detected { pub kind: String, pub commands: Vec<CommandSpec> }`
  - `pub fn detect(dir: &Path) -> Detected` — `kind` ∈ `"docker" | "node" | "php" | "python" | "unknown"`
  - `pub fn subdirs(dir: &Path) -> Vec<PathBuf>` (ordenado, sem ocultas/`node_modules`)
  - `pub fn dir_name(p: &Path) -> String`

Regras (primeira que casar, por pasta): compose (`docker compose up`, stop `docker compose stop`) → `package.json` com script `dev`/`start` (gerenciador pelo lockfile) → `artisan` → `composer.json`/`index.php` (`php -S localhost:8000 [-t public]`) → `manage.py` → `main.py`/`app.py`. Python usa `.venv\Scripts\python` ou `venv\Scripts\python` se existir. Se a raiz não casar nada, aplica as regras em cada subpasta (ex.: `minha-loja/frontend`, `minha-loja/backend`), um comando por subpasta com `name` = `cwd` = nome da subpasta.

- [ ] **Step 1: Escrever os testes**

Crie `src-tauri/src/detect.rs` só com o módulo de teste e os tipos (para compilar e falhar):

```rust
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

pub fn detect(_dir: &Path) -> Detected {
    todo!()
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
```

Adicione `mod detect;` como primeira linha de `src-tauri/src/lib.rs`.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detect`
Expected: 7 testes FAIL com `not yet implemented`.

- [ ] **Step 3: Implementar**

Substitua `pub fn detect(_dir: &Path) -> Detected { todo!() }` por:

```rust
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
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detect`
Expected: `test result: ok. 7 passed`.

- [ ] **Step 5: Checkpoint de commit** — `feat: detect run commands for node, docker, php and python projects`

---

### Task 3: Porta a partir dos logs (`ports.rs`)

**Files:**
- Create: `src-tauri/src/ports.rs`
- Modify: `src-tauri/src/lib.rs` (adicionar `mod ports;`)

**Interfaces:**
- Produces: `pub fn strip_ansi(s: &str) -> String`, `pub fn extract_url(line: &str) -> Option<String>` → sempre `http(s)://localhost:PORT` (sem barra final).

- [ ] **Step 1: Escrever os testes**

Crie `src-tauri/src/ports.rs`:

```rust
use regex::Regex;
use std::sync::LazyLock;

pub fn strip_ansi(_s: &str) -> String {
    todo!()
}

pub fn extract_url(_line: &str) -> Option<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(line: &str) -> Option<String> {
        extract_url(&strip_ansi(line))
    }

    #[test]
    fn known_dev_servers() {
        assert_eq!(url("  ➜  Local:   http://localhost:5173/").as_deref(), Some("http://localhost:5173"));
        assert_eq!(url("Starting development server at http://127.0.0.1:8000/").as_deref(), Some("http://localhost:8000"));
        assert_eq!(url("Uvicorn running on http://0.0.0.0:8080 (Press CTRL+C to quit)").as_deref(), Some("http://localhost:8080"));
        assert_eq!(url("PHP 8.3 Development Server (http://localhost:8000) started").as_deref(), Some("http://localhost:8000"));
        assert_eq!(url("Server listening on port 3000").as_deref(), Some("http://localhost:3000"));
        assert_eq!(url("App running on port: 4000").as_deref(), Some("http://localhost:4000"));
        assert_eq!(url("Local: https://localhost:8443/").as_deref(), Some("https://localhost:8443"));
    }

    #[test]
    fn colored_vite_output() {
        assert_eq!(url("http://localhost:\x1b[1m5173\x1b[22m/").as_deref(), Some("http://localhost:5173"));
    }

    #[test]
    fn unrelated_lines() {
        assert_eq!(url("compiled successfully in 1.2s"), None);
        assert_eq!(url("Network: http://192.168.0.10:5173/"), None);
    }

    #[test]
    fn strip_ansi_removes_escape_codes() {
        assert_eq!(strip_ansi("\x1b[32mok\x1b[0m"), "ok");
    }
}
```

Adicione `mod ports;` em `src-tauri/src/lib.rs`.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ports`
Expected: 4 FAIL com `not yet implemented`.

- [ ] **Step 3: Implementar** (substitui os dois `todo!()`)

```rust
static ANSI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[A-Za-z]").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1?\]):(\d{2,5})").unwrap()
});
static PORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:listening|running|started|serving)\b.*?\bport\b\s*:?\s*(\d{2,5})").unwrap()
});

pub fn strip_ansi(s: &str) -> String {
    ANSI.replace_all(s, "").into_owned()
}

/// First local URL announced in a log line, normalised to http(s)://localhost:PORT.
// ponytail: log scraping, misses servers that never print their port; the per-project URL override covers those.
pub fn extract_url(line: &str) -> Option<String> {
    let caps = URL.captures(line).or_else(|| PORT.captures(line))?;
    let https = line[caps.get(0)?.start()..].starts_with("https");
    Some(format!("{}://localhost:{}", if https { "https" } else { "http" }, &caps[1]))
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ports`
Expected: `ok. 4 passed`.

- [ ] **Step 5: Checkpoint de commit** — `feat: extract local server url from process logs`

---

### Task 4: Configuração e montagem de famílias (`config.rs`)

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs` (adicionar `mod config;`)

**Interfaces:**
- Consumes: `detect::{detect, dir_name, subdirs, CommandSpec}` (Task 2)
- Produces:
  - `pub struct ProjectOverride { pub commands: Option<Vec<CommandSpec>>, pub url: Option<String> }` (Serialize/Deserialize/Default/Clone/Debug/PartialEq)
  - `pub struct Config { pub families: Vec<String>, pub overrides: HashMap<String, ProjectOverride> }`
  - `Config::load(&Path) -> Config` (arquivo ausente → default; corrompido → renomeia para `.json.bak` e devolve default)
  - `Config::save(&self, &Path) -> Result<(), String>`
  - `Config::project(&self, &Path) -> Project`, `Config::families(&self) -> Vec<Family>`
  - `pub struct Project { name, path, kind: String, commands: Vec<CommandSpec>, url: Option<String>, customized: bool }` (Serialize)
  - `pub struct Family { name, path: String, projects: Vec<Project> }` (Serialize)
  - Chave de override = caminho absoluto do projeto (`family_path.join(subdir)` como string).

- [ ] **Step 1: Escrever os testes**

Crie `src-tauri/src/config.rs`:

```rust
use crate::detect::{detect, dir_name, subdirs, CommandSpec};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct ProjectOverride {
    pub commands: Option<Vec<CommandSpec>>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Config {
    pub families: Vec<String>,
    #[serde(default)]
    pub overrides: HashMap<String, ProjectOverride>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub commands: Vec<CommandSpec>,
    pub url: Option<String>,
    pub customized: bool,
}

#[derive(Serialize, Debug)]
pub struct Family {
    pub name: String,
    pub path: String,
    pub projects: Vec<Project>,
}

impl Config {
    pub fn load(_file: &Path) -> Config {
        todo!()
    }
    pub fn save(&self, _file: &Path) -> Result<(), String> {
        todo!()
    }
    pub fn project(&self, _path: &Path) -> Project {
        todo!()
    }
    pub fn families(&self) -> Vec<Family> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub").join("config.json");
        let mut cfg = Config::default();
        cfg.families.push(r"C:\x".into());
        cfg.save(&file).unwrap();
        assert_eq!(Config::load(&file), cfg);
    }

    #[test]
    fn missing_file_is_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Config::load(&dir.path().join("missing.json")), Config::default());
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.json");
        fs::write(&file, "{ not json").unwrap();
        assert_eq!(Config::load(&file), Config::default());
        assert!(dir.path().join("config.json.bak").exists());
    }

    #[test]
    fn families_list_subdirs_and_apply_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let fam = dir.path().join("Node");
        fs::create_dir_all(fam.join("api")).unwrap();
        fs::create_dir_all(fam.join("web")).unwrap();
        fs::write(fam.join("web").join("package.json"), r#"{"scripts":{"dev":"vite"}}"#).unwrap();

        let mut cfg = Config { families: vec![fam.to_string_lossy().into_owned()], ..Default::default() };
        let api_path = fam.join("api").to_string_lossy().into_owned();
        cfg.overrides.insert(
            api_path,
            ProjectOverride {
                commands: Some(vec![CommandSpec { name: "srv".into(), cwd: "".into(), cmd: "go run .".into(), stop: None }]),
                url: Some("http://localhost:9000".into()),
            },
        );

        let fams = cfg.families();
        assert_eq!(fams.len(), 1);
        assert_eq!(fams[0].name, "Node");
        let names: Vec<_> = fams[0].projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["api", "web"]);

        let api = &fams[0].projects[0];
        assert!(api.customized);
        assert_eq!(api.commands[0].cmd, "go run .");
        assert_eq!(api.url.as_deref(), Some("http://localhost:9000"));

        let web = &fams[0].projects[1];
        assert!(!web.customized);
        assert_eq!(web.kind, "node");
        assert_eq!(web.commands[0].cmd, "npm run dev");
    }
}
```

Adicione `mod config;` em `src-tauri/src/lib.rs`.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config`
Expected: 4 FAIL com `not yet implemented`.

- [ ] **Step 3: Implementar** (substitui o bloco `impl Config`)

```rust
impl Config {
    pub fn load(file: &Path) -> Config {
        let Ok(text) = std::fs::read_to_string(file) else {
            return Config::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|_| {
            // Keep the broken file instead of silently overwriting the user's setup.
            let _ = std::fs::rename(file, file.with_extension("json.bak"));
            Config::default()
        })
    }

    pub fn save(&self, file: &Path) -> Result<(), String> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(file, text).map_err(|e| e.to_string())
    }

    pub fn project(&self, path: &Path) -> Project {
        let key = path.to_string_lossy().into_owned();
        let det = detect(path);
        let ov = self.overrides.get(&key).cloned().unwrap_or_default();
        Project {
            name: dir_name(path),
            customized: ov.commands.is_some(),
            commands: ov.commands.unwrap_or(det.commands),
            url: ov.url,
            kind: det.kind,
            path: key,
        }
    }

    pub fn families(&self) -> Vec<Family> {
        self.families
            .iter()
            .map(|f| {
                let p = Path::new(f);
                Family { name: dir_name(p), path: f.clone(), projects: subdirs(p).iter().map(|d| self.project(d)).collect() }
            })
            .collect()
    }
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config`
Expected: `ok. 4 passed`.

- [ ] **Step 5: Checkpoint de commit** — `feat: persist families and per-project command overrides`

---

### Task 5: Gerenciador de processos (`runner.rs`)

**Files:**
- Create: `src-tauri/src/runner.rs`
- Modify: `src-tauri/src/lib.rs` (adicionar `mod runner;`)

**Interfaces:**
- Consumes: `detect::CommandSpec` (Task 2), `ports::{extract_url, strip_ansi}` (Task 3)
- Produces:
  - `pub enum RunnerEvent` serializado com `#[serde(tag = "type", rename_all = "camelCase")]`:
    `started {project, command}`, `log {project, command, line}`, `url {project, command, url}`, `exited {project, command, code: Option<i32>}`
  - `pub type Emit = Arc<dyn Fn(RunnerEvent) + Send + Sync>`
  - `pub struct RunningInfo { pub project: String, pub command: String, pub url: Option<String> }` (Serialize)
  - `Runner::new(Emit)`, `start(&self, project: &str, spec: &CommandSpec) -> Result<(), String>` (no-op se já rodando), `stop(&self, project, command)`, `stop_project(&self, project)`, `stop_all(&self)`, `running(&self) -> Vec<RunningInfo>`
  - Processo: `cmd /C <cmd>` com `raw_arg`, cwd = `project/spec.cwd`, sem janela. Parar = roda `spec.stop` (se houver, aguardando) e depois `taskkill /PID <pid> /T /F`. `exited` só é emitido depois que todos os logs foram emitidos.

- [ ] **Step 1: Escrever os testes**

Crie `src-tauri/src/runner.rs` com tipos, stubs e testes:

```rust
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

impl Runner {
    pub fn new(emit: Emit) -> Self {
        Self { procs: Default::default(), emit }
    }
    pub fn start(&self, _project: &str, _spec: &CommandSpec) -> Result<(), String> {
        todo!()
    }
    pub fn stop(&self, _project: &str, _command: &str) {
        todo!()
    }
    pub fn running(&self) -> Vec<RunningInfo> {
        todo!()
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
        assert!(wait_for(&ev, |e| matches!(e, RunnerEvent::Exited { .. })));
        assert!(dir.path().join("stopped.txt").exists());
        assert!(r.running().is_empty());
    }
}
```

Adicione `mod runner;` em `src-tauri/src/lib.rs`.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runner`
Expected: 2 FAIL com `not yet implemented`.

- [ ] **Step 3: Implementar**

Substitua o `impl Runner` inteiro e adicione `shell` acima dele:

```rust
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
            .insert(key.clone(), Proc { pid: child.id(), cwd, stop: spec.stop.clone(), url: None });
        (self.emit)(RunnerEvent::Started { project: key.0.clone(), command: key.1.clone() });

        let out = self.spawn_reader(key.clone(), child.stdout.take().unwrap());
        let err = self.spawn_reader(key.clone(), child.stderr.take().unwrap());
        let (procs, emit) = (self.procs.clone(), self.emit.clone());
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            // ponytail: waits for pipes to close; a detached grandchild holding them delays `exited`.
            let _ = out.join();
            let _ = err.join();
            procs.lock().unwrap().remove(&key);
            emit(RunnerEvent::Exited { project: key.0, command: key.1, code });
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
        let Some(p) = self.procs.lock().unwrap().get(&key).cloned() else {
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
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runner`
Expected: `ok. 2 passed`. Depois rode a suíte inteira: `cargo test --manifest-path src-tauri/Cargo.toml` → `ok. 17 passed`.

- [ ] **Step 5: Checkpoint de commit** — `feat: spawn and stop project processes with log streaming`

---

### Task 6: Comandos Tauri (`lib.rs`)

**Files:**
- Modify: `src-tauri/src/lib.rs` (substituir inteiro)

**Interfaces:**
- Consumes: Tasks 2–5.
- Produces (invocáveis do JS, args em camelCase):
  - `get_families() -> Family[]`
  - `add_family({ path }) -> void`, `remove_family({ path }) -> void`
  - `save_override({ project, value: ProjectOverride }) -> void` (value `{commands:null,url:null}` remove o override)
  - `start_project({ project }) -> void` (erro se não houver comandos), `stop_project({ project }) -> void`
  - `running() -> RunningInfo[]`
  - Evento global `"proc"` com payload `RunnerEvent`.

- [ ] **Step 1: Substituir `src-tauri/src/lib.rs`**

```rust
mod config;
mod detect;
mod ports;
mod runner;

use config::{Config, Family, ProjectOverride};
use runner::{Runner, RunnerEvent, RunningInfo};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

struct AppState {
    config: Mutex<Config>,
    file: PathBuf,
    runner: Runner,
}

type R<T> = Result<T, String>;

#[tauri::command]
fn get_families(state: State<AppState>) -> Vec<Family> {
    state.config.lock().unwrap().families()
}

#[tauri::command]
fn add_family(state: State<AppState>, path: String) -> R<()> {
    let mut cfg = state.config.lock().unwrap();
    if !cfg.families.contains(&path) {
        cfg.families.push(path);
    }
    cfg.save(&state.file)
}

#[tauri::command]
fn remove_family(state: State<AppState>, path: String) -> R<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.families.retain(|f| f != &path);
    cfg.save(&state.file)
}

#[tauri::command]
fn save_override(state: State<AppState>, project: String, value: ProjectOverride) -> R<()> {
    let mut cfg = state.config.lock().unwrap();
    if value == ProjectOverride::default() {
        cfg.overrides.remove(&project);
    } else {
        cfg.overrides.insert(project, value);
    }
    cfg.save(&state.file)
}

// async = off the main thread, so a slow `docker compose stop` doesn't freeze the UI.
#[tauri::command(async)]
fn start_project(state: State<'_, AppState>, project: String) -> R<()> {
    let p = state.config.lock().unwrap().project(Path::new(&project));
    if p.commands.is_empty() {
        return Err("Nenhum comando configurado para este projeto".into());
    }
    for spec in &p.commands {
        state.runner.start(&project, spec)?;
    }
    Ok(())
}

#[tauri::command(async)]
fn stop_project(state: State<'_, AppState>, project: String) -> R<()> {
    state.runner.stop_project(&project);
    Ok(())
}

#[tauri::command]
fn running(state: State<AppState>) -> Vec<RunningInfo> {
    state.runner.running()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let file = app.path().app_config_dir()?.join("config.json");
            let handle = app.handle().clone();
            let runner = Runner::new(Arc::new(move |e: RunnerEvent| {
                let _ = handle.emit("proc", e);
            }));
            app.manage(AppState { config: Mutex::new(Config::load(&file)), file, runner });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_families,
            add_family,
            remove_family,
            save_override,
            start_project,
            stop_project,
            running
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Don't leave orphan dev servers behind when the window closes.
                app.state::<AppState>().runner.stop_all();
            }
        });
}
```

- [ ] **Step 2: Compilar e rodar todos os testes Rust**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: compila sem erros; `ok. 17 passed`.

- [ ] **Step 3: Checkpoint de commit** — `feat: expose families, overrides and process control as tauri commands`

---

### Task 7: Camada de dados do frontend (`api.ts`, `runtime.ts`, `useRuntime.ts`)

**Files:**
- Create: `src/api.ts`, `src/runtime.ts`, `src/useRuntime.ts`, `tests/runtime.test.ts`

**Interfaces:**
- Consumes: comandos/evento da Task 6.
- Produces:
  - `src/api.ts`: tipos `CommandSpec`, `Project`, `Family`, `ProjectOverride`, `RunnerEvent`, `RunningInfo`; objeto `api` com `families, addFamily, removeFamily, saveOverride, start, stop, running, onEvent`
  - `src/runtime.ts`: `type CmdState = { running: boolean; url: string | null; logs: string[]; code: number | null }`, `type Runtime = Record<string, Record<string, CmdState>>` (projeto → comando → estado), `reduce(rt, e)`, `projectRunning(rt, path)`, `projectUrl(rt, project)`, `portOf(url)`, `MAX_LOG = 2000`
  - `src/useRuntime.ts`: `useRuntime(): Runtime`

- [ ] **Step 1: Escrever `src/api.ts`**

```ts
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
};
export type Family = { name: string; path: string; projects: Project[] };
export type ProjectOverride = { commands: CommandSpec[] | null; url: string | null };
export type RunnerEvent =
  | { type: "started"; project: string; command: string }
  | { type: "log"; project: string; command: string; line: string }
  | { type: "url"; project: string; command: string; url: string }
  | { type: "exited"; project: string; command: string; code: number | null };
export type RunningInfo = { project: string; command: string; url: string | null };

export const api = {
  families: () => invoke<Family[]>("get_families"),
  addFamily: (path: string) => invoke<void>("add_family", { path }),
  removeFamily: (path: string) => invoke<void>("remove_family", { path }),
  saveOverride: (project: string, value: ProjectOverride) => invoke<void>("save_override", { project, value }),
  start: (project: string) => invoke<void>("start_project", { project }),
  stop: (project: string) => invoke<void>("stop_project", { project }),
  running: () => invoke<RunningInfo[]>("running"),
  onEvent: (cb: (e: RunnerEvent) => void) => listen<RunnerEvent>("proc", (e) => cb(e.payload)),
};
```

- [ ] **Step 2: Escrever o teste do reducer**

Crie `tests/runtime.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { reduce, projectRunning, projectUrl, portOf, MAX_LOG, type Runtime } from "../src/runtime.ts";

const P = "C:\\Projetos\\Node\\web";
const project = { name: "web", path: P, kind: "node", commands: [], url: null, customized: false };

test("started → log → url → exited", () => {
  let rt: Runtime = {};
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "log", project: P, command: "dev", line: "hello" });
  rt = reduce(rt, { type: "url", project: P, command: "dev", url: "http://localhost:5173" });
  assert.equal(projectRunning(rt, P), true);
  assert.deepEqual(rt[P].dev.logs, ["hello"]);
  assert.equal(projectUrl(rt, project), "http://localhost:5173");

  rt = reduce(rt, { type: "exited", project: P, command: "dev", code: 1 });
  assert.equal(projectRunning(rt, P), false);
  assert.equal(rt[P].dev.code, 1);
  assert.equal(projectUrl(rt, project), null);
});

test("new run clears previous logs", () => {
  let rt: Runtime = {};
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "log", project: P, command: "dev", line: "old" });
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  assert.deepEqual(rt[P].dev.logs, []);
});

test("override url wins over detected url", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "url", project: P, command: "dev", url: "http://localhost:5173" });
  assert.equal(projectUrl(rt, { ...project, url: "http://localhost:8080" }), "http://localhost:8080");
});

test("logs are capped", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "dev" });
  for (let i = 0; i < MAX_LOG + 100; i++) rt = reduce(rt, { type: "log", project: P, command: "dev", line: `l${i}` });
  assert.equal(rt[P].dev.logs.length, MAX_LOG);
  assert.equal(rt[P].dev.logs.at(-1), `l${MAX_LOG + 99}`);
});

test("portOf", () => {
  assert.equal(portOf("http://localhost:5173"), "5173");
  assert.equal(portOf("https://localhost"), "443");
  assert.equal(portOf("nope"), "?");
});
```

- [ ] **Step 3: Rodar e ver falhar**

Run: `npm test`
Expected: FAIL com `Cannot find module ... src/runtime.ts`.

- [ ] **Step 4: Implementar `src/runtime.ts`**

```ts
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
```

- [ ] **Step 5: Rodar e ver passar**

Run: `npm test`
Expected: `# pass 5`, `# fail 0`.

- [ ] **Step 6: Escrever `src/useRuntime.ts`**

```ts
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
```

- [ ] **Step 7: Verificar tipos**

Run: `npm run build`
Expected: sem erros de TypeScript (o template ainda renderiza o App antigo; tudo bem).

- [ ] **Step 8: Checkpoint de commit** — `feat: frontend api bindings and runtime state reducer`

---

### Task 8: UI — layout, sidebar, painel e editor

**Files:**
- Create: `src/Sidebar.tsx`, `src/ProjectPanel.tsx`, `src/CommandEditor.tsx`
- Modify: `src/App.tsx` (substituir inteiro), `src/App.css` (substituir inteiro)
- Delete: `src/assets/react.svg` (não usado mais)

**Interfaces:**
- Consumes: `api` e tipos de `src/api.ts`; `Runtime`, `projectRunning`, `projectUrl`, `portOf` de `src/runtime.ts`; `useRuntime` de `src/useRuntime.ts`; `open`, `ask` de `@tauri-apps/plugin-dialog`; `openUrl` de `@tauri-apps/plugin-opener`.
- Produces: componentes `Sidebar`, `ProjectPanel`, `CommandEditor` (props abaixo).

- [ ] **Step 1: `src/App.tsx`**

```tsx
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Family } from "./api";
import { useRuntime } from "./useRuntime";
import { Sidebar } from "./Sidebar";
import { ProjectPanel } from "./ProjectPanel";
import "./App.css";

export default function App() {
  const [families, setFamilies] = useState<Family[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
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
  const project = families.flatMap((f) => f.projects).find((p) => p.path === selected) ?? null;

  return (
    <div className="app">
      <Sidebar
        families={families}
        rt={rt}
        selected={selected}
        onSelect={setSelected}
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
          <ProjectPanel key={project.path} project={project} rt={rt} run={run} />
        ) : (
          <p className="empty">Selecione um projeto na barra lateral.</p>
        )}
      </main>
    </div>
  );
}
```

- [ ] **Step 2: `src/Sidebar.tsx`**

```tsx
import { useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import type { Family } from "./api";
import { type Runtime, projectRunning, projectUrl, portOf } from "./runtime";

type Props = {
  families: Family[];
  rt: Runtime;
  selected: string | null;
  onSelect: (path: string) => void;
  onAdd: () => void;
  onRemove: (path: string) => void;
  onRefresh: () => void;
};

export function Sidebar({ families, rt, selected, onSelect, onAdd, onRemove, onRefresh }: Props) {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const remove = async (f: Family) => {
    if (await ask(`Remover a família "${f.name}"? A pasta não é apagada.`, { title: "Remover família" })) onRemove(f.path);
  };

  return (
    <aside className="sidebar">
      <header>
        <span>Projetos</span>
        <span>
          <button title="Recarregar pastas" onClick={onRefresh}>⟳</button>
          <button title="Adicionar família" onClick={onAdd}>+</button>
        </span>
      </header>
      {families.length === 0 && <p className="hint">Adicione uma família (ex.: a pasta Node) com o botão +.</p>}
      {families.map((f) => (
        <section key={f.path}>
          <div className="family">
            <button className="family-name" onClick={() => setCollapsed({ ...collapsed, [f.path]: !collapsed[f.path] })}>
              {collapsed[f.path] ? "▸" : "▾"} {f.name}
            </button>
            <button title="Remover família" onClick={() => remove(f)}>×</button>
          </div>
          {!collapsed[f.path] &&
            f.projects.map((p) => {
              const url = projectUrl(rt, p);
              return (
                <button
                  key={p.path}
                  className={`project ${selected === p.path ? "active" : ""}`}
                  onClick={() => onSelect(p.path)}
                >
                  <span className={`dot ${projectRunning(rt, p.path) ? "on" : ""}`} />
                  <span className="name">{p.name}</span>
                  {url && <span className="port">:{portOf(url)}</span>}
                </button>
              );
            })}
        </section>
      ))}
    </aside>
  );
}
```

- [ ] **Step 3: `src/ProjectPanel.tsx`**

```tsx
import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type Project } from "./api";
import { type Runtime, projectRunning, projectUrl, portOf } from "./runtime";
import { CommandEditor } from "./CommandEditor";

type Props = { project: Project; rt: Runtime; run: (p: Promise<unknown>) => void };

export function ProjectPanel({ project: p, rt, run }: Props) {
  const [tab, setTab] = useState(0);
  const [editing, setEditing] = useState(false);
  const running = projectRunning(rt, p.path);
  const url = projectUrl(rt, p);
  const states = rt[p.path] ?? {};
  const current = p.commands[tab] ?? p.commands[0];

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
          <button disabled={!url} onClick={() => url && run(openUrl(url))}>
            Abrir no navegador
          </button>
          {running ? (
            <button className="danger" onClick={() => run(api.stop(p.path))}>Parar</button>
          ) : (
            <button className="primary" disabled={p.commands.length === 0} onClick={() => run(api.start(p.path))}>
              Iniciar
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
            setTab(0);
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
            {p.commands.map((c, i) => {
              const s = states[c.name];
              return (
                <button key={c.name} className={i === tab ? "active" : ""} onClick={() => setTab(i)}>
                  <span className={`dot ${s?.running ? "on" : s?.code ? "err" : ""}`} /> {c.name}
                </button>
              );
            })}
          </nav>
          <code className="cmdline">
            {current.cwd ? `${current.cwd}> ` : "> "}
            {current.cmd}
          </code>
          <Logs lines={states[current.name]?.logs ?? []} />
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
```

- [ ] **Step 4: `src/CommandEditor.tsx`**

```tsx
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
```

- [ ] **Step 5: `src/App.css`** (substituir inteiro)

```css
:root {
  color-scheme: dark;
  --bg: #1a1a1a;
  --panel: #212121;
  --hover: #2a2a2a;
  --active: #333;
  --border: #2e2e2e;
  --text: #e8e6e3;
  --muted: #9a9893;
  --accent: #d97757;
  --ok: #4caf50;
  --err: #e5534b;
  font-family: "Segoe UI", system-ui, sans-serif;
  font-size: 14px;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--text); }
button {
  font: inherit; color: inherit; background: var(--hover); border: 1px solid var(--border);
  border-radius: 6px; padding: 6px 12px; cursor: pointer;
}
button:hover:not(:disabled) { background: var(--active); }
button:disabled { opacity: 0.4; cursor: default; }
button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
button.danger { background: var(--err); border-color: var(--err); color: #fff; }
input {
  font: inherit; color: inherit; background: var(--bg); border: 1px solid var(--border);
  border-radius: 6px; padding: 6px 8px;
}

.app { display: flex; height: 100vh; }
.sidebar {
  width: 280px; flex-shrink: 0; background: var(--panel); border-right: 1px solid var(--border);
  overflow-y: auto; padding: 8px;
}
.sidebar > header { display: flex; justify-content: space-between; align-items: center; padding: 4px 8px 12px; font-weight: 600; }
.sidebar header button, .family button { background: none; border: none; padding: 2px 8px; }
.family { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; }
.family-name { flex: 1; text-align: left; font-weight: 600; }
.project {
  display: flex; align-items: center; gap: 8px; width: 100%; background: none; border: none;
  text-align: left; padding: 6px 10px;
}
.project.active { background: var(--active); }
.project .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; background: var(--muted); opacity: 0.5; }
.dot.on { background: var(--ok); opacity: 1; }
.dot.err { background: var(--err); opacity: 1; }
.port { color: var(--ok); font-family: Consolas, monospace; font-size: 12px; }

main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.empty, .hint { color: var(--muted); padding: 8px; margin: 0; }
.error { background: var(--err); color: #fff; padding: 8px 16px; cursor: pointer; }
.panel { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 12px; padding: 16px 20px; }
.panel > header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
.panel h1 { margin: 0; font-size: 20px; }
.panel small { color: var(--muted); }
.badge { font-size: 11px; font-weight: normal; background: var(--hover); border-radius: 4px; padding: 2px 6px; vertical-align: middle; }
.actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); }
.tabs button { display: flex; align-items: center; gap: 6px; background: none; border: none; border-radius: 6px 6px 0 0; }
.tabs button.active { background: var(--hover); }
.cmdline { color: var(--muted); font-family: Consolas, monospace; font-size: 12px; }
.logs {
  flex: 1; min-height: 0; overflow: auto; margin: 0; padding: 10px; background: #111;
  border: 1px solid var(--border); border-radius: 6px; font: 12px/1.45 Consolas, monospace;
  white-space: pre-wrap; word-break: break-all;
}
.editor { display: flex; flex-direction: column; gap: 8px; background: var(--panel); border: 1px solid var(--border); border-radius: 8px; padding: 12px; }
.editor .row { display: flex; gap: 6px; }
.editor .row input { width: 150px; }
.editor .row input.grow { flex: 1; width: auto; }
.editor label { display: flex; gap: 8px; align-items: center; color: var(--muted); }
.editor label input { flex: 1; }
```

- [ ] **Step 6: Remover asset não usado e verificar build**

```bash
rm -f src/assets/react.svg
npm run build
npm test
```

Expected: `vite build` sem erros de TS; `npm test` → `# pass 5`. Se o `tsc` acusar `noUnusedLocals` em algo do template (ex.: `src/assets` vazio), remova o import/arquivo apontado.

- [ ] **Step 7: Checkpoint de commit** — `feat: sidebar with families, project panel, logs and command editor`

---

### Task 9: Verificação end-to-end manual

**Files:** nenhum (só ajustes se algo falhar).

- [ ] **Step 1: Subir o app em modo dev**

Run (em background, demora na 1ª vez): `npm run tauri dev`
Expected: janela "Gerenciador de Projetos" abre, sidebar vazia com dica "Adicione uma família...".

- [ ] **Step 2: Roteiro de verificação** (marcar cada item)

1. `+` → escolher uma pasta de família → aparecem as subpastas, com os projetos Vite detectados como `node`.
2. `+` → outra família → um projeto com front e back em subpastas mostra duas abas; um com `docker-compose.yml` aparece como `docker`.
3. Selecionar um projeto Vite (qualquer um com `npm run dev` via Vite) → **Iniciar** → aba de log enche, ponto verde na sidebar, `:5173` (ou a porta real) aparece na sidebar e "Porta N" no painel.
4. **Abrir no navegador** abre o navegador padrão na URL.
5. **Parar** → processo morre (conferir em `netstat -ano | findstr :5173` que a porta liberou), ponto apaga.
6. **Configurar** num projeto `unknown` (ex.: uma pasta só com `requirements.txt`) → adicionar comando, salvar → fecha app e reabre → override persiste; "Restaurar detecção automática" volta ao detectado.
7. Iniciar um projeto e **fechar a janela** → reabrir e checar com `netstat` que o servidor não ficou órfão.
8. Fechar o app.

- [ ] **Step 3: Rodar todas as suítes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run build
```

Expected: `ok. 17 passed`; `# pass 5`; build sem erros.

- [ ] **Step 4: Checkpoint final** — apresentar arquivos alterados, comandos de verificação e resultados reais, `git diff --stat`, e perguntar sobre commit. **Push** para `origin main` (`git push -u origin main`) só com autorização separada e explícita.

---

## Fora do escopo (deliberadamente)

- Detecção de porta via sockets abertos pelo processo (usa log + URL fixa). Adicionar se muitos projetos não imprimirem a porta.
- Portas mapeadas de `docker-compose.yml` (usar URL fixa por enquanto).
- Instalador (`npm run tauri build` já gera `.msi`/`.exe` em `src-tauri/target/release/bundle/` quando quiser).
- Terminal interativo (stdin) para os processos.
