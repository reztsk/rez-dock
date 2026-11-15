//! Extraction d'icônes haute résolution via IShellItemImageFactory (Shell officiel),
//! encodées en PNG data-URL. Ne touche jamais au processus : uniquement au fichier sur disque.

use base64::Engine;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};

static CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

pub fn icon_data_url(path: &str, size: i32) -> Option<String> {
    let key = format!("{}|{}", path.to_lowercase(), size);
    if let Some(hit) = CACHE.lock().unwrap().get(&key) {
        return hit.clone();
    }
    let result = extract(path, size);
    CACHE.lock().unwrap().insert(key, result.clone());
    result
}

fn extract(path: &str, size: i32) -> Option<String> {
    let lower = path.to_lowercase();
    // Images fournies par l'utilisateur : servies telles quelles.
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp") {
        let bytes = std::fs::read(path).ok()?;
        let mime = if lower.ends_with(".png") {
            "png"
        } else if lower.ends_with(".webp") {
            "webp"
        } else {
            "jpeg"
        };
        return Some(format!(
            "data:image/{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ));
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
        let bmp = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let png = hbitmap_to_png(bmp);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        png.map(|p| data_url(&p))
    }
}

unsafe fn hbitmap_to_png(bmp: HBITMAP) -> Option<Vec<u8>> {
    let mut info = BITMAP::default();
    if GetObjectW(HGDIOBJ(bmp.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut info as *mut _ as *mut _)) == 0 {
        return None;
    }
    let (w, h) = (info.bmWidth, info.bmHeight.abs());
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w * h * 4) as usize];
    let dc = GetDC(None);
    let lines = GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
    ReleaseDC(None, dc);
    if lines == 0 {
        return None;
    }

    // BGRA -> RGBA + gestion alpha (absent / prémultiplié).
    let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    let premultiplied = has_alpha && px.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
        if !has_alpha {
            p[3] = 255;
        } else if premultiplied && p[3] > 0 && p[3] < 255 {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }

    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&px).ok()?;
    }
    Some(out)
}
