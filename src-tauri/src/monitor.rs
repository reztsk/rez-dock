//! Threads de fond :
//!  - `tick` (250 ms) : mode gaming (plein écran), liste des fenêtres, maintien des barres natives
//!                     masquées, branchement/débranchement d'écrans.
//!  - `hit`  (16 ms)  : click-through hors des zones de chaque dock, suivi de la fenêtre active,
//!                     zones de réveil du masquage auto, touche Windows (GetAsyncKeyState, sans hook).
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
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowLongW, IsWindowVisible, IsZoomed, GWL_STYLE, WINDOW_STYLE,
    WS_CAPTION,
};

use crate::state::SHARED;
use crate::win::{self, hwnd_val, taskbar, windows as wins};
use crate::{config, dock};

pub fn start(app: AppHandle) {
    supervise("rez-tick", app.clone(), tick_loop);
    supervise("rez-hit", app.clone(), hit_loop);
    supervise("rez-guard", app, guard_loop);
}

/// Boucle immortelle : si elle panique, elle est relancée (et la barre Windows re-masquée,
/// le hook de panic l'ayant restaurée). Hotspot, touche Win et click-through ne s'arrêtent jamais.
fn supervise(name: &'static str, app: AppHandle, body: fn(AppHandle)) {
    thread::Builder::new()
        .name(name.into())
        .spawn(move || loop {
            let a = app.clone();
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || body(a))).is_ok() {
                return; // sortie volontaire (redémarrage en cours)
            }
            thread::sleep(Duration::from_millis(500));
            crate::commands::apply_native(&app);
        })
        .ok();
}

/// Chien de garde : si le moteur WebView2 de REZ Dock meurt (tué depuis le Gestionnaire des
/// tâches, plantage du runtime…), la fenêtre plein écran resterait sans interface : dock invisible,
/// souris captée, barre Windows masquée. On libère tout de suite la souris, on rend la barre
/// Windows, puis REZ Dock redémarre proprement.
fn guard_loop(app: AppHandle) {
    let mut seen = false;
    let mut misses = 0u32;
    loop {
        thread::sleep(Duration::from_millis(1500));
        if SHARED.gaming.load(Ordering::SeqCst) {
            continue;
        }
        if crate::win::tray::own_webview_count() > 0 {
            seen = true;
            misses = 0;
            continue;
        }
        // Pas encore démarré, ou simple trou de snapshot : on attend une 2e confirmation.
        misses += 1;
        if !seen || misses < 2 {
            continue;
        }
        for w in dock::windows(&app) {
            let _ = w.set_ignore_cursor_events(true);
            let _ = w.hide();
        }
        crate::win::glass::set_visible(false);
        SHARED.docks.lock().unwrap_or_else(|e| e.into_inner()).clear();
        taskbar::restore();
        app.restart();
    }
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
        // Bureau / shell, et navigateurs (Chrome, Edge, Brave, Opera, Firefox, apps Electron) :
        // jamais un jeu, même en F11 → le dock et son trait restent actifs par-dessus.
        if matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "XamlExplorerHostIslandWindow"
                | "Windows.UI.Core.CoreWindow" | "MultitaskingViewFrame" | "ForegroundStaging"
                | "Chrome_WidgetWin_1" | "MozillaWindowClass"
        ) {
            return false;
        }
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
                return true;
            }
        }
        // Fenêtre maximisée ou avec barre de titre : la barre Windows étant masquée, sa zone client
        // couvre tout l'écran — ce n'est PAS du plein écran (sinon le dock disparaît en boucle).
        let style = WINDOW_STYLE(GetWindowLongW(fg, GWL_STYLE) as u32);
        if IsZoomed(fg).as_bool() || style.contains(WS_CAPTION) {
            return false;
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
                for w in dock::windows(&a) {
                    let _ = w.set_ignore_cursor_events(true);
                }
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
    let mut last_monitors = monitors_key();

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
                let mut cur = SHARED.windows.lock().unwrap_or_else(|e| e.into_inner());
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
            let key = monitors_key();
            if key != last_monitors {
                last_monitors = key;
                dock::layout(&app);
                let _ = app.emit("display-changed", ());
            }
        }
    }
}

/// Empreinte des écrans (position, taille, DPI) : détecte branchement, résolution, mise à l'échelle.
fn monitors_key() -> Vec<(i32, i32, i32, i32, u32)> {
    win::monitors()
        .iter()
        .map(|m| (m.rect.left, m.rect.top, m.rect.right, m.rect.bottom, (m.scale * 100.0) as u32))
        .collect()
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
    // État précédent par dock : (souris sur le dock, souris dans la zone de réveil).
    let mut last: HashMap<String, (bool, bool)> = HashMap::new();
    let mut changes: Vec<(String, Option<bool>, Option<bool>)> = Vec::new();
    let mut last_shell = false;
    let mut flyout = false;
    let mut last_fg_raw: isize = 0;
    let mut n: u32 = 0;

    loop {
        thread::sleep(Duration::from_millis(16));
        if SHARED.gaming.load(Ordering::Relaxed) {
            last.clear();
            last_shell = false;
            continue;
        }
        n = n.wrapping_add(1);
        // Barre Windows ré-affichée par explorer (touche Win…) : re-masquée en ~100 ms.
        if n % 6 == 0 {
            taskbar::quick_check();
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

            // Touche Win / menu Démarrer ouvert → TOUS les docks remontent (masquage auto).
            let shell = flyout || win_key_down();
            if shell != last_shell {
                last_shell = shell;
                for w in dock::windows(&app) {
                    let _ = w.emit_to(w.label(), "shell-reveal", shell);
                }
            }

            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_err() {
                continue;
            }
            {
                let docks = SHARED.docks.lock().unwrap_or_else(|e| e.into_inner());
                for d in docks.iter() {
                    let inside = in_rects(&d.hit, pt);
                    // Zone de réveil (bord d'écran / petit trait) : n'intercepte aucun clic.
                    let reveal = in_rects(&d.reveal, pt);
                    let prev = last.get(d.label.as_str()).copied().unwrap_or((false, false));
                    if prev != (inside, reveal) {
                        changes.push((
                            d.label.clone(),
                            (prev.0 != inside).then_some(inside),
                            (prev.1 != reveal).then_some(reveal),
                        ));
                        last.insert(d.label.clone(), (inside, reveal));
                    }
                }
            }
            for (label, inside, reveal) in changes.drain(..) {
                if let Some(inside) = inside {
                    if let Some(w) = app.get_webview_window(&label) {
                        let _ = w.set_ignore_cursor_events(!inside);
                    }
                    let _ = app.emit_to(label.as_str(), "cursor-inside", inside);
                }
                if let Some(reveal) = reveal {
                    let _ = app.emit_to(label.as_str(), "reveal-zone", reveal);
                }
            }
        }
    }
}
