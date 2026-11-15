//! Métadonnées d'applications : nom lisible (FileDescription), résolution des raccourcis .lnk,
//! clé de correspondance fenêtre <-> élément épinglé, épingles par défaut.

use serde::Serialize;
use std::path::Path;
use windows::core::{Interface, HSTRING};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READ};
use windows::Win32::UI::Shell::{IShellItem, IShellLinkW, SHCreateItemFromParsingName, ShellLink};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub path: String,
    pub args: Option<String>,
    pub name: String,
    pub target: String,
    /// Nom de fichier exécutable en minuscules, utilisé pour associer les fenêtres ouvertes.
    pub match_key: String,
    pub icon: Option<String>,
}

fn file_name_lower(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn stem(p: &str) -> String {
    Path::new(p)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string())
}

/// FileDescription de la ressource VERSIONINFO (ex : "Google Chrome").
pub fn file_description(path: &str) -> Option<String> {
    unsafe {
        let wpath = HSTRING::from(path);
        let size = GetFileVersionInfoSizeW(&wpath, None);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        GetFileVersionInfoW(&wpath, None, size, buf.as_mut_ptr() as *mut _).ok()?;

        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let mut langs: Vec<(u16, u16)> = Vec::new();
        if VerQueryValueW(buf.as_ptr() as *const _, &HSTRING::from("\\VarFileInfo\\Translation"), &mut ptr, &mut len).as_bool()
            && len >= 4
        {
            let pairs = std::slice::from_raw_parts(ptr as *const u16, (len / 2) as usize);
            for c in pairs.chunks_exact(2) {
                langs.push((c[0], c[1]));
            }
        }
        langs.extend([(0x0409, 0x04b0), (0x040c, 0x04b0), (0x0409, 0x04e4)]);
        for (lang, cp) in langs {
            let q = HSTRING::from(format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\FileDescription"));
            if VerQueryValueW(buf.as_ptr() as *const _, &q, &mut ptr, &mut len).as_bool() && len > 1 {
                let s = std::slice::from_raw_parts(ptr as *const u16, len as usize);
                let text = super::from_wide(s).trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        None
    }
}

/// Résout un .lnk -> (cible, arguments).
pub fn resolve_shortcut(path: &str) -> Option<(String, String)> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let file: IPersistFile = link.cast().ok()?;
        file.Load(&HSTRING::from(path), STGM_READ).ok()?;
        let mut target = [0u16; 1024];
        link.GetPath(&mut target, std::ptr::null_mut(), 0).ok()?;
        let mut args = [0u16; 2048];
        let _ = link.GetArguments(&mut args);
        Some((super::from_wide(&target), super::from_wide(&args)))
    }
}

/// Clé de correspondance : gère les lanceurs Squirrel (Discord, Teams classic…) "Update.exe --processStart X.exe".
fn match_key_for(target: &str, args: &str) -> String {
    let file = file_name_lower(target);
    if file == "update.exe" {
        let mut it = args.split_whitespace();
        while let Some(tok) = it.next() {
            if tok.eq_ignore_ascii_case("--processStart") {
                if let Some(exe) = it.next() {
                    return exe.trim_matches('"').to_lowercase();
                }
            }
        }
    }
    file
}

pub fn app_info(path: &str) -> AppInfo {
    let lower = path.to_lowercase();
    if lower.ends_with(".lnk") {
        let (target, args) = resolve_shortcut(path).unwrap_or_default();
        let target = if target.is_empty() { path.to_string() } else { target };
        return AppInfo {
            path: path.to_string(),
            args: None,
            name: stem(path),
            match_key: match_key_for(&target, &args),
            target,
            icon: None,
        };
    }
    if lower.ends_with(".url") || lower.starts_with("shell:") || lower.contains("://") || lower.ends_with(':') {
        return AppInfo {
            path: path.to_string(),
            args: None,
            name: stem(path),
            target: path.to_string(),
            match_key: String::new(),
            icon: None,
        };
    }
    AppInfo {
        path: path.to_string(),
        args: None,
        name: file_description(path).unwrap_or_else(|| stem(path)),
        target: path.to_string(),
        match_key: file_name_lower(path),
        icon: None,
    }
}

fn shell_item_exists(parse_name: &str) -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(parse_name), None).is_ok()
    }
}

/// Applications courantes détectées sur la machine (premier lancement).
pub fn default_pins() -> Vec<AppInfo> {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let pf = env("ProgramFiles");
    let pf86 = env("ProgramFiles(x86)");
    let local = env("LOCALAPPDATA");
    let roaming = env("APPDATA");
    let windir = {
        let w = env("WINDIR");
        if w.is_empty() { "C:\\Windows".to_string() } else { w }
    };

    let mut out = Vec::new();
    let mut push_exe = |p: String, name: &str| {
        if Path::new(&p).exists() {
            let mut info = app_info(&p);
            info.name = name.to_string();
            out.push(info);
        }
    };
    push_exe(format!("{windir}\\explorer.exe"), "Explorateur");
    for p in [
        format!("{pf}\\Google\\Chrome\\Application\\chrome.exe"),
        format!("{local}\\Google\\Chrome\\Application\\chrome.exe"),
    ] {
        if Path::new(&p).exists() {
            push_exe(p, "Chrome");
            break;
        }
    }
    push_exe(format!("{pf86}\\Microsoft\\Edge\\Application\\msedge.exe"), "Edge");
    push_exe(format!("{pf}\\Mozilla Firefox\\firefox.exe"), "Firefox");
    push_exe(format!("{local}\\Programs\\Microsoft VS Code\\Code.exe"), "VS Code");
    push_exe(format!("{roaming}\\Spotify\\Spotify.exe"), "Spotify");
    push_exe(format!("{pf86}\\Steam\\steam.exe"), "Steam");

    let discord = format!("{local}\\Discord\\Update.exe");
    if Path::new(&discord).exists() {
        let ico = format!("{local}\\Discord\\app.ico");
        out.push(AppInfo {
            path: discord.clone(),
            args: Some("--processStart Discord.exe".into()),
            name: "Discord".into(),
            target: discord,
            match_key: "discord.exe".into(),
            icon: Path::new(&ico).exists().then_some(ico),
        });
    }

    let terminal = "shell:AppsFolder\\Microsoft.WindowsTerminal_8wekyb3d8bbwe!App";
    if shell_item_exists(terminal) {
        out.push(AppInfo {
            path: terminal.into(),
            args: None,
            name: "Terminal".into(),
            target: terminal.into(),
            match_key: "windowsterminal.exe".into(),
            icon: None,
        });
    }
    out
}
