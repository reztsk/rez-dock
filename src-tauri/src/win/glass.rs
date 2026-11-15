//! Verre natif "Expo-Glass" : un petit pool de fenêtres Win32 floutant réellement le bureau
//! (accent DWM blur/acrylique), chacune dimensionnée EXACTEMENT sur une surface du dock
//! (barre, popover) et placée juste sous la fenêtre dock dans l'ordre Z.
//!
//! Pourquoi des fenêtres dimensionnées et pas une région : DWM ignore SetWindowRgn pour le flou
//! d'accent (il floute tout le rectangle de la fenêtre). Et `backdrop-filter` CSS ne floute que le
//! contenu du WebView, jamais les fenêtres derrière.
//! Les coins arrondis viennent de DWM (Windows 11) ; un léger retrait garde le flou à l'intérieur
//! de l'arrondi CSS, plus large.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex};
use windows::core::{BOOL, HSTRING, PCSTR, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWCP_ROUND, DWMWCP_ROUNDSMALL, DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::hwnd;

/// Nombre max de surfaces simultanées (barre + popover + marge).
const POOL: usize = 3;

/// (x, y, w, h, rayon) en pixels écran physiques.
pub type GlassRect = (i32, i32, i32, i32, i32);

#[derive(Clone, Copy, Default, PartialEq)]
struct Placed {
    rect: GlassRect,
    corner: i32,
}

struct Pool {
    windows: Vec<isize>,
    placed: Vec<Option<Placed>>,
    last: Vec<GlassRect>,
    scale: f64,
}

static STATE: Mutex<Pool> = Mutex::new(Pool { windows: Vec::new(), placed: Vec::new(), last: Vec::new(), scale: 1.0 });
static MODE: AtomicU32 = AtomicU32::new(u32::MAX);
static VISIBLE: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlurMode {
    Off = 0,
    Blur = 3,
    Acrylic = 4,
}

impl BlurMode {
    /// 0 → désactivé, 1..49 → flou gaussien DWM, 50..100 → acrylique (flou intense + grain).
    pub fn from_intensity(v: f64) -> Self {
        if v <= 0.5 {
            BlurMode::Off
        } else if v < 50.0 {
            BlurMode::Blur
        } else {
            BlurMode::Acrylic
        }
    }
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient: u32,
    animation: u32,
}

#[repr(C)]
struct CompositionData {
    attrib: u32,
    data: *mut c_void,
    size: usize,
}

type SetWindowCompositionAttribute = unsafe extern "system" fn(HWND, *mut CompositionData) -> BOOL;

unsafe extern "system" fn wndproc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => DefWindowProcW(h, msg, wp, lp),
    }
}

