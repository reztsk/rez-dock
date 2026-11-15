//! Threads de fond :
//!  - `tick` (250 ms) : mode gaming (plein écran), liste des fenêtres, maintien barre native masquée.
//!  - `hit`  (16 ms)  : click-through hors des zones du dock, suivi de la fenêtre active,
//!                     zone de réveil du masquage auto, touche Windows (GetAsyncKeyState, sans hook).
//!
//! Mode gaming = 100 % passif : GetForegroundWindow + GetClientRect + MonitorInfo +
//! SHQueryUserNotificationState. Aucun handle sur le jeu, aucun hook, aucune lecture mémoire.
//! Quand un jeu est plein écran, TOUT le polling de fenêtres s'arrête.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetCursorPos, GetForegroundWindow, IsWindowVisible};

use crate::state::SHARED;
use crate::win::{self, hwnd_val, taskbar, windows as wins};
use crate::{config, dock};

pub fn start(app: AppHandle) {
    let a = app.clone();
    thread::Builder::new().name("rez-tick".into()).spawn(move || tick_loop(a)).ok();
    thread::Builder::new().name("rez-hit".into()).spawn(move || hit_loop(app)).ok();
}

fn is_fullscreen_foreground() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || !IsWindowVisible(fg).as_bool() {
            return false;
        }
        if wins::pid_of(fg) == std::process::id() {
            return false;
        }
        let class = wins::class_name(fg);
        if matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "XamlExplorerHostIslandWindow"
                | "Windows.UI.Core.CoreWindow" | "MultitaskingViewFrame" | "ForegroundStaging"
        ) {
            return false;
        }
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
                return true;
            }
        }
        // Plein écran fenêtré sans bordure : la zone client couvre tout le moniteur.
        let mut rc = RECT::default();
        if GetClientRect(fg, &mut rc).is_err() {
            return false;
        }
        let mut tl = POINT { x: rc.left, y: rc.top };
        let mut br = POINT { x: rc.right, y: rc.bottom };
        let _ = ClientToScreen(fg, &mut tl);
        let _ = ClientToScreen(fg, &mut br);
        let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        tl.x <= m.left && tl.y <= m.top && br.x >= m.right && br.y >= m.bottom
    }
}

fn set_gaming(app: &AppHandle, active: bool) {
    SHARED.gaming.store(active, Ordering::SeqCst);
    let _ = app.emit("gaming", active);
    if active {
        // Laisse l'animation de sortie se jouer, puis masque réellement les fenêtres.
        let a = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(380));
            if SHARED.gaming.load(Ordering::SeqCst) {
                dock::set_visible(&a, false);
                let _ = a.get_webview_window("dock").map(|w| w.set_ignore_cursor_events(true));
            }
        });
    } else {
        dock::set_visible(app, true);
    }
}

fn tick_loop(app: AppHandle) {
    let mut tick: u64 = 0;
    let mut pending = false;
    let mut streak = 0u32;
    let mut cache: HashMap<u32, String> = HashMap::new();
    let mut last_monitor = win::primary_monitor();

    loop {
        thread::sleep(Duration::from_millis(250));
        tick = tick.wrapping_add(1);
        let gaming = SHARED.gaming.load(Ordering::SeqCst);

        // --- Mode gaming (anti-conflit anti-cheat) ---
        if config::gaming_mode() {
            let fs = is_fullscreen_foreground();
            if fs == pending {
                streak = streak.saturating_add(1);
            } else {
                pending = fs;
                streak = 0;
            }
            // Stable sur ~500 ms pour éviter les faux positifs (alt-tab, splash screens).
            if streak >= 2 && fs != gaming {
                set_gaming(&app, fs);
            }
        } else if gaming {
            set_gaming(&app, false);
        }
        if SHARED.gaming.load(Ordering::SeqCst) {
            continue; // Rien d'autre tant qu'un jeu est plein écran.
        }

        // --- Fenêtres ouvertes ---
        if tick % 3 == 0 {
            let list = wins::enumerate(&mut cache);
            let changed = {
                let mut cur = SHARED.windows.lock().unwrap();
                if *cur != list {
                    *cur = list.clone();
                    true
                } else {
                    false
                }
            };
            if changed {
                let _ = app.emit("windows-changed", &list);
            }
        }

        // --- Barre native masquée / zone de travail (contrôle doux : 1 s, n'agit que si besoin) ---
        if tick % 4 == 0 {
            taskbar::enforce();
        }

        // --- Changement d'écran / DPI ---
        if tick % 8 == 0 {
            let m = win::primary_monitor();
            if m.0 != last_monitor.0 || (m.2 - last_monitor.2).abs() > f64::EPSILON {
                last_monitor = m;
                dock::layout(&app);
                let _ = app.emit("display-changed", m.2);
            }
        }
    }
}

