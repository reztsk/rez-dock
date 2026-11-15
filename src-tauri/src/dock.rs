//! Placement des fenêtres dock — UN dock par écran connecté.
//!
//! Chaque fenêtre dock couvre tout son écran (moins 1 px en haut pour ne jamais être prise pour
//! une app plein écran par Windows) et reste transparente + click-through hors des surfaces du dock.
//! Le bord choisi (bas/haut/gauche/droite/libre) est donc du pur CSS, sans redimensionnement natif.
//!
//! Labels : `dock` = écran principal (déclaré dans tauri.conf.json), `dock-1`, `dock-2`… = écrans
//! secondaires, créés/détruits dynamiquement (branchement, option « Dock sur tous les écrans »).

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::config::{self, Edge};
use crate::state::{Bounds, DockSlot, ScreenRect, Surface, SHARED};
use crate::win::{self, glass, taskbar, Monitor};

pub const PRIMARY: &str = "dock";

/// Marges (CSS px) occupées par la barre Windows quand elle reste visible.
#[derive(Serialize, Clone, Copy, Default, Debug, PartialEq)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

static CREATING: AtomicBool = AtomicBool::new(false);

pub fn label_for(index: usize) -> String {
    if index == 0 {
        PRIMARY.to_owned()
    } else {
        format!("dock-{index}")
    }
}

pub fn is_dock(label: &str) -> bool {
    label == PRIMARY || label.starts_with("dock-")
}

fn index_of(label: &str) -> Option<usize> {
    if label == PRIMARY {
        Some(0)
    } else {
        label.strip_prefix("dock-").and_then(|n| n.parse().ok())
    }
}

/// Écrans qui reçoivent un dock : tous, ou seulement le principal.
pub fn used_monitors() -> Vec<Monitor> {
    let mut all = win::monitors();
    if !config::all_monitors() {
        all.truncate(1);
    }
    all
}

fn bounds_of(m: &Monitor) -> Bounds {
    Bounds {
        x: m.rect.left,
        y: m.rect.top + 1,
        w: m.rect.right - m.rect.left,
        h: m.rect.bottom - m.rect.top - 1,
        scale: m.scale,
    }
}

/// Zones déjà prises par la barre native visible (rien quand REZ la masque).
fn insets_of(m: &Monitor) -> Insets {
    if taskbar::is_active() {
        return Insets::default();
    }
    let s = if m.scale > 0.0 { m.scale } else { 1.0 };
    let (mon, work) = (m.rect, m.work);
    Insets {
        top: ((work.top - mon.top).max(0) as f64 / s).round(),
        right: ((mon.right - work.right).max(0) as f64 / s).round(),
        bottom: ((mon.bottom - work.bottom).max(0) as f64 / s).round(),
        left: ((work.left - mon.left).max(0) as f64 / s).round(),
    }
}

/// Insets du dock `label` (calcul frais : valable même pendant la création de la fenêtre).
pub fn insets_for(label: &str) -> Insets {
    let used = used_monitors();
    index_of(label).and_then(|i| used.get(i)).map(insets_of).unwrap_or_default()
}

/// Fenêtre dock d'un écran secondaire (mêmes propriétés que `dock` dans tauri.conf.json).
fn build_dock_window(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("REZ Dock")
        .inner_size(400.0, 200.0)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        .visible(false)
        .focused(false)
        .build()?;
    let _ = w.set_ignore_cursor_events(true);
    Ok(w)
}