/// Crée le pool sur un thread dédié (boucle de messages propre).
pub fn create(scale: f64) {
    let (tx, rx) = mpsc::channel::<Vec<isize>>();
    std::thread::Builder::new()
        .name("rez-glass".into())
        .spawn(move || unsafe {
            let mut created = Vec::new();
            if let Ok(module) = GetModuleHandleW(None) {
                let class = HSTRING::from("RezGlassBackdrop");
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    lpfnWndProc: Some(wndproc),
                    hInstance: module.into(),
                    lpszClassName: PCWSTR(class.as_ptr()),
                    hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                    ..Default::default()
                };
                RegisterClassExW(&wc);
                for _ in 0..POOL {
                    let h = CreateWindowExW(
                        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                        &class,
                        &HSTRING::from("REZ Glass"),
                        WS_POPUP,
                        0,
                        0,
                        1,
                        1,
                        None,
                        None,
                        Some(module.into()),
                        None,
                    );
                    if let Ok(h) = h {
                        let none = DWMWA_COLOR_NONE;
                        let _ = DwmSetWindowAttribute(h, DWMWA_BORDER_COLOR, &none as *const u32 as *const _, 4);
                        created.push(h.0 as isize);
                    }
                }
            }
            let _ = tx.send(created);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("thread verre");
    let windows = rx.recv().unwrap_or_default();
    let mut st = STATE.lock().unwrap();
    st.placed = vec![None; windows.len()];
    st.windows = windows;
    st.scale = scale;
}

pub fn set_scale(scale: f64) {
    STATE.lock().unwrap().scale = scale;
}

pub fn set_mode(mode: BlurMode) {
    if MODE.swap(mode as u32, Ordering::SeqCst) == mode as u32 {
        return;
    }
    let windows = STATE.lock().unwrap().windows.clone();
    unsafe {
        let Ok(user32) = GetModuleHandleW(&HSTRING::from("user32.dll")) else { return };
        let Some(addr) = GetProcAddress(user32, PCSTR(c"SetWindowCompositionAttribute".as_ptr() as *const u8)) else {
            return;
        };
        let swca: SetWindowCompositionAttribute = std::mem::transmute(addr);
        for w in windows {
            let mut accent = AccentPolicy {
                state: mode as u32,
                flags: if mode == BlurMode::Acrylic { 2 } else { 0 },
                // AABBGGRR — teinte quasi nulle : la couleur est gérée côté CSS (en direct).
                gradient: 0x0110_1010,
                animation: 0,
            };
            let mut data = CompositionData {
                attrib: 19, // WCA_ACCENT_POLICY
                data: &mut accent as *mut _ as *mut c_void,
                size: std::mem::size_of::<AccentPolicy>(),
            };
            let _ = swca(hwnd(w), &mut data);
        }
    }
    reapply();
}

/// Place une fenêtre de verre par surface, juste sous `dock` dans l'ordre Z.
pub fn set_surfaces(dock: isize, rects: &[GlassRect]) {
    let mut st = STATE.lock().unwrap();
    st.last = rects.to_vec();
    let show = VISIBLE.load(Ordering::SeqCst) && MODE.load(Ordering::SeqCst) != BlurMode::Off as u32;
    let rho = (8.0 * st.scale).round() as i32; // rayon DWM "ROUND" à ce DPI

    for i in 0..st.windows.len() {
        let w = hwnd(st.windows[i]);
        let target = if show { rects.get(i).copied().filter(|r| r.2 > 4 && r.3 > 4) } else { None };
        let Some((x, y, cw, ch, r)) = target else {
            if st.placed[i].take().is_some() {
                unsafe {
                    let _ = ShowWindow(w, SW_HIDE);
                }
            }
            continue;
        };
        // Coin DWM le plus proche + retrait pour rester sous l'arrondi CSS (rayon r).
        let (corner, dwm_r) = if r < 3 {
            (DWMWCP_DONOTROUND, 0)
        } else if r < 7 {
            (DWMWCP_ROUNDSMALL, (4.0 * st.scale).round() as i32)
        } else {
            (DWMWCP_ROUND, rho)
        };
        let inset = (((r - dwm_r).max(0) as f64) * 0.293).ceil() as i32;
        let rect = (x + inset, y + inset, (cw - 2 * inset).max(1), (ch - 2 * inset).max(1), r);
        let placed = Placed { rect, corner: corner.0 };
        let was = st.placed[i];
        if was == Some(placed) {
            continue;
        }
        unsafe {
            if was.map(|p| p.corner) != Some(corner.0) {
                let c: DWM_WINDOW_CORNER_PREFERENCE = corner;
                let _ = DwmSetWindowAttribute(w, DWMWA_WINDOW_CORNER_PREFERENCE, &c as *const _ as *const _, 4);
            }
            let _ = SetWindowPos(
                w,
                Some(hwnd(dock)),
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
        st.placed[i] = Some(placed);
    }
}

fn reapply() {
    let (rects, dock) = {
        let mut st = STATE.lock().unwrap();
        for p in st.placed.iter_mut() {
            *p = None;
        }
        // Force le masquage puis la ré-application complète.
        for &w in &st.windows {
            unsafe {
                let _ = ShowWindow(hwnd(w), SW_HIDE);
            }
        }
        (st.last.clone(), crate::state::SHARED.dock_hwnd.load(Ordering::Relaxed))
    };
    set_surfaces(dock, &rects);
}

pub fn set_visible(visible: bool) {
    VISIBLE.store(visible, Ordering::SeqCst);
    reapply();
}
