//! Nettoyage de l'ancienne installation « REZ » de REZ Dock (v1.0 → v1.2).
//!
//! Jusqu'à la 1.2.0, REZ Dock s'installait sous le nom produit « REZ » — le même que REZ Island.
//! Depuis la 1.2.1 il s'installe à part (« REZ Dock », `com.rez.dock`). Après la mise à jour,
//! l'ancienne copie resterait : double entrée « REZ », et surtout double démarrage auto.
//!
//! Règle de sécurité : on n'agit QUE sur un `rez.exe` prouvé être l'ancien REZ Dock (marqueurs
//! propres au dock dans le binaire). Tout ce qui pointe vers REZ Island est laissé intact.

use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

use crate::win::windows as wins;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const UNINST: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\REZ";
const OLD_NAME: &str = "REZ";

/// Lancé en arrière-plan au démarrage (après la migration des réglages).
pub fn cleanup(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("rez-legacy".into())
        .spawn(move || {
            // Laisse l'ancien processus (qui vient de lancer la mise à jour) se terminer.
            std::thread::sleep(std::time::Duration::from_secs(8));
            run(&app);
        })
        .ok();
}

fn run(app: &tauri::AppHandle) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut olds: Vec<PathBuf> = Vec::new();

    // 1. Démarrage auto « REZ » pointant vers l'ancien dock → supprimé (sinon deux docks au boot).
    if let Ok(run) = hkcu.open_subkey_with_flags(RUN, KEY_READ | KEY_SET_VALUE) {
        if let Ok(cmd) = run.get_value::<String, _>(OLD_NAME) {
            let exe = exe_from_cmd(&cmd);
            if is_old_dock(&exe) {
                let _ = run.delete_value(OLD_NAME);
                olds.push(exe);
            }
        }
    }

    // 2. Entrée « REZ » des applications installées = ancien dock → désinstallation silencieuse.
    //    (supprime rez.exe, l'entrée et ses raccourcis ; jamais les réglages)
    let mut uninstalled: Option<PathBuf> = None;
    if let Ok(key) = hkcu.open_subkey_with_flags(UNINST, KEY_READ) {
        if let Ok(loc) = key.get_value::<String, _>("InstallLocation") {
            let dir = PathBuf::from(loc.trim().trim_matches('"'));
            let exe = dir.join("rez.exe");
            let un = dir.join("uninstall.exe");
            if is_old_dock(&exe) && un.is_file() && !is_running(&exe) {
                if Command::new(&un).arg("/S").spawn().is_ok() {
                    uninstalled = Some(exe.clone());
                }
            }
            if is_old_dock(&exe) {
                olds.push(exe);
            }
        }
    }

    // 3. Copie orpheline (entrée « REZ » reprise par REZ Island) : on retire ses raccourcis « REZ ».
    if let Ok(local) = app.path().local_data_dir() {
        let exe = local.join("REZ").join("rez.exe");
        if is_old_dock(&exe) {
            olds.push(exe);
        }
    }
    let mut links: Vec<PathBuf> = Vec::new();
    if let Ok(d) = app.path().desktop_dir() {
        links.push(d.join("REZ.lnk"));
    }
    if let Ok(d) = app.path().data_dir() {
        links.push(d.join(r"Microsoft\Windows\Start Menu\Programs\REZ.lnk"));
    }
    olds.sort();
    olds.dedup();
    for exe in olds.iter().filter(|e| Some(*e) != uninstalled.as_ref()) {
        for lnk in &links {
            if link_targets(lnk, exe) {
                let _ = std::fs::remove_file(lnk);
            }
        }
    }
}

/// `"C:\…\rez.exe" --autostart` → `C:\…\rez.exe`
fn exe_from_cmd(cmd: &str) -> PathBuf {
    let c = cmd.trim();
    if let Some(rest) = c.strip_prefix('"') {
        return PathBuf::from(rest.split('"').next().unwrap_or(""));
    }
    let lower = c.to_ascii_lowercase();
    match lower.find(".exe") {
        Some(i) => PathBuf::from(&c[..i + 4]),
        None => PathBuf::from(c),
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// `rez.exe` de l'ancien REZ Dock ? (jamais l'exécutable en cours, jamais REZ Island)
fn is_old_dock(exe: &Path) -> bool {
    let named = exe.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case("rez.exe"));
    if !named || !exe.is_file() {
        return false;
    }
    if let Ok(me) = std::env::current_exe() {
        if same_path(&me, exe) {
            return false;
        }
    }
    let Ok(bytes) = std::fs::read(exe) else { return false };
    // Classe Win32 du verre natif + fichier d'état de la barre : propres au dock (v1.0 → v1.2).
    contains(&bytes, b"RezGlassBackdrop") && contains(&bytes, b"taskbar-state.json")
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

/// L'ancien dock tourne encore ? (on retentera au prochain lancement)
fn is_running(exe: &Path) -> bool {
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return true };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        while ok {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            let name = String::from_utf16_lossy(&e.szExeFile[..len]);
            if name.eq_ignore_ascii_case("rez.exe") {
                // Même nom que REZ Island : on compare le chemin complet.
                if wins::exe_of_pid(e.th32ProcessID).is_some_and(|p| same_path(Path::new(&p), exe)) {
                    found = true;
                    break;
                }
            }
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
        found
    }
}

/// Le raccourci `.lnk` pointe-t-il vers `exe` ? (chemin stocké en ANSI et/ou UTF-16 dans le fichier)
fn link_targets(lnk: &Path, exe: &Path) -> bool {
    let Ok(raw) = std::fs::read(lnk) else { return false };
    let lower: Vec<u8> = raw.iter().map(|b| b.to_ascii_lowercase()).collect();
    let path = exe.to_string_lossy().to_lowercase();
    let wide: Vec<u8> = path.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    contains(&lower, path.as_bytes()) || contains(&lower, &wide)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_commands() {
        assert_eq!(exe_from_cmd(r#""C:\A B\rez.exe" --autostart"#), PathBuf::from(r"C:\A B\rez.exe"));
        assert_eq!(exe_from_cmd(r"C:\x\rez.exe --autostart"), PathBuf::from(r"C:\x\rez.exe"));
    }

    #[test]
    fn detects_old_dock_binary_only() {
        let dir = std::env::temp_dir().join(format!("rez-legacy-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("rez.exe");
        std::fs::write(&exe, b"MZ..RezGlassBackdrop..taskbar-state.json..").unwrap();
        assert!(is_old_dock(&exe));
        std::fs::write(&exe, b"MZ..com.rez.island..").unwrap();
        assert!(!is_old_dock(&exe));
        let other = dir.join("island.exe");
        std::fs::write(&other, b"RezGlassBackdrop taskbar-state.json").unwrap();
        assert!(!is_old_dock(&other));
        let lnk = dir.join("REZ.lnk");
        let wide: Vec<u8> = exe.to_string_lossy().to_uppercase().encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        std::fs::write(&lnk, wide).unwrap();
        assert!(link_targets(&lnk, &exe));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
