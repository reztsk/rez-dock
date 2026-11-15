//! Masquage réversible de la barre des tâches native + zone de travail — version "douce".
//!
//! Compatibilité Wallpaper Engine / multi-écran :
//!  - un seul `ShowWindow(SW_HIDE)` sur la barre PRINCIPALE (les barres des écrans secondaires
//!    ne sont jamais touchées) ; ensuite, simple vérification toutes les secondes ;
//!  - la zone de travail est posée SANS diffusion globale : ni explorer ni le fond d'écran animé
//!    (Wallpaper Engine, Lively…) ne reçoivent de WM_SETTINGCHANGE. Seules les fenêtres
//!    maximisées sont notifiées pour s'ajuster ;
//!  - correction de zone de travail limitée à une fois toutes les 3 s (jamais de ping-pong).
//!
//! L'état d'origine est sauvegardé sur disque : restauré même après un arrêt brutal.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::{BOOL, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::Edge;

const ABM_GETSTATE: u32 = 4;
const ABM_SETSTATE: u32 = 10;
const ABS_AUTOHIDE: u32 = 1;
const ABS_ALWAYSONTOP: u32 = 2;
const FIX_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct Saved {
    appbar_state: u32,
    work: [i32; 4],
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
static DESIRED: Mutex<Option<RECT>> = Mutex::new(None);
static LAST_FIX: Mutex<Option<Instant>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn init(state_file: PathBuf) {
    if let Ok(text) = std::fs::read_to_string(&state_file) {
        if let Ok(saved) = serde_json::from_str::<Saved>(&text) {
            // Arrêt brutal précédent : on reprend l'état d'origine réel.
            *SAVED.lock().unwrap() = Some(saved);
        }
    }
    let _ = FILE.set(state_file);
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Barre principale uniquement (écran principal, celui du dock).
fn main_tray() -> Option<HWND> {
    unsafe { FindWindowW(&HSTRING::from("Shell_TrayWnd"), None).ok().filter(|h| !h.is_invalid()) }
}

fn appbar(msg: u32, state: u32) -> u32 {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: main_tray().unwrap_or_default(),
        lParam: LPARAM(state as isize),
        ..Default::default()
    };
    unsafe { SHAppBarMessage(msg, &mut data) as u32 }
}

fn get_work_area() -> RECT {
    let mut rc = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut rc as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    rc
}

/// Pose la zone de travail.
/// `broadcast = true` (restauration uniquement) : WM_SETTINGCHANGE global → explorer recalcule.
/// `broadcast = false` (REZ actif) : aucune diffusion ; seules les fenêtres maximisées sont prévenues.
fn set_work_area(mut rc: RECT, broadcast: bool) {
    unsafe {
        let flags = if broadcast { SPIF_SENDCHANGE } else { SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0) };
        let _ = SystemParametersInfoW(SPI_SETWORKAREA, 0, Some(&mut rc as *mut _ as *mut _), flags);
    }
    if !broadcast {
        notify_maximized();
    }
}

unsafe extern "system" fn notify_cb(h: HWND, lp: LPARAM) -> BOOL {
    let explorer = lp.0 as u32;
    // Fenêtres maximisées seulement : un fond d'écran animé n'est jamais "maximisé" → jamais dérangé.
    if IsWindowVisible(h).as_bool() && IsZoomed(h).as_bool() && super::windows::pid_of(h) != explorer {
        let _ = SendNotifyMessageW(h, WM_SETTINGCHANGE, WPARAM(SPI_SETWORKAREA.0 as usize), LPARAM(0));
    }
    BOOL(1)
}

/// WM_SETTINGCHANGE(SPI_SETWORKAREA) non bloquant aux fenêtres maximisées (hors explorer).
fn notify_maximized() {
    let explorer = main_tray().map(super::windows::pid_of).unwrap_or(0);
    unsafe {
        let _ = EnumWindows(Some(notify_cb), LPARAM(explorer as isize));
    }
}

