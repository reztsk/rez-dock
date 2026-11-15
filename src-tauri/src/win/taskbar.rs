//! Masquage réversible de la barre des tâches native + réservation de la zone de travail.
//! L'état d'origine est sauvegardé sur disque : restauré même après un arrêt brutal (au lancement suivant).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::{BOOL, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::Edge;

const ABM_GETSTATE: u32 = 4;
const ABM_SETSTATE: u32 = 10;
const ABS_AUTOHIDE: u32 = 1;
const ABS_ALWAYSONTOP: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct Saved {
    appbar_state: u32,
    work: [i32; 4],
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
static DESIRED: Mutex<Option<RECT>> = Mutex::new(None);
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

unsafe extern "system" fn collect_cb(h: HWND, lp: LPARAM) -> BOOL {
    let list = &mut *(lp.0 as *mut Vec<HWND>);
    if super::windows::class_name(h) == "Shell_SecondaryTrayWnd" {
        list.push(h);
    }
    BOOL(1)
}

fn tray_windows() -> Vec<HWND> {
    let mut list = Vec::new();
    unsafe {
        if let Ok(main) = FindWindowW(&HSTRING::from("Shell_TrayWnd"), None) {
            if !main.is_invalid() {
                list.push(main);
            }
        }
        let _ = EnumWindows(Some(collect_cb), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

fn appbar(msg: u32, state: u32) -> u32 {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: tray_windows().first().copied().unwrap_or_default(),
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
/// `broadcast = true` (restauration) : WM_SETTINGCHANGE global → explorer recalcule sa propre zone.
/// `broadcast = false` (REZ actif) : explorer n'est PAS notifié (sinon il ré-impose ses 48 px
/// en ~200 ms et les deux se battent) ; seules les autres fenêtres le sont, pour que les
/// fenêtres maximisées s'ajustent au dock.
fn set_work_area(mut rc: RECT, broadcast: bool) {
    unsafe {
        let flags = if broadcast { SPIF_SENDCHANGE } else { SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0) };
        let _ = SystemParametersInfoW(SPI_SETWORKAREA, 0, Some(&mut rc as *mut _ as *mut _), flags);
    }
    if !broadcast {
        notify_apps();
    }
}

unsafe extern "system" fn notify_cb(h: HWND, lp: LPARAM) -> BOOL {
    let explorer = lp.0 as u32;
    if IsWindowVisible(h).as_bool() && super::windows::pid_of(h) != explorer {
        let _ = SendNotifyMessageW(h, WM_SETTINGCHANGE, WPARAM(SPI_SETWORKAREA.0 as usize), LPARAM(0));
    }
    BOOL(1)
}

/// WM_SETTINGCHANGE(SPI_SETWORKAREA) non bloquant à toutes les fenêtres visibles sauf explorer.
fn notify_apps() {
    let explorer = tray_windows().first().map(|&h| super::windows::pid_of(h)).unwrap_or(0);
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

/// Masque la barre native (instantané) et réserve `reserve` px physiques le long de `edge`
/// sur l'écran principal (0 ou mode libre = aucune réservation).
pub fn apply(hide: bool, edge: Edge, reserve: i32, monitor: RECT) {
    if !hide {
        restore();
        return;
    }
    // 1. Masquage immédiat de Shell_TrayWnd (+ barres secondaires multi-écrans).
    for w in tray_windows() {
        unsafe {
            let _ = ShowWindow(w, SW_HIDE);
        }
    }
    // 2. Sauvegarde de l'état d'origine (une seule fois). L'auto-masquage natif est COUPÉ pendant
    //    que REZ tourne : avec lui, explorer ré-affiche Shell_TrayWnd en permanence.
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
        if original & ABS_AUTOHIDE != 0 || appbar(ABM_GETSTATE, 0) & ABS_AUTOHIDE != 0 {
            appbar(ABM_SETSTATE, original & ABS_ALWAYSONTOP);
        }
    }
    // 3. Zone de travail : les fenêtres maximisées s'arrêtent au bord du dock.
    let r = reserve.max(0);
    let desired = match edge {
        Edge::Bottom => RECT { bottom: monitor.bottom - r, ..monitor },
        Edge::Top => RECT { top: monitor.top + r, ..monitor },
        Edge::Left => RECT { left: monitor.left + r, ..monitor },
        Edge::Right => RECT { right: monitor.right - r, ..monitor },
        Edge::Free => monitor,
    };
    *DESIRED.lock().unwrap() = Some(desired);
    if get_work_area() != desired {
        set_work_area(desired, false);
    }
    ACTIVE.store(true, Ordering::Relaxed);
}

/// Appelé périodiquement : explorer réaffiche parfois sa barre ou recalcule la zone de travail.
pub fn enforce() {
    if !is_active() {
        return;
    }
    for w in tray_windows() {
        unsafe {
            if IsWindowVisible(w).as_bool() {
                let _ = ShowWindow(w, SW_HIDE);
            }
        }
    }
    if let Some(d) = *DESIRED.lock().unwrap() {
        if get_work_area() != d {
            set_work_area(d, false);
        }
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
    for w in tray_windows() {
        unsafe {
            let _ = ShowWindow(w, SW_SHOWNA);
        }
    }
    persist(None);
}
