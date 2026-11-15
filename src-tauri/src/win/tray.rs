//! "Icônes cachées" (la petite flèche Windows) reconstruites SANS lire la mémoire d'explorer :
//!  1. Windows 11 enregistre chaque app ayant une icône de notification dans
//!     HKCU\Control Panel\NotifyIconSettings (chemin exe + capture PNG de l'icône).
//!  2. On croise avec la liste des processus (Toolhelp : noms uniquement, aucun handle ouvert).
//! + onglet "Arrière-plan" : processus utilisateur sans fenêtre (session courante, hors C:\Windows).

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use windows::core::GUID;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use super::{appinfo, icons, windows::exe_of_pid};
use crate::state::SHARED;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrayApp {
    pub name: String,
    pub exe_path: String,
    pub exe_name: String,
    pub icon: Option<String>,
    pub tooltip: String,
    pub pids: Vec<u32>,
    pub promoted: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BgProcess {
    pub name: String,
    pub path: String,
    pub pids: Vec<u32>,
}

/// Processus critiques jamais terminables depuis REZ.
const PROTECTED: &[&str] = &[
    "system", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe",
    "svchost.exe", "explorer.exe", "dwm.exe", "fontdrvhost.exe", "sihost.exe", "ctfmon.exe",
];

/// Moteur d'affichage des apps (REZ Dock compris) : jamais une « app en arrière-plan » à fermer.
const WEBVIEW: &str = "msedgewebview2.exe";

/// REZ Dock + tous ses descendants (moteur WebView2 : msedgewebview2.exe navigateur, rendu, GPU…).
/// Les terminer tuerait l'interface du dock : jamais listés, jamais terminables.
fn own_tree(procs: &[(u32, u32, String)]) -> HashSet<u32> {
    let mut tree: HashSet<u32> = HashSet::from([std::process::id()]);
    loop {
        let before = tree.len();
        for &(pid, parent, _) in procs {
            // pid != parent : garde-fou contre les pids recyclés / boucles.
            if pid != parent && tree.contains(&parent) {
                tree.insert(pid);
            }
        }
        if tree.len() == before {
            return tree;
        }
    }
}

/// Processus WebView2 de REZ Dock encore vivants (0 = interface du dock morte).
pub fn own_webview_count() -> usize {
    let procs = snapshot();
    let tree = own_tree(&procs);
    procs.iter().filter(|(pid, _, name)| tree.contains(pid) && name == WEBVIEW).count()
}

/// (pid, pid parent, nom exe minuscule) de tous les processus.
fn snapshot() -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                out.push((
                    entry.th32ProcessID,
                    entry.th32ParentProcessID,
                    super::from_wide(&entry.szExeFile).to_lowercase(),
                ));
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// "{GUID}\sous\chemin.exe" -> chemin réel via SHGetKnownFolderPath.
fn resolve_known_folder(path: &str) -> String {
    if !path.starts_with('{') {
        return path.to_string();
    }
    let Some(end) = path.find('}') else { return path.to_string() };
    let guid_str = &path[1..end];
    let rest = &path[end + 1..];
    let Ok(guid) = GUID::try_from(guid_str) else { return path.to_string() };
    unsafe {
        match SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, None) {
            Ok(p) => {
                let base = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as *const _));
                format!("{base}{rest}")
            }
            Err(_) => path.to_string(),
        }
    }
}

