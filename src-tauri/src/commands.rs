//! Commandes IPC exposées au frontend.

use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::state::{Surface, SHARED};
use crate::win::{appinfo, glass, icons, shell, taskbar, tray, windows as wins};
use crate::{config, dock};

#[tauri::command]
pub fn get_config() -> Value {
    SHARED.config.lock().unwrap().clone()
}

/// `persist=false` : aperçu live (pas d'écriture disque, pas de changement natif lourd).
#[tauri::command]
pub fn save_config(app: AppHandle, config: Value, source: String, persist: Option<bool>) -> Result<(), String> {
    let persist = persist.unwrap_or(true);
    let previous = std::mem::replace(&mut *SHARED.config.lock().unwrap(), config.clone());
    let _ = app.emit("config-changed", json!({ "config": config, "source": source }));
    if persist {
        config::write(&app, &config)?;
        if config::native_fingerprint(&previous) != config::native_fingerprint(&config) {
            apply_native(&app);
        }
    }
    Ok(())
}

/// Applique les réglages natifs (barre Windows, verre, démarrage auto).
pub fn apply_native(app: &AppHandle) {
    dock::layout(app);
    glass::set_mode(glass::BlurMode::from_intensity(config::blur()));
    let auto = app.autolaunch();
    let want = config::autostart();
    if auto.is_enabled().unwrap_or(false) != want {
        let _ = if want { auto.enable() } else { auto.disable() };
    }
}

#[tauri::command]
pub fn get_windows() -> Vec<wins::WinInfo> {
    SHARED.windows.lock().unwrap().clone()
}

#[tauri::command]
pub fn activate_window(hwnd: isize) {
    wins::activate(hwnd);
}

#[tauri::command]
pub fn focus_window(hwnd: isize) {
    wins::focus(hwnd);
}

#[tauri::command]
pub fn minimize_window(hwnd: isize) {
    wins::minimize(hwnd);
}

#[tauri::command]
pub fn close_window(hwnd: isize) {
    wins::close(hwnd);
}

#[tauri::command]
pub fn launch(path: String, args: Option<String>) -> Result<(), String> {
    shell::launch(&path, args.as_deref())
}

#[tauri::command]
pub fn open_location(path: String) -> Result<(), String> {
    shell::open_location(&path)
}

#[tauri::command]
pub fn shell_action(action: String) -> Result<(), String> {
    shell::action(&action)
}

#[tauri::command]
pub async fn get_icon(path: String, size: Option<i32>) -> Option<String> {
    let size = size.unwrap_or(128).clamp(16, 256);
    tauri::async_runtime::spawn_blocking(move || icons::icon_data_url(&path, size))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub async fn app_info(path: String) -> appinfo::AppInfo {
    let p = path.clone();
    tauri::async_runtime::spawn_blocking(move || appinfo::app_info(&p))
        .await
        .unwrap_or_else(|_| appinfo::AppInfo {
            path: path.clone(),
            args: None,
            name: path.clone(),
            target: path,
            match_key: String::new(),
            icon: None,
        })
}

#[tauri::command]
pub async fn default_pins() -> Vec<appinfo::AppInfo> {
    tauri::async_runtime::spawn_blocking(appinfo::default_pins).await.unwrap_or_default()
}

#[tauri::command]
pub async fn get_tray_apps() -> Vec<tray::TrayApp> {
    tauri::async_runtime::spawn_blocking(tray::tray_apps).await.unwrap_or_default()
}

#[tauri::command]
pub async fn get_background_processes() -> Vec<tray::BgProcess> {
    tauri::async_runtime::spawn_blocking(tray::background_processes).await.unwrap_or_default()
}

#[tauri::command]
pub fn kill_processes(pids: Vec<u32>) -> usize {
    tray::kill(&pids)
}

#[tauri::command]
pub fn set_surfaces(glass: Vec<Surface>, hit: Vec<Surface>, reveal: Option<Vec<Surface>>) {
    dock::set_surfaces(&glass, &hit, &reveal.unwrap_or_default());
}

#[tauri::command]
pub fn get_layout() -> dock::Insets {
    dock::insets()
}

#[tauri::command]
pub fn is_gaming() -> bool {
    SHARED.gaming.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app).map_err(|e| e.to_string())
}

pub fn open_settings_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("REZ — Réglages")
        .inner_size(1040.0, 720.0)
        .min_inner_size(860.0, 600.0)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .theme(Some(tauri::Theme::Dark))
        .effects(tauri::utils::config::WindowEffectsConfig {
            effects: vec![tauri::window::Effect::Mica],
            ..Default::default()
        })
        .disable_drag_drop_handler()
        .center()
        .visible(true)
        .build()?;
    w.set_focus()?;
    Ok(())
}

/// À appeler AVANT l'installation d'une mise à jour (l'installeur termine le processus).
#[tauri::command]
pub fn prepare_update() {
    taskbar::restore();
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    taskbar::restore();
    app.exit(0);
}
