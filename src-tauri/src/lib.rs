//! REZ — Dock Expo-Glass pour Windows.

mod commands;
mod config;
mod dock;
mod legacy;
mod monitor;
mod state;
mod win;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use win::taskbar;

/// Ancien identifiant (≤ 1.2.0) : `app.rez.dock`, nom produit « REZ » partagé avec REZ Island.
/// On reprend ses réglages (et l'état de la barre Windows à restaurer) une seule fois.
/// Le dossier de REZ Island (`com.rez.island`) n'est jamais lu ni modifié.
fn migrate_legacy_dir(app: &tauri::AppHandle) {
    let Ok(dir) = app.path().app_config_dir() else { return };
    let Some(old) = dir.parent().map(|p| p.join("app.rez.dock")) else { return };
    if old == dir || !old.is_dir() {
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    for name in ["config.json", "taskbar-state.json"] {
        let (src, dst) = (old.join(name), dir.join(name));
        if src.is_file() && !dst.exists() {
            let _ = std::fs::copy(&src, &dst);
        }
    }
}

pub fn run() {
    // Filet de sécurité : la barre Windows est TOUJOURS restaurée, même sur panic.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        taskbar::restore();
        default_hook(info);
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Relancer REZ alors qu'il tourne déjà = ouvrir les réglages.
            // Hors du thread principal (création de fenêtre bloquante sinon sous Windows).
            let app = app.clone();
            std::thread::spawn(move || {
                let _ = commands::open_settings_window(&app);
            });
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_windows,
            commands::activate_window,
            commands::focus_window,
            commands::minimize_window,
            commands::close_window,
            commands::launch,
            commands::open_location,
            commands::shell_action,
            commands::get_icon,
            commands::app_info,
            commands::default_pins,
            commands::get_tray_apps,
            commands::get_background_processes,
            commands::kill_processes,
            commands::set_surfaces,
            commands::is_gaming,
            commands::get_layout,
            commands::open_settings,
            commands::prepare_update,
            commands::quit_app,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            migrate_legacy_dir(&handle);
            legacy::cleanup(&handle);
            config::load(&handle);
            if let Ok(dir) = handle.path().app_config_dir() {
                taskbar::init(dir.join("taskbar-state.json"));
            }

            let dock = app.get_webview_window(dock::PRIMARY).expect("fenêtre dock manquante");
            let _ = dock.set_ignore_cursor_events(true);

            // Barres Windows masquées + un dock (et son verre natif) par écran.
            commands::apply_native(&handle);
            dock::set_visible(&handle, true);

            monitor::start(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == dock::PRIMARY {
                    taskbar::restore();
                    window.app_handle().exit(0);
                } else if dock::is_dock(label) {
                    api.prevent_close(); // docks secondaires : gérés par REZ (Alt+F4 ignoré)
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("erreur au démarrage de REZ")
        .run(|_app, event| {
            if let RunEvent::Exit = event {
                taskbar::restore();
            }
        });
}
