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
    #[serde(default)]
    pub hidden: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub commands: Vec<CommandSpec>,
    pub url: Option<String>,
    pub customized: bool,
    pub hidden: bool,
    /// Names of this project's commands hidden as sub-projects (key `<path>::<name>` in Config.hidden).
    pub hidden_commands: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Family {
    pub name: String,
    pub path: String,
    pub projects: Vec<Project>,
}

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
        let customized = ov.commands.is_some();
        let commands = ov.commands.unwrap_or(det.commands);
        let hidden_commands = commands
            .iter()
            .filter(|c| self.hidden.contains(&format!("{key}::{}", c.name)))
            .map(|c| c.name.clone())
            .collect();
        Project {
            name: dir_name(path),
            customized,
            commands,
            url: ov.url,
            kind: det.kind,
            hidden: self.hidden.contains(&key),
            hidden_commands,
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
            api_path.clone(),
            ProjectOverride {
                commands: Some(vec![CommandSpec { name: "srv".into(), cwd: "".into(), cmd: "go run .".into(), stop: None }]),
                url: Some("http://localhost:9000".into()),
            },
        );
        cfg.hidden.push(api_path);
        cfg.hidden.push(format!("{}::node", fam.join("web").to_string_lossy()));

        let fams = cfg.families();
        assert_eq!(fams.len(), 1);
        assert_eq!(fams[0].name, "Node");
        let names: Vec<_> = fams[0].projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["api", "web"]);

        let api = &fams[0].projects[0];
        assert!(api.customized);
        assert!(api.hidden);
        assert_eq!(api.commands[0].cmd, "go run .");
        assert_eq!(api.url.as_deref(), Some("http://localhost:9000"));

        let web = &fams[0].projects[1];
        assert!(!web.customized);
        assert!(!web.hidden);
        assert_eq!(web.hidden_commands, ["node"]);
        assert!(api.hidden_commands.is_empty());
        assert_eq!(web.kind, "node");
        assert_eq!(web.commands[0].cmd, "npm run dev");
    }
}
