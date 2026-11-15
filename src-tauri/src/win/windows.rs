//! Énumération des fenêtres de la barre des tâches + actions (activer, réduire, fermer).
//! Lecture seule via EnumWindows / GetWindowText / QueryFullProcessImageName (accès limité).

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::{hwnd, hwnd_val};
use crate::state::SHARED;

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WinInfo {
    pub hwnd: isize,
    pub title: String,
    pub exe: String,
    pub pid: u32,
    pub minimized: bool,
    pub active: bool,
}

unsafe extern "system" fn collect_cb(h: HWND, lp: LPARAM) -> BOOL {
    let list = &mut *(lp.0 as *mut Vec<HWND>);
    list.push(h);
    BOOL(1)
}

pub fn class_name(h: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn title(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn pid_of(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

/// Chemin de l'exécutable (PROCESS_QUERY_LIMITED_INFORMATION : même droit que le Gestionnaire des tâches).
pub fn exe_of_pid(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut size).is_ok();
        let _ = CloseHandle(handle);
        ok.then(|| String::from_utf16_lossy(&buf[..size as usize]))
    }
}

fn is_cloaked(h: HWND) -> bool {
    let mut cloaked: u32 = 0;
    unsafe {
        DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0
    }
}

/// Même heuristique que la barre des tâches Windows.
fn is_taskbar_window(h: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(h).as_bool() || GetWindowTextLengthW(h) == 0 {
            return false;
        }
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        let app_window = ex & WS_EX_APPWINDOW.0 != 0;
        if !app_window {
            if ex & WS_EX_TOOLWINDOW.0 != 0 || ex & WS_EX_NOACTIVATE.0 != 0 {
                return false;
            }
            if let Ok(owner) = GetWindow(h, GW_OWNER) {
                if !owner.is_invalid() {
                    return false;
                }
            }
        }
        if is_cloaked(h) {
            return false;
        }
        !matches!(class_name(h).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
    }
}

unsafe extern "system" fn child_cb(h: HWND, lp: LPARAM) -> BOOL {
    let data = &mut *(lp.0 as *mut (u32, u32));
    let pid = pid_of(h);
    if pid != data.0 {
        data.1 = pid;
        return BOOL(0);
    }
    BOOL(1)
}

/// Les apps UWP sont hébergées par ApplicationFrameHost : on retrouve le vrai processus via la fenêtre enfant.
fn uwp_real_pid(h: HWND, frame_pid: u32) -> Option<u32> {
    let mut data = (frame_pid, 0u32);
    unsafe {
        let _ = EnumChildWindows(Some(h), Some(child_cb), LPARAM(&mut data as *mut _ as isize));
    }
    (data.1 != 0).then_some(data.1)
}

pub fn enumerate(cache: &mut HashMap<u32, String>) -> Vec<WinInfo> {
    let mut handles: Vec<HWND> = Vec::with_capacity(256);
    unsafe {
        let _ = EnumWindows(Some(collect_cb), LPARAM(&mut handles as *mut _ as isize));
    }
    let own_pid = std::process::id();
    let active = SHARED.last_fg.load(Ordering::Relaxed);
    let mut seen = Vec::new();
    let mut out = Vec::new();

    for h in handles {
        if !is_taskbar_window(h) {
            continue;
        }
        let mut pid = pid_of(h);
        let t = title(h);
        // Fenêtres REZ exclues, sauf la fenêtre de réglages (pour pouvoir la retrouver).
        if pid == own_pid && !t.starts_with("REZ Dock — ") {
            continue;
        }
        let mut exe = match cache.get(&pid) {
            Some(e) => e.clone(),
            None => {
                let e = exe_of_pid(pid).unwrap_or_default();
                cache.insert(pid, e.clone());
                e
            }
        };
        if exe.to_lowercase().ends_with("applicationframehost.exe") {
            if let Some(real) = uwp_real_pid(h, pid) {
                pid = real;
                exe = match cache.get(&real) {
                    Some(e) => e.clone(),
                    None => {
                        let e = exe_of_pid(real).unwrap_or(exe);
                        cache.insert(real, e.clone());
                        e
                    }
                };
            }
        }
        seen.push(pid);
        let v = hwnd_val(h);
        out.push(WinInfo {
            hwnd: v,
            title: t,
            exe,
            pid,
            minimized: unsafe { IsIconic(h).as_bool() },
            active: v == active,
        });
    }
    cache.retain(|pid, _| seen.contains(pid));
    out
}

fn alt_tap() {
    let key = |up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_MENU,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[key(false), key(true)], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn focus(v: isize) {
    let h = hwnd(v);
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        if !SetForegroundWindow(h).as_bool() {
            // Déverrouille le verrou de premier plan (technique standard, aucune injection).
            alt_tap();
            let _ = SetForegroundWindow(h);
        }
    }
    SHARED.last_fg.store(v, Ordering::Relaxed);
}

/// Clic sur une icône : restaure / met au premier plan / réduit si déjà active (comportement barre des tâches).
pub fn activate(v: isize) {
    let h = hwnd(v);
    unsafe {
        if IsIconic(h).as_bool() {
            focus(v);
        } else if SHARED.last_fg.load(Ordering::Relaxed) == v {
            let _ = ShowWindow(h, SW_MINIMIZE);
            SHARED.last_fg.store(0, Ordering::Relaxed);
        } else {
            focus(v);
        }
    }
}

pub fn minimize(v: isize) {
    unsafe {
        let _ = ShowWindow(hwnd(v), SW_MINIMIZE);
    }
}

pub fn close(v: isize) {
    unsafe {
        let _ = PostMessageW(Some(hwnd(v)), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

