//! Placement de la fenêtre dock.
//!
//! La fenêtre couvre tout l'écran principal (moins 1 px en haut pour ne jamais être prise pour une
//! app plein écran par Windows) et reste transparente + click-through hors des surfaces du dock.
//! Le bord choisi (bas/haut/gauche/droite/libre) est donc du pur CSS, sans redimensionnement natif.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::config::{self, Edge};
use crate::state::{Bounds, Surface, SHARED};
use crate::win::{self, glass, taskbar};

/// Marges (CSS px) occupées par la barre Windows quand elle reste visible.
#[derive(Serialize, Clone, Copy, Default, Debug, PartialEq)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

pub fn compute_bounds() -> Bounds {
    let (mon, _, scale) = win::primary_monitor();
    Bounds {
        x: mon.left,
        y: mon.top + 1,
        w: mon.right - mon.left,
        h: mon.bottom - mon.top - 1,
        scale,
    }
}

/// Zones déjà prises par la barre native visible (rien quand REZ la masque).
pub fn insets() -> Insets {
    if taskbar::is_active() {
        return Insets::default();
    }
    let (mon, work, scale) = win::primary_monitor();
    let s = if scale > 0.0 { scale } else { 1.0 };
    Insets {
        top: ((work.top - mon.top).max(0) as f64 / s).round(),
        right: ((mon.right - work.right).max(0) as f64 / s).round(),
        bottom: ((mon.bottom - work.bottom).max(0) as f64 / s).round(),
        left: ((work.left - mon.left).max(0) as f64 / s).round(),
    }
}

/// (Ré)applique tout le layout natif : barre Windows, zone de travail, fenêtre dock, verre.
pub fn layout(app: &AppHandle) {
    let (mon, _, scale) = win::primary_monitor();
    let edge = config::edge();
    // Masquage auto = comme macOS : les fenêtres maximisées prennent tout l'écran.
    let reserve = if config::reserve_space() && !config::auto_hide() && edge != Edge::Free {
        (config::reserved_css_thickness() * scale).round() as i32
    } else {
        0
    };
    taskbar::apply(config::hide_taskbar(), edge, reserve, mon);

    let b = compute_bounds();
    *SHARED.bounds.lock().unwrap() = b;
    glass::set_scale(b.scale);
    if let Some(w) = app.get_webview_window("dock") {
        let _ = w.set_position(PhysicalPosition::new(b.x, b.y));
        let _ = w.set_size(PhysicalSize::new(b.w as u32, b.h as u32));
        let _ = w.set_always_on_top(true);
    }
    let _ = app.emit("layout-changed", insets());
}

/// Surfaces envoyées par le frontend (CSS px relatifs à la fenêtre).
pub fn set_surfaces(glass_rects: &[Surface], hit_rects: &[Surface], reveal_rects: &[Surface]) {
    let b = *SHARED.bounds.lock().unwrap();
    let s = if b.scale > 0.0 { b.scale } else { 1.0 };
    let px = |v: f64| (v * s).round() as i32;

    let rects: Vec<glass::GlassRect> = glass_rects
        .iter()
        .map(|r| (b.x + px(r.x), b.y + px(r.y), px(r.w), px(r.h), px(r.r)))
        .collect();
    glass::set_surfaces(SHARED.dock_hwnd.load(std::sync::atomic::Ordering::Relaxed), &rects);

    let to_screen = |list: &[Surface]| -> Vec<crate::state::ScreenRect> {
        list.iter()
            .map(|r| (b.x + px(r.x), b.y + px(r.y), b.x + px(r.x + r.w), b.y + px(r.y + r.h)))
            .collect()
    };
    *SHARED.hit.lock().unwrap() = to_screen(hit_rects);
    *SHARED.reveal.lock().unwrap() = to_screen(reveal_rects);
}

/// Affiche/masque dock + verre sans jamais voler le focus (crucial en jeu).
/// Passe par Tauri (et non ShowWindow brut) : sinon tao ré-applique son propre état "masqué"
/// au prochain changement de style. Avec `focus: false`, tao utilise SW_SHOWNOACTIVATE.
pub fn set_visible(app: &AppHandle, visible: bool) {
    if let Some(w) = app.get_webview_window("dock") {
        let _ = if visible { w.show() } else { w.hide() };
        if visible {
            let _ = w.set_always_on_top(true);
        }
    }
    glass::set_visible(visible);
}
