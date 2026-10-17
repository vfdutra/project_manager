mod config;
mod detect;
mod ports;
mod runner;

use config::{Config, Family, ProjectOverride};
use runner::{Runner, RunnerEvent, RunningInfo};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

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

#[tauri::command]
fn set_hidden(state: State<AppState>, project: String, hidden: bool) -> R<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.hidden.retain(|p| p != &project);
    if hidden {
        cfg.hidden.push(project);
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
    // "Iniciar todos" skips sub-projects the user hid.
    let visible: Vec<_> = p.commands.iter().filter(|c| !p.hidden_commands.contains(&c.name)).collect();
    if visible.is_empty() {
        return Err("Todos os sub-projetos estão ocultos".into());
    }
    for spec in visible {
        state.runner.start(&project, spec)?;
    }
    Ok(())
}

#[tauri::command(async)]
fn stop_project(state: State<'_, AppState>, project: String) -> R<()> {
    state.runner.stop_project(&project);
    Ok(())
}

#[tauri::command(async)]
fn start_command(state: State<'_, AppState>, project: String, command: String) -> R<()> {
    let p = state.config.lock().unwrap().project(Path::new(&project));
    let spec = p
        .commands
        .iter()
        .find(|c| c.name == command)
        .ok_or(format!("Comando \"{command}\" não encontrado"))?;
    state.runner.start(&project, spec)
}

#[tauri::command(async)]
fn stop_command(state: State<'_, AppState>, project: String, command: String) -> R<()> {
    state.runner.stop(&project, &command);
    Ok(())
}

#[tauri::command]
fn running(state: State<AppState>) -> Vec<RunningInfo> {
    state.runner.running()
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch just brings the running window back.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let file = app.path().app_config_dir()?.join("config.json");
            let handle = app.handle().clone();
            let runner = Runner::new(Arc::new(move |e: RunnerEvent| {
                let _ = handle.emit("proc", e);
            }));
            app.manage(AppState { config: Mutex::new(Config::load(&file)), file, runner });

            let show = MenuItem::with_id(app, "show", "Abrir", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair (para todos os projetos)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Gerenciador de Projetos")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Keep running in the tray; "Sair" in the tray menu really quits.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_families,
            add_family,
            remove_family,
            save_override,
            set_hidden,
            start_project,
            stop_project,
            start_command,
            stop_command,
            running
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Don't leave orphan dev servers behind when the app quits (tray "Sair").
                app.state::<AppState>().runner.stop_all();
            }
        });
}
