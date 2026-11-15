//! État global partagé entre les threads (moniteur, hit-test, commandes).

use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicIsize};
use std::sync::{LazyLock, Mutex};

use crate::win::windows::WinInfo;

/// Rectangle physique d'une fenêtre dock (pixels écran) + facteur DPI.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub scale: f64,
}

/// Surface envoyée par le frontend (CSS px, relatif à la fenêtre dock).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Surface {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub r: f64,
}

/// Rectangle écran physique (left, top, right, bottom).
pub type ScreenRect = (i32, i32, i32, i32);

/// Un dock (une fenêtre par écran).
#[derive(Clone, Debug)]
pub struct DockSlot {
    pub label: String,
    pub bounds: Bounds,
    /// Zones où le dock capte la souris (le reste est click-through).
    pub hit: Vec<ScreenRect>,
    /// Zones de réveil du masquage auto (bord d'écran + petit trait) : survol = le dock remonte.
    /// Ne captent PAS la souris (les clics passent aux fenêtres dessous).
    pub reveal: Vec<ScreenRect>,
}

pub struct Shared {
    pub config: Mutex<serde_json::Value>,
    /// Docks actifs, écran principal en premier.
    pub docks: Mutex<Vec<DockSlot>>,
    /// Mode gaming actif (jeu plein écran détecté) → dock masqué, polling suspendu.
    pub gaming: AtomicBool,
    /// Dernière fenêtre au premier plan qui n'appartient pas à REZ.
    pub last_fg: AtomicIsize,
    pub windows: Mutex<Vec<WinInfo>>,
}

pub static SHARED: LazyLock<Shared> = LazyLock::new(|| Shared {
    config: Mutex::new(serde_json::Value::Null),
    docks: Mutex::new(Vec::new()),
    gaming: AtomicBool::new(false),
    last_fg: AtomicIsize::new(0),
    windows: Mutex::new(Vec::new()),
});
