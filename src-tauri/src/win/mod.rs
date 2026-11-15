//! Couche Win32. API publiques et documentées uniquement :
//! aucune lecture/écriture mémoire d'un autre processus, aucun hook, aucune injection.

pub mod appinfo;
pub mod glass;
pub mod icons;
pub mod shell;
pub mod taskbar;
pub mod tray;
pub mod windows;

use ::windows::Win32::Foundation::{HWND, RECT};
use ::windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use ::windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
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

/// Écran principal : (rect moniteur, rect zone de travail, échelle DPI).
pub fn primary_monitor() -> (RECT, RECT, f64) {
    unsafe {
        let mon = MonitorFromPoint(Default::default(), MONITOR_DEFAULTTOPRIMARY);
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
        (mi.rcMonitor, mi.rcWork, scale)
    }
}