/// (Ré)applique tout le layout natif : barres Windows, zones de travail, fenêtres dock, verre.
pub fn layout(app: &AppHandle) {
    let used = used_monitors();
    let edge = config::edge();
    // Masquage auto = comme macOS : les fenêtres maximisées prennent tout l'écran.
    let reserve_css = if config::reserve_space() && !config::auto_hide() && edge != Edge::Free {
        config::reserved_css_thickness()
    } else {
        0.0
    };
    let targets: Vec<_> = used.iter().map(|m| (m.rect, (reserve_css * m.scale).round() as i32)).collect();
    taskbar::apply(config::hide_taskbar(), edge, &targets);

    let previous = SHARED.docks.lock().unwrap().clone();
    let mut slots: Vec<DockSlot> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for (i, m) in used.iter().enumerate() {
        let label = label_for(i);
        let Some(w) = app.get_webview_window(&label) else {
            missing.push(label);
            continue;
        };
        let b = bounds_of(m);
        let _ = w.set_position(PhysicalPosition::new(b.x, b.y));
        let _ = w.set_size(PhysicalSize::new(b.w as u32, b.h as u32));
        let _ = w.set_always_on_top(true);
        let hwnd = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        glass::ensure(&label, hwnd, b.scale);
        let (hit, reveal) = previous
            .iter()
            .find(|s| s.label == label && s.bounds == b)
            .map(|s| (s.hit.clone(), s.reveal.clone()))
            .unwrap_or_default();
        slots.push(DockSlot { label, bounds: b, hit, reveal });
    }

    // Docks en trop (écran débranché, option multi-écran désactivée).
    for (label, w) in app.webview_windows() {
        if label.starts_with("dock-") && !slots.iter().any(|s| s.label == label) && !missing.contains(&label) {
            glass::remove(&label);
            let _ = w.destroy();
        }
    }

    *SHARED.docks.lock().unwrap() = slots.clone();
    for s in &slots {
        let _ = app.emit_to(s.label.as_str(), "layout-changed", insets_for(&s.label));
    }

    // Nouveaux écrans : création hors du thread appelant (jamais de blocage WebView2), puis re-layout.
    if !missing.is_empty() && !CREATING.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        std::thread::spawn(move || {
            for label in &missing {
                let _ = build_dock_window(&app, label);
            }
            CREATING.store(false, Ordering::SeqCst);
            layout(&app);
            if !SHARED.gaming.load(Ordering::SeqCst) {
                set_visible(&app, true);
            }
        });
    }
}

/// Surfaces envoyées par le frontend du dock `label` (CSS px relatifs à sa fenêtre).
pub fn set_surfaces(label: &str, glass_rects: &[Surface], hit_rects: &[Surface], reveal_rects: &[Surface]) {
    let mut docks = SHARED.docks.lock().unwrap();
    let Some(slot) = docks.iter_mut().find(|s| s.label == label) else { return };
    let b = slot.bounds;
    let s = if b.scale > 0.0 { b.scale } else { 1.0 };
    let px = |v: f64| (v * s).round() as i32;

    let rects: Vec<glass::GlassRect> = glass_rects
        .iter()
        .map(|r| (b.x + px(r.x), b.y + px(r.y), px(r.w), px(r.h), px(r.r)))
        .collect();
    glass::set_surfaces(label, &rects);

    let to_screen = |list: &[Surface]| -> Vec<ScreenRect> {
        list.iter()
            .map(|r| (b.x + px(r.x), b.y + px(r.y), b.x + px(r.x + r.w), b.y + px(r.y + r.h)))
            .collect()
    };
    slot.hit = to_screen(hit_rects);
    slot.reveal = to_screen(reveal_rects);
}

/// Toutes les fenêtres dock (un par écran).
pub fn windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows().into_iter().filter(|(l, _)| is_dock(l)).map(|(_, w)| w).collect()
}

/// Affiche/masque docks + verre sans jamais voler le focus (crucial en jeu).
/// Passe par Tauri (et non ShowWindow brut) : sinon tao ré-applique son propre état "masqué"
/// au prochain changement de style. Avec `focus: false`, tao utilise SW_SHOWNOACTIVATE.
pub fn set_visible(app: &AppHandle, visible: bool) {
    for w in windows(app) {
        let _ = if visible { w.show() } else { w.hide() };
        if visible {
            let _ = w.set_always_on_top(true);
        }
    }
    glass::set_visible(visible);
}
