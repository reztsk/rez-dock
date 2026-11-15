//! Lancement d'applications et raccourcis système (Démarrer, Recherche, Bureau…).

use std::path::Path;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn shell_open(file: &str, args: Option<&str>, dir: Option<&str>) -> Result<(), String> {
    let file_w = HSTRING::from(file);
    let args_w = args.filter(|a| !a.trim().is_empty()).map(HSTRING::from);
    let dir_w = dir.map(HSTRING::from);
    let r = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &file_w,
            args_w.as_ref().map(|a| PCWSTR(a.as_ptr())).unwrap_or(PCWSTR::null()),
            dir_w.as_ref().map(|d| PCWSTR(d.as_ptr())).unwrap_or(PCWSTR::null()),
            SW_SHOWNORMAL,
        )
    };
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Impossible d'ouvrir {file} (code {})", r.0 as isize))
    }
}

pub fn launch(path: &str, args: Option<&str>) -> Result<(), String> {
    let dir = Path::new(path)
        .parent()
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().to_string());
    shell_open(path, args, dir.as_deref())
}

pub fn open_location(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        shell_open("explorer.exe", Some(&format!("/select,\"{path}\"")), None)
    } else {
        Err("Fichier introuvable".into())
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Raccourci clavier système (déclenché uniquement par un clic utilisateur sur un module).
fn chord(keys: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| key(k, false)).collect();
    inputs.extend(keys.iter().rev().map(|&k| key(k, true)));
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn action(name: &str) -> Result<(), String> {
    const VK_S: VIRTUAL_KEY = VIRTUAL_KEY(0x53);
    const VK_D: VIRTUAL_KEY = VIRTUAL_KEY(0x44);
    const VK_N: VIRTUAL_KEY = VIRTUAL_KEY(0x4E);
    const VK_A: VIRTUAL_KEY = VIRTUAL_KEY(0x41);
    match name {
        "start" => chord(&[VK_LWIN]),
        "search" => chord(&[VK_LWIN, VK_S]),
        "desktop" => chord(&[VK_LWIN, VK_D]),
        "notifications" => chord(&[VK_LWIN, VK_N]),
        "quick-settings" => chord(&[VK_LWIN, VK_A]),
        "taskview" => chord(&[VK_LWIN, VK_TAB]),
        "trash" => return shell_open("shell:RecycleBinFolder", None, None),
        "taskmgr" => return shell_open("taskmgr.exe", None, None),
        "tray-settings" => return shell_open("ms-settings:taskbar", None, None),
        "explorer" => return shell_open("explorer.exe", None, None),
        other => return Err(format!("Action inconnue : {other}")),
    }
    Ok(())
}