fn file_name_lower(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn tray_apps() -> Vec<TrayApp> {
    let procs = snapshot();
    let mine = own_tree(&procs);
    let mut running: HashMap<String, Vec<u32>> = HashMap::new();
    for (pid, _, name) in procs {
        if !mine.contains(&pid) {
            running.entry(name).or_default().push(pid);
        }
    }

    let mut out: Vec<TrayApp> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let own = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(root) = hkcu.open_subkey("Control Panel\\NotifyIconSettings") else {
        return out;
    };
    for sub in root.enum_keys().flatten() {
        let Ok(key) = root.open_subkey(&sub) else { continue };
        let raw: String = key.get_value("ExecutablePath").unwrap_or_default();
        if raw.is_empty() {
            continue;
        }
        let exe_path = resolve_known_folder(&raw);
        let exe_name = file_name_lower(&exe_path);
        if exe_name.is_empty() || exe_path.to_lowercase() == own || exe_name == "explorer.exe" || exe_name == WEBVIEW {
            continue;
        }
        let Some(pids) = running.get(&exe_name) else { continue };
        // Plusieurs icônes enregistrées pour le même exe : une seule entrée.
        if !seen.insert(exe_path.to_lowercase()) {
            continue;
        }
        let tooltip: String = key.get_value("InitialTooltip").unwrap_or_default();
        let promoted: u32 = key.get_value("IsPromoted").unwrap_or(0);
        let snapshot = key.get_raw_value("IconSnapshot").map(|v| v.bytes.into_owned()).unwrap_or_default();
        let icon = if snapshot.starts_with(&[0x89, b'P', b'N', b'G']) {
            Some(icons::data_url(&snapshot))
        } else {
            icons::icon_data_url(&exe_path, 64)
        };
        let name = appinfo::file_description(&exe_path)
            .or_else(|| (!tooltip.is_empty()).then(|| tooltip.lines().next().unwrap_or("").to_string()))
            .unwrap_or_else(|| exe_name.trim_end_matches(".exe").to_string());
        out.push(TrayApp {
            name,
            exe_path,
            exe_name,
            icon,
            tooltip,
            pids: pids.clone(),
            promoted: promoted != 0,
        });
    }
    out.sort_by(|a, b| b.promoted.cmp(&a.promoted).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

pub fn background_processes() -> Vec<BgProcess> {
    let mut session = 0u32;
    unsafe {
        let _ = ProcessIdToSessionId(std::process::id(), &mut session);
    }
    let windowed: HashSet<u32> =
        SHARED.windows.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|w| w.pid).collect();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()).to_lowercase();
    let procs = snapshot();
    let mine = own_tree(&procs);

    let mut groups: HashMap<String, BgProcess> = HashMap::new();
    for (pid, _, name) in procs {
        if pid <= 4
            || mine.contains(&pid)
            || name == WEBVIEW
            || windowed.contains(&pid)
            || PROTECTED.contains(&name.as_str())
        {
            continue;
        }
        let mut s = 0u32;
        if unsafe { ProcessIdToSessionId(pid, &mut s) }.is_err() || s != session {
            continue; // services / autres sessions
        }
        let Some(path) = exe_of_pid(pid) else { continue };
        let lower = path.to_lowercase();
        if lower.starts_with(&windir) {
            continue;
        }
        groups
            .entry(lower)
            .or_insert_with(|| BgProcess {
                name: appinfo::file_description(&path)
                    .unwrap_or_else(|| name.trim_end_matches(".exe").to_string()),
                path: path.clone(),
                pids: Vec::new(),
            })
            .pids
            .push(pid);
    }
    let mut out: Vec<BgProcess> = groups.into_values().collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Fermeture forcée demandée explicitement par l'utilisateur (bouton + confirmation).
pub fn kill(pids: &[u32]) -> usize {
    let procs = snapshot();
    let mine = own_tree(&procs);
    let names: HashMap<u32, String> = procs.into_iter().map(|(pid, _, name)| (pid, name)).collect();
    let mut killed = 0;
    for &pid in pids {
        // Jamais REZ Dock ni son moteur WebView2 (sinon : dock disparu, souris captée, barre masquée).
        if pid <= 4 || mine.contains(&pid) {
            continue;
        }
        if let Some(n) = names.get(&pid) {
            if PROTECTED.contains(&n.as_str()) || n == WEBVIEW {
                continue;
            }
        } else {
            continue;
        }
        unsafe {
            if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                if TerminateProcess(h, 1).is_ok() {
                    killed += 1;
                }
                let _ = CloseHandle(h);
            }
        }
    }
    killed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_tree_covers_all_descendants() {
        let me = std::process::id();
        let procs = vec![
            (me, 1, "rez-dock.exe".to_string()),
            (9001, me, WEBVIEW.to_string()),
            (9002, 9001, WEBVIEW.to_string()),
            (9003, 9002, WEBVIEW.to_string()),
            (9100, 1, "other.exe".to_string()),
            (9101, 9100, WEBVIEW.to_string()),
            (9200, 9200, "self-parent.exe".to_string()),
        ];
        let tree = own_tree(&procs);
        assert!([me, 9001, 9002, 9003].iter().all(|p| tree.contains(p)));
        assert!([9100, 9101, 9200].iter().all(|p| !tree.contains(p)));
    }

    #[test]
    fn background_list_never_contains_self_or_webview() {
        let me = std::process::id();
        let list = background_processes();
        assert!(list.iter().all(|p| !p.pids.contains(&me)));
        assert!(list.iter().all(|p| !p.path.to_lowercase().ends_with(WEBVIEW)));
        assert_eq!(kill(&[me]), 0);
    }
}
