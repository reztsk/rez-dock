//! Couche Win32. API publiques et documentées uniquement :
//! aucune lecture/écriture mémoire d'un autre processus, aucun hook, aucune injection.

pub mod appinfo;
pub mod glass;
pub mod icons;
pub mod shell;
pub mod taskbar;
pub mod tray;
pub mod windows;

use ::windows::core::BOOL;
use ::windows::Win32::Foundation::{HWND, LPARAM, RECT};
use ::windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use ::windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use ::windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
use std::ffi::c_void;

#[inline]
pub fn hwnd(v: isize) -> HWND {
    HWND(v as *mut c_void)
}

#[inline]
pub fn hwnd_val(h: HWND) -> isize {
    h.0 as isize
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Un écran : rect moniteur, zone de travail, échelle DPI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monitor {
    pub rect: RECT,
    pub work: RECT,
    pub scale: f64,
    pub primary: bool,
}

fn monitor_info(mon: HMONITOR) -> Monitor {
    unsafe {
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(mon, &mut mi);
        let (mut dx, mut dy) = (96u32, 96u32);
        let scale = if GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
            dx as f64 / 96.0
        } else {
            1.0
        };
        Monitor { rect: mi.rcMonitor, work: mi.rcWork, scale, primary: mi.dwFlags & MONITORINFOF_PRIMARY != 0 }
    }
}

/// Écran principal : (rect moniteur, rect zone de travail, échelle DPI).
pub fn primary_monitor() -> (RECT, RECT, f64) {
    let m = unsafe { monitor_info(MonitorFromPoint(Default::default(), MONITOR_DEFAULTTOPRIMARY)) };
    (m.rect, m.work, m.scale)
}

unsafe extern "system" fn enum_cb(mon: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
    let list = &mut *(lp.0 as *mut Vec<Monitor>);
    list.push(monitor_info(mon));
    BOOL(1)
}

/// Tous les écrans connectés : principal en premier, puis de gauche à droite.
pub fn monitors() -> Vec<Monitor> {
    let mut list: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_cb), LPARAM(&mut list as *mut _ as isize));
    }
    list.sort_by_key(|m| (!m.primary, m.rect.left, m.rect.top));
    if list.is_empty() {
        let (rect, work, scale) = primary_monitor();
        list.push(Monitor { rect, work, scale, primary: true });
    }
    list
}
