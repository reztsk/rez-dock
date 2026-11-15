//! Masquage réversible de la barre des tâches native sur TOUS les écrans + zones de travail.
//!
//! Compatibilité Wallpaper Engine / Lively (zéro scintillement) :
//!  - barres masquées par un simple `ShowWindow(SW_HIDE)` : aucun processus touché, aucun
//!    redémarrage d'explorer, aucune modification du bureau (Progman/WorkerW jamais touchés) ;
//!  - vérification non agressive : on ne re-masque QUE si explorer a ré-affiché une barre ;
//!  - zones de travail posées SANS diffusion globale : ni explorer ni le fond d'écran animé ne
//!    reçoivent de WM_SETTINGCHANGE. Seules les fenêtres maximisées sont notifiées ;
//!  - correction de zone de travail limitée à une fois toutes les 3 s (jamais de ping-pong).
//!
//! L'état d'origine (zones de travail de chaque écran, auto-masquage natif) est sauvegardé sur
//! disque : restauré même après un arrêt brutal.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::{BOOL, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::{hwnd, hwnd_val};
use crate::config::Edge;

const ABM_GETSTATE: u32 = 4;
const ABM_SETSTATE: u32 = 10;
const ABS_AUTOHIDE: u32 = 1;
const ABS_ALWAYSONTOP: u32 = 2;
const FIX_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Saved {
    appbar_state: u32,
    /// Zone de travail d'origine de l'écran principal (format v1.0/v1.1).
    work: [i32; 4],
    /// Zones de travail d'origine de chaque écran géré (v1.2+).
    #[serde(default)]
    works: Vec<[i32; 4]>,
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
/// Zone de travail voulue pour chaque écran géré.
static DESIRED: Mutex<Vec<RECT>> = Mutex::new(Vec::new());
/// Rectangles des écrans gérés (détection branchement/débranchement).
static MANAGED: Mutex<Vec<RECT>> = Mutex::new(Vec::new());
/// Barres masquées (cache pour la vérification rapide).
static TRAYS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static LAST_FIX: Mutex<Option<Instant>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static SECONDARY: AtomicBool = AtomicBool::new(false);

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

fn main_tray() -> Option<HWND> {
    unsafe { FindWindowW(&HSTRING::from("Shell_TrayWnd"), None).ok().filter(|h| !h.is_invalid()) }
}

/// Barre principale + (option) barres des écrans secondaires.
fn find_trays(secondary: bool) -> Vec<HWND> {
    let mut list: Vec<HWND> = main_tray().into_iter().collect();
    if secondary {
        let class = HSTRING::from("Shell_SecondaryTrayWnd");
        let mut prev: Option<HWND> = None;
        while let Ok(h) = unsafe { FindWindowExW(None, prev, &class, None) } {
            if h.is_invalid() || list.len() > 16 {
                break;
            }
            list.push(h);
            prev = Some(h);
        }
    }
    list
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

/// Zone de travail actuelle de l'écran qui contient `rc`.
fn work_of(rc: RECT) -> RECT {
    unsafe {
        let mon = MonitorFromRect(&rc, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(mon, &mut mi);
        mi.rcWork
    }
}

/// Pose la zone de travail de l'écran qui contient `rc` (SPI_SETWORKAREA est par écran).
/// `broadcast = true` (restauration uniquement) : WM_SETTINGCHANGE global → explorer recalcule.
fn set_work_area(mut rc: RECT, broadcast: bool) {
    unsafe {
        let flags = if broadcast { SPIF_SENDCHANGE } else { SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0) };
        let _ = SystemParametersInfoW(SPI_SETWORKAREA, 0, Some(&mut rc as *mut _ as *mut _), flags);
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

fn persist(saved: Option<&Saved>) {
    if let Some(f) = FILE.get() {
        match saved {
            Some(s) => {
                if let Some(dir) = f.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(f, serde_json::to_string(s).unwrap_or_default());
            }
            None => {
                let _ = std::fs::remove_file(f);
            }
        }
    }
}

fn to_arr(r: RECT) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

fn to_rect(a: [i32; 4]) -> RECT {
    RECT { left: a[0], top: a[1], right: a[2], bottom: a[3] }
}

/// Corrige les zones de travail qui ont dérivé (throttlé, sauf `force`).
fn fix_work_areas(force: bool) {
    let wrong: Vec<RECT> = DESIRED.lock().unwrap().iter().copied().filter(|d| work_of(*d) != *d).collect();
    if wrong.is_empty() {
        return;
    }
    {
        let mut last = LAST_FIX.lock().unwrap();
        if !force && last.is_some_and(|t| t.elapsed() < FIX_INTERVAL) {
            return; // Explorer vient de la changer : on n'entre pas dans un ping-pong.
        }
        *last = Some(Instant::now());
    }
    for d in wrong {
        set_work_area(d, false);
    }
    notify_maximized();
}

/// Masque uniquement les barres visibles (aucun appel si déjà masquées).
fn hide_visible(trays: &[HWND]) {
    for &t in trays {
        unsafe {
            if IsWindowVisible(t).as_bool() {
                let _ = ShowWindow(t, SW_HIDE);
            }
        }
    }
}

/// Masque la barre native de chaque écran géré et pose sa zone de travail.
/// `targets` : (rect écran, épaisseur réservée en px physiques) — écran principal en premier.
/// Plusieurs écrans → les barres secondaires (`Shell_SecondaryTrayWnd`) sont masquées aussi.
pub fn apply(hide: bool, edge: Edge, targets: &[(RECT, i32)]) {
    if !hide || targets.is_empty() {
        restore();
        return;
    }
    let monitors: Vec<RECT> = targets.iter().map(|t| t.0).collect();
    // Écrans gérés changés (branchement, option multi-écran) : on repart de l'état d'origine.
    if is_active() && *MANAGED.lock().unwrap() != monitors {
        restore();
    }
    let secondary = monitors.len() > 1;
    SECONDARY.store(secondary, Ordering::Relaxed);

    // 1. Masquage immédiat des barres.
    let trays = find_trays(secondary);
    hide_visible(&trays);
    *TRAYS.lock().unwrap() = trays.iter().map(|&h| hwnd_val(h)).collect();

    // 2. Sauvegarde de l'état d'origine (une seule fois). Si l'auto-masquage natif est actif,
    //    il est coupé pendant que REZ tourne (sinon explorer ré-affiche sa barre sans arrêt).
    {
        let mut saved = SAVED.lock().unwrap();
        if saved.is_none() {
            let s = Saved {
                appbar_state: appbar(ABM_GETSTATE, 0),
                work: to_arr(work_of(monitors[0])),
                works: monitors.iter().map(|m| to_arr(work_of(*m))).collect(),
            };
            persist(Some(&s));
            *saved = Some(s);
        }
        let original = saved.as_ref().map(|s| s.appbar_state).unwrap_or(0);
        if appbar(ABM_GETSTATE, 0) & ABS_AUTOHIDE != 0 {
            appbar(ABM_SETSTATE, original & ABS_ALWAYSONTOP);
        }
    }

    // 2b. Écrans NON gérés (multi-écran désactivé) : leur barre et leur zone de travail
    //     d'origine sont remises en place (ex. reprise après un arrêt brutal en mode multi-écran).
    if !secondary {
        for t in find_trays(true).into_iter().skip(1) {
            unsafe {
                if !IsWindowVisible(t).as_bool() {
                    let _ = ShowWindow(t, SW_SHOWNA);
                }
            }
        }
        let works = SAVED.lock().unwrap().as_ref().map(|s| s.works.clone()).unwrap_or_default();
        for w in works.into_iter().map(to_rect) {
            let managed = monitors.iter().any(|m| w.left >= m.left && w.top >= m.top && w.right <= m.right && w.bottom <= m.bottom);
            if !managed && work_of(w) != w {
                set_work_area(w, false);
            }
        }
    }

    // 3. Zones de travail.
    let desired: Vec<RECT> = targets
        .iter()
        .map(|&(m, reserve)| {
            let r = reserve.max(0);
            match edge {
                Edge::Bottom => RECT { bottom: m.bottom - r, ..m },
                Edge::Top => RECT { top: m.top + r, ..m },
                Edge::Left => RECT { left: m.left + r, ..m },
                Edge::Right => RECT { right: m.right - r, ..m },
                Edge::Free => m,
            }
        })
        .collect();
    *DESIRED.lock().unwrap() = desired;
    *MANAGED.lock().unwrap() = monitors;
    fix_work_areas(true);
    ACTIVE.store(true, Ordering::Relaxed);
}

/// Vérification périodique (1 s) : n'agit QUE si explorer a ré-affiché une barre ou changé une zone.
pub fn enforce() {
    if !is_active() {
        return;
    }
    let trays = find_trays(SECONDARY.load(Ordering::Relaxed));
    hide_visible(&trays);
    *TRAYS.lock().unwrap() = trays.iter().map(|&h| hwnd_val(h)).collect();
    fix_work_areas(false);
}

/// Vérification rapide (~100 ms, handles en cache, zéro énumération) : évite qu'une barre
/// ré-affichée par explorer (touche Win, menu Démarrer) reste visible à côté de REZ.
pub fn quick_check() {
    if !is_active() {
        return;
    }
    let trays: Vec<HWND> = TRAYS.lock().unwrap().iter().map(|&v| hwnd(v)).collect();
    hide_visible(&trays);
}

/// Restauration complète (quitter, mise à jour, panic, désactivation de l'option).
pub fn restore() {
    let saved = SAVED.lock().unwrap().take();
    let was_active = ACTIVE.swap(false, Ordering::Relaxed);
    DESIRED.lock().unwrap().clear();
    MANAGED.lock().unwrap().clear();
    TRAYS.lock().unwrap().clear();
    if !was_active && saved.is_none() {
        return;
    }
    if let Some(s) = saved {
        appbar(ABM_SETSTATE, s.appbar_state);
        let list = if s.works.is_empty() { vec![s.work] } else { s.works };
        let last = list.len().saturating_sub(1);
        for (i, w) in list.into_iter().enumerate() {
            // Une seule diffusion, à la fin : explorer recalcule tout.
            set_work_area(to_rect(w), i == last);
        }
        notify_maximized();
    }
    for t in find_trays(true) {
        unsafe {
            let _ = ShowWindow(t, SW_SHOWNA);
        }
    }
    persist(None);
}