fn persist(saved: Option<Saved>) {
    if let Some(f) = FILE.get() {
        match saved {
            Some(s) => {
                if let Some(dir) = f.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(f, serde_json::to_string(&s).unwrap_or_default());
            }
            None => {
                let _ = std::fs::remove_file(f);
            }
        }
    }
}

fn fix_work_area(desired: RECT, force: bool) {
    if get_work_area() == desired {
        return;
    }
    let mut last = LAST_FIX.lock().unwrap();
    if !force && last.is_some_and(|t| t.elapsed() < FIX_INTERVAL) {
        return; // Explorer vient de la changer : on n'entre pas dans un ping-pong.
    }
    *last = Some(Instant::now());
    set_work_area(desired, false);
}

/// Masque la barre native (instantané) et réserve `reserve` px physiques le long de `edge`
/// sur l'écran principal (0, masquage auto ou mode libre = aucune réservation).
pub fn apply(hide: bool, edge: Edge, reserve: i32, monitor: RECT) {
    if !hide {
        restore();
        return;
    }
    // 1. Masquage immédiat de Shell_TrayWnd (écran principal seulement).
    if let Some(w) = main_tray() {
        unsafe {
            let _ = ShowWindow(w, SW_HIDE);
        }
    }
    // 2. Sauvegarde de l'état d'origine (une seule fois). Si l'auto-masquage natif est actif,
    //    il est coupé pendant que REZ tourne (sinon explorer ré-affiche sa barre sans arrêt).
    {
        let mut saved = SAVED.lock().unwrap();
        if saved.is_none() {
            let wa = get_work_area();
            let s = Saved {
                appbar_state: appbar(ABM_GETSTATE, 0),
                work: [wa.left, wa.top, wa.right, wa.bottom],
            };
            *saved = Some(s);
            persist(Some(s));
        }
        let original = saved.unwrap().appbar_state;
        if appbar(ABM_GETSTATE, 0) & ABS_AUTOHIDE != 0 {
            appbar(ABM_SETSTATE, original & ABS_ALWAYSONTOP);
        }
    }
    // 3. Zone de travail.
    let r = reserve.max(0);
    let desired = match edge {
        Edge::Bottom => RECT { bottom: monitor.bottom - r, ..monitor },
        Edge::Top => RECT { top: monitor.top + r, ..monitor },
        Edge::Left => RECT { left: monitor.left + r, ..monitor },
        Edge::Right => RECT { right: monitor.right - r, ..monitor },
        Edge::Free => monitor,
    };
    *DESIRED.lock().unwrap() = Some(desired);
    fix_work_area(desired, true);
    ACTIVE.store(true, Ordering::Relaxed);
}

/// Vérification périodique (1 s) : n'agit QUE si explorer a ré-affiché sa barre ou changé la zone.
pub fn enforce() {
    if !is_active() {
        return;
    }
    if let Some(w) = main_tray() {
        unsafe {
            if IsWindowVisible(w).as_bool() {
                let _ = ShowWindow(w, SW_HIDE);
            }
        }
    }
    if let Some(d) = *DESIRED.lock().unwrap() {
        fix_work_area(d, false);
    }
}

/// Restauration complète (quitter, mise à jour, panic, désactivation de l'option).
pub fn restore() {
    let saved = SAVED.lock().unwrap().take();
    let was_active = ACTIVE.swap(false, Ordering::Relaxed);
    *DESIRED.lock().unwrap() = None;
    if !was_active && saved.is_none() {
        return;
    }
    if let Some(s) = saved {
        appbar(ABM_SETSTATE, s.appbar_state);
        set_work_area(
            RECT {
                left: s.work[0],
                top: s.work[1],
                right: s.work[2],
                bottom: s.work[3],
            },
            true,
        );
    }
    if let Some(w) = main_tray() {
        unsafe {
            let _ = ShowWindow(w, SW_SHOWNA);
        }
    }
    persist(None);
}
