//! Persistance de la configuration (JSON opaque, schéma possédé par le frontend).
//! Rust ne lit que les quelques champs dont il a besoin (comportement natif).

use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::state::SHARED;

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

pub fn load(app: &AppHandle) {
    let value = file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null);
    *SHARED.config.lock().unwrap() = value;
}

/// Écriture atomique (tmp + rename) pour ne jamais corrompre la config.
pub fn write(app: &AppHandle, value: &Value) -> Result<(), String> {
    let path = file(app).ok_or("dossier de configuration introuvable")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

pub fn get(ptr: &str) -> Option<Value> {
    SHARED.config.lock().unwrap().pointer(ptr).cloned()
}

pub fn bool_at(ptr: &str, default: bool) -> bool {
    get(ptr).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn f64_at(ptr: &str, default: f64) -> f64 {
    get(ptr).and_then(|v| v.as_f64()).unwrap_or(default)
}

// --- Valeurs natives utilisées par le backend (défauts = défauts du frontend) ---

pub fn hide_taskbar() -> bool {
    bool_at("/behavior/hideTaskbar", true)
}
pub fn reserve_space() -> bool {
    bool_at("/behavior/reserveSpace", true)
}
pub fn gaming_mode() -> bool {
    bool_at("/behavior/gamingMode", true)
}
/// Masquage automatique façon macOS (défaut : activé).
pub fn auto_hide() -> bool {
    bool_at("/behavior/autoHide", true)
}
/// Un dock sur chaque écran (défaut : activé).
pub fn all_monitors() -> bool {
    bool_at("/behavior/allMonitors", true)
}
pub fn autostart() -> bool {
    bool_at("/behavior/autostart", false)
}
pub fn blur() -> f64 {
    f64_at("/appearance/blur", 45.0)
}

/// Bord d'ancrage du dock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
    Free,
}

pub fn edge() -> Edge {
    match get("/behavior/position").and_then(|v| v.as_str().map(str::to_owned)).as_deref() {
        Some("top") => Edge::Top,
        Some("left") => Edge::Left,
        Some("right") => Edge::Right,
        Some("free") => Edge::Free,
        _ => Edge::Bottom,
    }
}

/// Épaisseur réservée (CSS px) le long du bord : (icône + padding*2) × échelle + marge + respiration.
pub fn reserved_css_thickness() -> f64 {
    let scale = f64_at("/appearance/scale", 1.0).clamp(0.4, 3.0);
    let icon = f64_at("/appearance/iconSize", 34.0);
    let pad = f64_at("/appearance/padding", 5.0);
    let margin = f64_at("/behavior/bottomMargin", 6.0);
    (icon + pad * 2.0) * scale + margin + 6.0
}

/// Champs natifs : si l'un change, on ré-applique le layout natif.
pub fn native_fingerprint(v: &Value) -> String {
    let keys = [
        "/behavior/hideTaskbar",
        "/behavior/reserveSpace",
        "/behavior/autoHide",
        "/behavior/allMonitors",
        "/behavior/bottomMargin",
        "/behavior/autostart",
        "/behavior/gamingMode",
        "/behavior/position",
        "/appearance/iconSize",
        "/appearance/padding",
        "/appearance/scale",
        "/appearance/blur",
    ];
    keys.iter()
        .map(|k| v.pointer(k).map(|x| x.to_string()).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("|")
}