/// Menu Démarrer / Recherche / Centre de notifications au premier plan ?
/// (vérifié seulement quand la fenêtre active change : coût nul le reste du temps)
fn is_shell_flyout(fg: HWND) -> bool {
    let class = wins::class_name(fg);
    match class.as_str() {
        "XamlExplorerHostIslandWindow" => true,
        "Windows.UI.Core.CoreWindow" => {
            let exe = wins::exe_of_pid(wins::pid_of(fg)).unwrap_or_default().to_ascii_lowercase();
            ["startmenuexperiencehost.exe", "searchhost.exe", "searchapp.exe", "shellexperiencehost.exe"]
                .iter()
                .any(|n| exe.ends_with(n))
        }
        _ => false,
    }
}

/// Touche Windows enfoncée ? GetAsyncKeyState = simple lecture de l'état du clavier
/// (aucun hook, aucune injection) — et jamais appelé pendant un jeu plein écran.
fn win_key_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0 || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0 }
}

fn in_rects(list: &[crate::state::ScreenRect], pt: POINT) -> bool {
    list.iter().any(|&(l, t, r, b)| pt.x >= l && pt.x < r && pt.y >= t && pt.y < b)
}

fn hit_loop(app: AppHandle) {
    let own = std::process::id();
    let mut last_inside = false;
    let mut last_reveal = false;
    let mut last_shell = false;
    let mut flyout = false;
    let mut last_fg_raw: isize = 0;
    let Some(dock_win) = app.get_webview_window("dock") else { return };

    loop {
        thread::sleep(Duration::from_millis(16));
        if SHARED.gaming.load(Ordering::Relaxed) {
            last_inside = false;
            last_reveal = false;
            last_shell = false;
            continue;
        }
        unsafe {
            // Dernière fenêtre active hors REZ (pour le clic "réduire si déjà active").
            let fg = GetForegroundWindow();
            let raw = hwnd_val(fg);
            if raw != last_fg_raw {
                last_fg_raw = raw;
                if !fg.is_invalid() && wins::pid_of(fg) != own {
                    SHARED.last_fg.store(raw, Ordering::Relaxed);
                }
                flyout = !fg.is_invalid() && is_shell_flyout(fg);
            }

            // Touche Win / menu Démarrer ouvert → le dock remonte (masquage auto).
            let shell = flyout || win_key_down();
            if shell != last_shell {
                last_shell = shell;
                let _ = app.emit_to("dock", "shell-reveal", shell);
            }

            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_err() {
                continue;
            }
            let inside = in_rects(&SHARED.hit.lock().unwrap(), pt);
            if inside != last_inside {
                last_inside = inside;
                let _ = dock_win.set_ignore_cursor_events(!inside);
                let _ = app.emit_to("dock", "cursor-inside", inside);
            }
            // Zone de réveil (bord d'écran / petit trait) : n'intercepte aucun clic.
            let reveal = in_rects(&SHARED.reveal.lock().unwrap(), pt);
            if reveal != last_reveal {
                last_reveal = reveal;
                let _ = app.emit_to("dock", "reveal-zone", reveal);
            }
        }
    }
}
