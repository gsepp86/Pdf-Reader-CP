// PDF Reader CP 2.2 - Programma opensource progettato da Peppe CP con l'aiuto della IA.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, Button, Color32, RichText, Vec2};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::{Duration, Instant}};
use std::io::{BufRead as _, BufReader, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};

const ACCENT: Color32 = Color32::from_rgb(198, 47, 47);
const BAR: Color32 = Color32::from_rgb(229, 231, 235);
const A4: (f32, f32) = (595.0, 842.0); // punti (1/72")
const ICON: &[u8] = include_bytes!("../assets/icon.png");
const DONATE_URL: &str = "https://paypal.me/PeppeCP";
const GITHUB_URL: &str = "https://github.com/gsepp86/Pdf-Reader-CP";
const NUPS: [u32; 6] = [1, 2, 4, 6, 9, 16]; // pagine per foglio

// ---------- impostazioni di stampa (salvate: servono a "Stampa subito") ----------
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum PageSel { All, Current, Custom }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum ScaleMode { Fit, Actual, Custom }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum ColorMode { Color, Gray, Bw }

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
struct Settings { printer: String, copies: u32, collate: bool, sel: PageSel, custom: String, scale: ScaleMode, pct: u32, rotate: bool, center: bool, nup: u32, border: bool, color: ColorMode, bright: i32, drv_printer: String, drv_mode: String, background: bool, start_max: bool, start_pan: bool, fit_page: bool, ask_copies: bool }
impl Default for Settings {
    fn default() -> Self {
        Self { printer: String::new(), copies: 1, collate: true, sel: PageSel::All, custom: "1-".into(), scale: ScaleMode::Fit, pct: 100, rotate: true, center: true, nup: 1, border: true, color: ColorMode::Color, bright: 0, drv_printer: String::new(), drv_mode: String::new(), background: false, start_max: true, start_pan: false, fit_page: false, ask_copies: true }
    }
}
impl Settings {
    fn path() -> Option<PathBuf> { dirs::config_dir().map(|d| d.join("PDFReaderCP").join("settings.json")) }
    fn load() -> Self {
        Self::path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    fn save(&self) {
        if let Some(p) = Self::path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap());
        }
    }
}

fn pages_for(st: &Settings, cur: usize, n: usize) -> Vec<usize> {
    match st.sel {
        PageSel::All => (0..n).collect(),
        PageSel::Current => vec![cur],
        PageSel::Custom => {
            // es. "1-3, 5"  |  "1,1" = pagina 1 due volte  |  "2x3" = pagina 2 per 3 volte (anche "1-3x2": ogni pagina due volte)
            let mut v = vec![];
            for part in st.custom.split(',') {
                let part = part.trim().to_lowercase();
                let (spec, rep) = match part.split_once('x') {
                    Some((a, k)) => (a.trim().to_string(), k.trim().parse::<usize>().unwrap_or(1).clamp(1, 99)),
                    None => (part.clone(), 1),
                };
                let (a, b) = match spec.split_once('-') {
                    Some((a, b)) => (a.trim().parse::<usize>().unwrap_or(1), if b.trim().is_empty() { n } else { b.trim().parse::<usize>().unwrap_or(n) }),
                    None => { let x = spec.trim().parse::<usize>().unwrap_or(0); (x, x) }
                };
                for p in a.max(1)..=b.min(n) { for _ in 0..rep { v.push(p - 1); } }
            }
            v
        }
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n { format!("{}…", s.chars().take(n - 1).collect::<String>()) } else { s.to_string() }
}

// ---------- rendering ----------
fn rot90(w: usize, h: usize, d: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut o = vec![0u8; d.len()];
    for y in 0..h { for x in 0..w {
        let (s, t) = ((y * w + x) * 4, (x * h + (h - 1 - y)) * 4);
        o[t..t + 4].copy_from_slice(&d[s..s + 4]);
    } }
    (h, w, o)
}

/// true se i byte grezzi restituiti da PDFium sono BGRA (da scambiare in RGBA). Impostato all'avvio da detect_byte_order().
static BGRA_RAW: AtomicBool = AtomicBool::new(false);

/// Rileva l'ordine dei byte di PDFium disegnando un minuscolo PDF tutto rosso: cosi' i colori sono giusti con qualsiasi versione della libreria.
fn detect_byte_order(pdfium: &Pdfium) {
    const PROBE: &[u8] = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 20 20]/Contents 4 0 R>>endobj\n4 0 obj<</Length 24>>stream\n1 0 0 rg 0 0 20 20 re f\nendstream\nendobj\ntrailer<</Root 1 0 R>>\n";
    let Ok(doc) = pdfium.load_pdf_from_byte_vec(PROBE.to_vec(), None) else { return };
    let Ok(page) = doc.pages().get(0) else { return };
    let Ok(bmp) = page.render_with_config(&PdfRenderConfig::new().set_target_width(20)) else { return };
    let raw = bmp.as_raw_bytes();
    let (w, h) = (bmp.width() as usize, bmp.height() as usize);
    let i = ((h / 2) * w + w / 2) * 4;
    if raw.len() < i + 4 { return; }
    // rosso puro: in RGBA e' [255,0,0], in BGRA e' [0,0,255]
    BGRA_RAW.store(raw[i] < 128 && raw[i + 2] > 128, Ordering::Relaxed);
}

/// Rende una pagina in RGBA con la larghezza indicata (px).
fn render(doc: &PdfDocument, page: usize, width: i32) -> Option<(usize, usize, Vec<u8>)> {
    let p = doc.pages().get(page as u16).ok()?;
    let bmp = p.render_with_config(&PdfRenderConfig::new().set_target_width(width)).ok()?;
    let (w, h) = (bmp.width() as usize, bmp.height() as usize);
    let mut d = bmp.as_raw_bytes().to_vec();
    let bgra = BGRA_RAW.load(Ordering::Relaxed);
    for c in d.chunks_exact_mut(4) { if bgra { c.swap(0, 2); } c[3] = 255; }
    Some((w, h, d))
}

fn blit(dst: &mut [u8], dw: usize, dh: usize, src: &[u8], sw: usize, sh: usize, x: i32, y: i32) {
    for r in 0..sh {
        let ty = y + r as i32;
        if ty < 0 || ty >= dh as i32 { continue; }
        let x0 = x.max(0) as usize;
        let x1 = (x + sw as i32).min(dw as i32).max(0) as usize;
        if x1 <= x0 { continue; }
        let (sx, len) = ((x0 as i32 - x) as usize, x1 - x0);
        let (d, s) = ((ty as usize * dw + x0) * 4, (r * sw + sx) * 4);
        dst[d..d + len * 4].copy_from_slice(&src[s..s + len * 4]);
    }
}

fn outline(dst: &mut [u8], dw: usize, dh: usize, x: i32, y: i32, w: usize, h: usize) {
    let mut px = |xx: i32, yy: i32| {
        if xx >= 0 && yy >= 0 && (xx as usize) < dw && (yy as usize) < dh {
            let i = (yy as usize * dw + xx as usize) * 4;
            dst[i..i + 3].copy_from_slice(&[140, 140, 140]);
        }
    };
    for i in 0..w as i32 { px(x + i, y); px(x + i, y + h as i32 - 1); }
    for j in 0..h as i32 { px(x, y + j); px(x + w as i32 - 1, y + j); }
}

/// Compone UN foglio (1, 2, 4, 6, 9 o 16 pagine) su carta `pap` (punti). Usata sia per l'anteprima sia per la stampa.
/// `ppp` = pixel per punto. Restituisce RGBA gia' ruotato come la carta fisica.
fn compose(_pdfium: &Pdfium, d: &Doc, pages: &[usize], pap: (f32, f32), st: &Settings, ppp: f32, apply_tone: bool) -> Option<(usize, usize, Vec<u8>)> {
    let n = st.nup.max(1) as usize;
    let (pw0, ph0) = d.sizes[*pages.first()?];
    let land = pw0 > ph0;
    let pg = match n { 2 => (1, 2), 4 => (2, 2), 6 => (2, 3), 9 => (3, 3), 16 => (4, 4), _ => (1, 1) }; // colonne x righe su foglio verticale
    // 2 su 1 / 6 su 1 con pagine verticali: foglio orizzontale (pagine affiancate)
    let sheet_land = st.rotate && if n == 1 { land } else if pg.0 != pg.1 { !land } else { land };
    let (cols, rows) = if sheet_land { (pg.1, pg.0) } else { pg };
    let (mx, mn) = (pap.0.max(pap.1), pap.0.min(pap.1));
    let (sw, sh) = if sheet_land { (mx, mn) } else { (mn, mx) };
    let rot = sheet_land != (pap.0 > pap.1);
    let (bw, bh) = (((sw * ppp) as usize).max(1), ((sh * ppp) as usize).max(1));
    let mut buf = vec![255u8; bw * bh * 4];
    let (cw, ch) = (sw / cols as f32, sh / rows as f32);
    let m = if n > 1 { 6.0 } else { 0.0 };
    for (i, &p) in pages.iter().enumerate() {
        let (pw, ph) = d.sizes[p];
        let (aw, ah) = (cw - 2.0 * m, ch - 2.0 * m);
        let fit = (aw / pw).min(ah / ph);
        let s = if n > 1 { fit } else { match st.scale { ScaleMode::Fit => fit, ScaleMode::Actual => 1.0, ScaleMode::Custom => st.pct.clamp(25, 200) as f32 / 100.0 } };
        let (w, h) = (pw * s, ph * s);
        let (cx, cy) = ((i % cols) as f32 * cw + m, (i / cols) as f32 * ch + m);
        let (x, y) = if st.center || n > 1 { (cx + (aw - w) / 2.0, cy + (ah - h) / 2.0) } else { (cx, cy) };
        let (iw, ih, data) = render(&d.pdf, p, ((w * ppp) as i32).max(8))?;
        let (xi, yi) = ((x * ppp) as i32, (y * ppp) as i32);
        blit(&mut buf, bw, bh, &data, iw, ih, xi, yi);
        if n > 1 && st.border {
            for t in 0..((ppp / 2.0).max(1.0) as i32) {
                outline(&mut buf, bw, bh, xi + t, yi + t, (iw as i32 - 2 * t).max(1) as usize, (ih as i32 - 2 * t).max(1) as usize);
            }
        }
    }
    if apply_tone { tone(&mut buf, st); }
    Some(if rot { rot90(bw, bh, &buf) } else { (bw, bh, buf) })
}

/// Legge i caratteri (con posizione) di una pagina dal livello di testo del PDF.
fn load_chars(doc: &PdfDocument, page: usize) -> Vec<TCh> {
    let mut v = vec![];
    let Ok(p) = doc.pages().get(page as u16) else { return v };
    let Ok(t) = p.text() else { return v };
    for ch in t.chars().iter() {
        let c = ch.unicode_char().unwrap_or('\0');
        let r = ch.loose_bounds().map(|b| [b.left().value, b.bottom().value, b.right().value, b.top().value]).unwrap_or([0.0; 4]);
        v.push(TCh { c, r });
    }
    v
}

/// Carattere piu' vicino al punto (x, y) in coordinate pagina PDF.
fn hit(chars: &[TCh], x: f32, y: f32) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, c) in chars.iter().enumerate() {
        let [l, b, r, t] = c.r;
        if r - l < 0.01 || t - b < 0.01 { continue; }
        let dx = if x < l { l - x } else if x > r { x - r } else { 0.0 };
        let dy = if y < b { b - y } else if y > t { y - t } else { 0.0 };
        let d = dx * dx + dy * dy * 4.0; // privilegia la stessa riga
        if best.map_or(true, |(_, bd)| d < bd) { best = Some((i, d)); }
        if d == 0.0 { break; }
    }
    best.map(|(i, _)| i)
}

/// Colore / scala di grigi / bianco e nero e chiaro-scuro (curva gamma: il bianco della carta resta bianco).
fn tone(buf: &mut [u8], st: &Settings) {
    if st.color == ColorMode::Color && st.bright == 0 { return; }
    let g = 2f32.powf(-(st.bright as f32) / 50.0); // >0 = piu' chiaro, <0 = piu' scuro
    let lut: Vec<u8> = (0..256).map(|v| (255.0 * (v as f32 / 255.0).powf(g)).round().clamp(0.0, 255.0) as u8).collect();
    for p in buf.chunks_exact_mut(4) {
        let lum = ((0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as usize).min(255);
        match st.color {
            ColorMode::Color => { p[0] = lut[p[0] as usize]; p[1] = lut[p[1] as usize]; p[2] = lut[p[2] as usize]; }
            ColorMode::Gray => { let l = lut[lum]; p[0] = l; p[1] = l; p[2] = l; }
            ColorMode::Bw => { let v = if lut[lum] >= 128 { 255 } else { 0 }; p[0] = v; p[1] = v; p[2] = v; }
        }
    }
}

/// Posizioni (indice di carattere) di tutte le occorrenze di `q` in `t`.
fn find_all(t: &[char], q: &[char]) -> Vec<usize> {
    let mut v = vec![];
    if q.is_empty() || t.len() < q.len() { return v; }
    for i in 0..=t.len() - q.len() { if t[i..i + q.len()] == *q { v.push(i); } }
    v
}

fn to_hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn from_hex(s: &str) -> Vec<u8> { (0..s.len() / 2).filter_map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()).collect() }

fn confirm(title: &str, text: &str) -> bool {
    rfd::MessageDialog::new().set_title(title).set_description(text).set_buttons(rfd::MessageButtons::YesNo).set_level(rfd::MessageLevel::Warning).show() == rfd::MessageDialogResult::Yes
}

fn err(msg: &str) {
    rfd::MessageDialog::new().set_title("PDF Reader CP").set_description(msg).set_level(rfd::MessageLevel::Error).show();
}

// ---------- spooler Windows (GDI) ----------
#[cfg(windows)]
mod spool {
    use windows::{core::{w, PCWSTR}, Win32::Foundation::HANDLE, Win32::Graphics::Gdi::*, Win32::Graphics::Printing::{ClosePrinter, DocumentPropertiesW, OpenPrinterW}, Win32::Storage::Xps::*, Win32::UI::WindowsAndMessaging::GetForegroundWindow};
    pub struct Job { dc: HDC, aborted: bool }

    /// Crea il contesto di stampa, applicando (se presenti) le impostazioni del driver (DEVMODE).
    unsafe fn make_dc(printer: &str, devmode: Option<&[u8]>) -> HDC {
        let p: Vec<u16> = printer.encode_utf16().chain([0]).collect();
        let mut al: Vec<u64> = Vec::new(); // copia allineata del DEVMODE
        let dm: Option<*const DEVMODEW> = match devmode {
            Some(d) if d.len() >= std::mem::size_of::<DEVMODEW>() => {
                al = vec![0u64; (d.len() + 7) / 8];
                std::ptr::copy_nonoverlapping(d.as_ptr(), al.as_mut_ptr() as *mut u8, d.len());
                Some(al.as_ptr() as *const DEVMODEW)
            }
            _ => None,
        };
        let dc = CreateDCW(w!("WINSPOOL"), PCWSTR(p.as_ptr()), PCWSTR::null(), dm);
        drop(al);
        dc
    }

    /// Finestra "Proprieta'" del driver (fronte/retro, vassoio, qualita', colore...). Some(DEVMODE) se l'utente preme OK.
    pub fn driver_dialog(printer: &str, current: Option<&[u8]>) -> Option<Vec<u8>> {
        unsafe {
            let name: Vec<u16> = printer.encode_utf16().chain([0]).collect();
            let pn = PCWSTR(name.as_ptr());
            let mut h = HANDLE::default();
            OpenPrinterW(pn, &mut h, None).ok()?;
            let hwnd = GetForegroundWindow();
            let size = DocumentPropertiesW(hwnd, h, pn, None, None, 0);
            let mut out = None;
            if size > 0 {
                let mut buf = vec![0u64; (size as usize + 7) / 8];
                let dm = buf.as_mut_ptr() as *mut DEVMODEW;
                let mut mode = 4u32 | 2u32; // DM_IN_PROMPT | DM_OUT_BUFFER
                let mut input: Option<*const DEVMODEW> = None;
                if let Some(c) = current {
                    if c.len() == size as usize {
                        std::ptr::copy_nonoverlapping(c.as_ptr(), buf.as_mut_ptr() as *mut u8, c.len());
                        mode |= 8; // DM_IN_BUFFER: riparte dalle impostazioni attuali
                        input = Some(dm as *const DEVMODEW);
                    }
                }
                if DocumentPropertiesW(hwnd, h, pn, Some(dm), input, mode) == 1 { // IDOK
                    out = Some(std::slice::from_raw_parts(buf.as_ptr() as *const u8, size as usize).to_vec());
                }
            }
            let _ = ClosePrinter(h);
            out
        }
    }

    /// Dimensione fisica della carta (punti) con le impostazioni correnti del driver.
    pub fn paper_pts(printer: &str, devmode: Option<&[u8]>) -> Option<(f32, f32)> {
        unsafe {
            let dc = make_dc(printer, devmode);
            if dc.is_invalid() { return None; }
            let (dx, dy) = (GetDeviceCaps(dc, LOGPIXELSX) as f32, GetDeviceCaps(dc, LOGPIXELSY) as f32);
            let r = (GetDeviceCaps(dc, PHYSICALWIDTH) as f32 / dx * 72.0, GetDeviceCaps(dc, PHYSICALHEIGHT) as f32 / dy * 72.0);
            let _ = DeleteDC(dc);
            if r.0 > 50.0 && r.1 > 50.0 { Some(r) } else { None }
        }
    }

    impl Job {
        pub fn new(printer: &str, name: &str, devmode: Option<&[u8]>) -> Result<Job, String> {
            unsafe {
                let dc = make_dc(printer, devmode);
                if dc.is_invalid() { return Err(format!("Impossibile aprire la stampante «{printer}»")); }
                let n: Vec<u16> = name.encode_utf16().chain([0]).collect();
                let di = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: PCWSTR(n.as_ptr()), ..Default::default() };
                if StartDocW(dc, &di) <= 0 { let _ = DeleteDC(dc); return Err("Avvio del lavoro di stampa non riuscito".into()); }
                Ok(Job { dc, aborted: false })
            }
        }
        /// Area stampabile in punti e risoluzione (dpi).
        pub fn area(&self) -> (f32, f32, f32) {
            unsafe {
                let d = GetDeviceCaps(self.dc, LOGPIXELSX) as f32;
                (GetDeviceCaps(self.dc, HORZRES) as f32 / d * 72.0, GetDeviceCaps(self.dc, VERTRES) as f32 / d * 72.0, d)
            }
        }
        /// Annulla il lavoro: non viene mandato alla stampante.
        pub fn abort(&mut self) { if !self.aborted { self.aborted = true; unsafe { AbortDoc(self.dc); } } }
        pub fn page(&self, bgra: &[u8], w: i32, h: i32, x: i32, y: i32, dw: i32, dh: i32) {
            unsafe {
                StartPage(self.dc);
                let mut bi = BITMAPINFO::default();
                bi.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: 0, ..Default::default() };
                StretchDIBits(self.dc, x, y, dw, dh, 0, 0, w, h, Some(bgra.as_ptr() as *const _), &bi, DIB_RGB_COLORS, SRCCOPY);
                EndPage(self.dc);
            }
        }
    }
    impl Drop for Job { fn drop(&mut self) { unsafe { if !self.aborted { EndDoc(self.dc); } let _ = DeleteDC(self.dc); } } }
}
#[cfg(not(windows))]
mod spool {
    pub struct Job;
    impl Job {
        pub fn new(_: &str, _: &str, _: Option<&[u8]>) -> Result<Job, String> { Err("La stampa e' disponibile solo su Windows".into()) }
        pub fn area(&self) -> (f32, f32, f32) { (595.0, 842.0, 72.0) }
        pub fn abort(&mut self) {}
        pub fn page(&self, _: &[u8], _: i32, _: i32, _: i32, _: i32, _: i32, _: i32) {}
    }
    pub fn driver_dialog(_: &str, _: Option<&[u8]>) -> Option<Vec<u8>> { None }
    pub fn paper_pts(_: &str, _: Option<&[u8]>) -> Option<(f32, f32)> { None }
}

// ---------- applicazione ----------
struct Doc { id: u64, pdf: PdfDocument<'static>, path: PathBuf, name: String, sizes: Vec<(f32, f32)>, cur: usize, zoom: f32, fitpage: bool, tex: HashMap<(usize, i32), egui::TextureHandle>, text: HashMap<usize, Vec<TCh>>, ptext: HashMap<usize, Vec<char>> }
#[derive(PartialEq, Clone, Copy)]
enum Tab { Home, Doc(usize) }
type PvKey = (u64, ColorMode, i32); // versione del foglio + colore + chiaro/scuro
type RawKey = (usize, usize, PageSel, String, u32, bool, ScaleMode, u32, bool, bool, (i32, i32));
/// Carattere del testo di una pagina: unicode + riquadro [sinistra, basso, destra, alto] in punti PDF.
struct TCh { c: char, r: [f32; 4] }
/// Selezione di testo: documento, pagina, indice del primo e dell'ultimo carattere.
struct Sel { doc: usize, page: usize, a: usize, b: usize }

/// Lavoro di stampa in corso: procede un passo (un foglio) per volta cosi' l'interfaccia resta viva e mostra il cerchio di caricamento.
struct PrintTask {
    ids: Vec<u64>, st: Settings, di: usize, job: Option<spool::Job>,
    sheets: Vec<Vec<usize>>, order: Vec<usize>, pos: usize, area: (f32, f32, f32),
    last: Option<(usize, usize, usize, Vec<u8>)>, total: usize, done: usize, cancel: bool,
}

struct App {
    pdfium: &'static Pdfium, docs: Vec<Doc>, tab: Tab,
    st: Settings, draft: Settings, printers: Vec<String>,
    show_print: bool, prev: usize, pv: Option<(PvKey, egui::TextureHandle)>,
    msg: String, msg_at: Option<Instant>, logo: egui::TextureHandle, fs: bool, show_info: bool, scroll_tab: bool,
    ipc: Option<std::sync::mpsc::Receiver<Ipc>>, started: bool, sel: Option<Sel>, paper: (f32, f32), paper_key: (String, String),
    pv_raw: Option<(RawKey, usize, usize, Vec<u8>)>, pv_gen: u64, last_size: (i32, i32), last_change: Instant,
    pan: bool, search: String, search_hit: Option<(usize, usize)>, search_focus: bool, jump: bool,
    show_cfg: bool, cfg: Settings, cfg_tab: usize,
    show_pall: bool, pa_mode: bool,
    over_acc: f32, over_at: Instant, edge: (bool, bool), scroll_reset: Option<bool>, tabs_overflow: bool,
    task: Option<PrintTask>, tab_drag: Option<usize>, next_id: u64,
}

impl App {
    fn new(cc: &eframe::CreationContext, listener: Option<std::net::TcpListener>, init: Vec<PathBuf>) -> Self {
        let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_default();
        let lib = Pdfium::pdfium_platform_library_name_at_path(&exe);
        let bind = match Pdfium::bind_to_library(lib).or_else(|_| Pdfium::bind_to_system_library()) {
            Ok(b) => b,
            Err(_) => { err("Libreria pdfium.dll non trovata. Reinstalla il programma."); std::process::exit(1) }
        };
        let mut v = egui::Visuals::light();
        v.selection.bg_fill = ACCENT;
        cc.egui_ctx.set_visuals(v);
        let ic = eframe::icon_data::from_png_bytes(ICON).expect("icona");
        let logo = cc.egui_ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied([ic.width as usize, ic.height as usize], &ic.rgba), Default::default());
        let mut printers: Vec<String> = printers::get_printers().into_iter().map(|p| p.name).collect();
        printers.sort_by_key(|p| p.to_lowercase()); // ordine alfabetico
        printers.dedup();
        let mut st = Settings::load();
        if !printers.contains(&st.printer) {
            st.printer = printers::get_default_printer().map(|p| p.name).or_else(|| printers.first().cloned()).unwrap_or_default();
        }
        let pdfium: &'static Pdfium = Box::leak(Box::new(Pdfium::new(bind))); // vive per tutto il programma: i PDF aperti restano in memoria
        detect_byte_order(pdfium);
        let start_pan = st.start_pan;
        let ipc = listener.map(|l| { let (tx, rx) = std::sync::mpsc::channel(); serve(l, tx, cc.egui_ctx.clone()); rx });
        let mut app = App { pdfium, docs: vec![], tab: Tab::Home, draft: st.clone(), st, printers, show_print: false, prev: 0, pv: None, msg: String::new(), msg_at: None, logo, fs: false, show_info: false, scroll_tab: false, ipc, started: false, sel: None, paper: A4, paper_key: (String::new(), String::new()), pv_raw: None, pv_gen: 0, last_size: (0, 0), last_change: Instant::now(), pan: start_pan, search: String::new(), search_hit: None, search_focus: false, jump: false, show_cfg: false, cfg: Settings::default(), cfg_tab: 0, show_pall: false, pa_mode: false, over_acc: 0.0, over_at: Instant::now(), edge: (true, false), scroll_reset: None, tabs_overflow: false, task: None, tab_drag: None, next_id: 1 };
        for a in init { app.open(a); }
        app
    }

    /// Sposta una scheda (trascinandola): STAMPA TUTTO stampa le schede nell'ordine in cui sono disposte.
    fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.docs.len() || to >= self.docs.len() { return; }
        let cur_id = if let Tab::Doc(i) = self.tab { self.docs.get(i).map(|d| d.id) } else { None };
        let d = self.docs.remove(from);
        self.docs.insert(to, d);
        if let Some(id) = cur_id { if let Some(i) = self.docs.iter().position(|d| d.id == id) { self.tab = Tab::Doc(i); } }
        self.sel = None; self.pv = None; self.pv_raw = None; self.show_print = false; self.scroll_tab = true;
    }

    fn toast(&mut self, s: String) { self.msg = s; self.msg_at = Some(Instant::now()); }

    fn ensure_chars(&mut self, di: usize, page: usize) {
        if di >= self.docs.len() || self.docs[di].text.contains_key(&page) { return; }
        let v = load_chars(&self.docs[di].pdf, page);
        let d = &mut self.docs[di];
        if d.text.len() >= 20 { d.text.clear(); }
        d.text.insert(page, v);
    }

    /// Testo (minuscolo) di una pagina, per la ricerca.
    fn ensure_ptext(&mut self, di: usize, page: usize) {
        if di >= self.docs.len() || self.docs[di].ptext.contains_key(&page) { return; }
        let v: Vec<char> = self.docs[di].pdf.pages().get(page as u16).ok()
            .and_then(|p| p.text().ok().map(|t| t.all().chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect::<Vec<char>>()))
            .unwrap_or_default();
        self.docs[di].ptext.insert(page, v);
    }

    /// Cerca il risultato successivo/precedente (anche in altre pagine) e porta li' la vista.
    fn search_step(&mut self, di: usize, forward: bool) {
        let q: Vec<char> = self.search.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
        if q.is_empty() { return; }
        let n = self.docs[di].sizes.len();
        let p0 = self.docs[di].cur;
        let from = self.search_hit.filter(|(p, _)| *p == p0).map(|(_, s)| s);
        let mut found = None;
        for k in 0..=n {
            let p = if forward { (p0 + k) % n } else { (p0 + n - k % n) % n };
            self.ensure_ptext(di, p);
            let hits = find_all(&self.docs[di].ptext[&p], &q);
            let pick = if forward { hits.iter().copied().find(|&h| k > 0 || from.map_or(true, |f| h > f)) }
                       else { hits.iter().copied().rev().find(|&h| k > 0 || from.map_or(true, |f| h < f)) };
            if let Some(h) = pick { found = Some((p, h)); break; }
        }
        match found {
            Some((p, h)) => { self.docs[di].cur = p; self.search_hit = Some((p, h)); self.jump = true; }
            None => { let t = self.search.clone(); self.toast(format!("Nessun risultato per «{t}»")); }
        }
    }

    fn selected_text(&self) -> Option<String> {
        let s = self.sel.as_ref()?;
        let ch = self.docs.get(s.doc)?.text.get(&s.page)?;
        let (lo, hi) = (s.a.min(s.b), s.a.max(s.b));
        let t: String = ch.get(lo..=hi)?.iter().map(|c| c.c).filter(|c| *c >= ' ' || matches!(*c, '\n' | '\r' | '\t')).collect();
        if t.trim().is_empty() { None } else { Some(t) }
    }

    fn open(&mut self, path: PathBuf) {
        // gia' aperto: passa a quella scheda invece di duplicarla
        if let Some(i) = self.docs.iter().position(|d| d.path == path) { self.tab = Tab::Doc(i); self.scroll_tab = true; return; }
        let pdf = self.pdfium.load_pdf_from_file(&path, None).ok();
        let sizes: Option<Vec<(f32, f32)>> = pdf.as_ref().map(|d| d.pages().iter().map(|p| (p.width().value, p.height().value)).collect());
        match (pdf, sizes) {
            (Some(pdf), Some(s)) if !s.is_empty() => {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let id = self.next_id; self.next_id += 1;
                self.docs.push(Doc { id, pdf, path, name, sizes: s, cur: 0, zoom: 1.0, fitpage: self.st.fit_page, tex: HashMap::new(), text: HashMap::new(), ptext: HashMap::new() });
                self.tab = Tab::Doc(self.docs.len() - 1);
                self.scroll_tab = true;
            }
            _ => err(&format!("Impossibile aprire il file:\n{}", path.display())),
        }
    }

    fn pick(&mut self) {
        if let Some(v) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_title("Apri PDF").pick_files() {
            for p in v { self.open(p); }
        }
    }

    fn get_tex(&mut self, ctx: &egui::Context, di: usize, page: usize, width: i32) -> Option<egui::TextureHandle> {
        if let Some(t) = self.docs[di].tex.get(&(page, width)) { return Some(t.clone()); }
        // mentre ridimensioni o fai zoom mostro la versione gia' pronta (scalata) e rifaccio il rendering a riposo: niente blocchi
        if self.last_change.elapsed() < Duration::from_millis(180) {
            if let Some(t) = self.docs[di].tex.iter().filter(|(k, _)| k.0 == page).min_by_key(|(k, _)| (k.1 - width).abs()).map(|(_, t)| t.clone()) {
                ctx.request_repaint_after(Duration::from_millis(200));
                return Some(t);
            }
        }
        let (w, h, d) = render(&self.docs[di].pdf, page, width)?;
        let t = ctx.load_texture("page", egui::ColorImage::from_rgba_unmultiplied([w, h], &d), egui::TextureOptions::LINEAR);
        let doc = &mut self.docs[di];
        doc.tex.retain(|k, _| k.0 != page);
        if doc.tex.len() >= 6 { doc.tex.clear(); }
        doc.tex.insert((page, width), t.clone());
        Some(t)
    }

    /// Avvia la stampa delle schede indicate (nell'ordine dato). `all_pages` = true: tutte le pagine di ogni scheda (STAMPA TUTTO).
    fn print_docs(&mut self, idx: &[usize], all_pages: bool) {
        if self.task.is_some() { self.toast("Stampa gia' in corso".into()); return; }
        if self.st.printer.is_empty() { err("Nessuna stampante selezionata"); return; }
        let mut st = self.st.clone();
        if all_pages { st.sel = PageSel::All; }
        let ids: Vec<u64> = idx.iter().filter_map(|&i| self.docs.get(i).map(|d| d.id)).collect();
        if ids.is_empty() { return; }
        let nup = st.nup.max(1) as usize;
        let c = st.copies.max(1) as usize;
        let total: usize = ids.iter().filter_map(|id| self.docs.iter().find(|d| d.id == *id))
            .map(|d| ((pages_for(&st, d.cur, d.sizes.len()).len() + nup - 1) / nup) * c).sum();
        self.task = Some(PrintTask { ids, st, di: 0, job: None, sheets: vec![], order: vec![], pos: 0, area: (595.0, 842.0, 72.0), last: None, total, done: 0, cancel: false });
    }

    /// Un passo del lavoro di stampa. Quando finisce, il lavoro e' in coda nella stampante e il cerchio di caricamento sparisce.
    fn print_step(&mut self, ctx: &egui::Context) {
        let Some(mut t) = self.task.take() else { return };
        if t.cancel {
            if let Some(j) = t.job.as_mut() { j.abort(); }
            self.toast("Stampa annullata".into());
            return;
        }
        match self.advance(&mut t) {
            Ok(true) => { self.task = Some(t); ctx.request_repaint(); }
            Ok(false) => { let (p, n) = (t.st.printer.clone(), t.ids.len()); self.toast(format!("Lavoro inviato a «{p}» ({n} schede in coda)")); }
            Err(e) => { if let Some(j) = t.job.as_mut() { j.abort(); } err(&e); }
        }
    }

    /// Esegue un'unita' di lavoro: apre il lavoro di una scheda, oppure invia un foglio, oppure chiude il lavoro (che va in coda).
    fn advance(&self, t: &mut PrintTask) -> Result<bool, String> {
        if t.job.is_none() { // apri la scheda successiva
            if t.di >= t.ids.len() { return Ok(false); }
            let Some(d) = self.docs.iter().find(|d| d.id == t.ids[t.di]) else { t.di += 1; return Ok(true); }; // scheda chiusa nel frattempo
            let pages = pages_for(&t.st, d.cur, d.sizes.len());
            if pages.is_empty() { return Err(format!("Nessuna pagina da stampare in {}", d.name)); }
            let nup = t.st.nup.max(1) as usize;
            t.sheets = pages.chunks(nup).map(|c| c.to_vec()).collect();
            let (c, ns) = (t.st.copies.max(1) as usize, t.sheets.len());
            t.order = if t.st.collate { (0..c).flat_map(|_| 0..ns).collect() } else { (0..ns).flat_map(|s| std::iter::repeat(s).take(c)).collect() };
            let dm: Option<Vec<u8>> = if t.st.drv_printer == t.st.printer && !t.st.drv_mode.is_empty() { Some(from_hex(&t.st.drv_mode)) } else { None };
            let job = spool::Job::new(&t.st.printer, &d.name, dm.as_deref()).or_else(|e| if dm.is_some() { spool::Job::new(&t.st.printer, &d.name, None) } else { Err(e) })?;
            t.area = job.area(); t.pos = 0; t.last = None; t.job = Some(job);
            return Ok(true);
        }
        if t.pos < t.order.len() { // invia un foglio
            let Some(d) = self.docs.iter().find(|d| d.id == t.ids[t.di]) else {
                if let Some(j) = t.job.as_mut() { j.abort(); }
                t.job = None; t.di += 1; return Ok(true);
            };
            let s = t.order[t.pos];
            if t.last.as_ref().map(|l| l.0) != Some(s) {
                let (aw, ah, _) = t.area;
                let (w, h, mut b) = compose(&self.pdfium, d, &t.sheets[s], (aw, ah), &t.st, 300.0 / 72.0, true).ok_or("Rendering della pagina non riuscito")?;
                for q in b.chunks_exact_mut(4) { q.swap(0, 2); } // RGBA -> BGRA per GDI
                t.last = Some((s, w, h, b));
            }
            if let (Some(l), Some(job)) = (t.last.as_ref(), t.job.as_ref()) {
                let (aw, ah, dpi) = t.area;
                let f = dpi / 72.0;
                job.page(&l.3, l.1 as i32, l.2 as i32, 0, 0, (aw * f) as i32, (ah * f) as i32);
            }
            t.pos += 1; t.done += 1;
            return Ok(true);
        }
        t.job = None; // chiude il lavoro: la scheda e' in coda nella stampante
        t.di += 1;
        Ok(true)
    }

    /// STAMPA TUTTO: tutte le schede aperte (ognuna per intero), in sequenza, con un solo messaggio di conferma.
    fn print_all(&mut self) {
        if !self.docs.is_empty() { self.show_pall = true; }
    }

    /// Richiesta di STAMPA TUTTO: stampare con le impostazioni attuali (STAMPA), cambiarle al volo (CAMBIA) o annullare.
    fn pall_window(&mut self, ctx: &egui::Context) {
        let n = self.docs.len();
        if n == 0 { self.show_pall = false; return; }
        let (mut open, mut go, mut one, mut change, mut cancel) = (true, false, false, false, ctx.input(|i| i.key_pressed(egui::Key::Escape)));
        let c = self.st.copies.max(1);
        let nup = self.st.nup.max(1) as usize;
        let pagine: usize = self.docs.iter().map(|d| d.sizes.len()).sum();
        let fogli: usize = self.docs.iter().map(|d| (d.sizes.len() + nup - 1) / nup).sum(); // fogli per copia
        let tot = fogli * c as usize;
        let warn = c > 1 && self.st.ask_copies;
        let colore = match self.st.color { ColorMode::Color => "COLORI", ColorMode::Gray => "SCALA DI GRIGI", ColorMode::Bw => "BIANCO E NERO" };
        let stampante = self.st.printer.to_uppercase();
        egui::Window::new("STAMPA TUTTO").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_min_width(440.0);
            ui.label(RichText::new("STAMPARE CON LE IMPOSTAZIONI ATTUALI?").strong().size(18.0));
            ui.add_space(6.0);
            if warn { ui.label(RichText::new(format!("ATTENZIONE: SONO IMPOSTATE {c} COPIE PER OGNI FILE.")).strong().color(ACCENT)); ui.add_space(4.0); }
            ui.label(format!("SCHEDE APERTE: {n}"));
            ui.label(format!("PAGINE PER COPIA: {pagine}"));
            ui.label(format!("FOGLI PER COPIA: {fogli} ({nup} PAGINE PER FOGLIO)"));
            ui.label(format!("COPIE PER OGNI FILE: {c}"));
            ui.label(RichText::new(format!("TOTALE FOGLI: {tot}")).strong());
            ui.label(format!("STAMPANTE: {stampante}"));
            ui.label(format!("COLORE: {colore}"));
            ui.label(RichText::new("LE SCHEDE VENGONO STAMPATE NELL'ORDINE IN CUI SONO DISPOSTE").weak().small());
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(Button::new(RichText::new("STAMPA").strong().size(16.0).color(Color32::WHITE)).fill(ACCENT).min_size(Vec2::new(120.0, 38.0))).clicked() { go = true; }
                if ui.add(Button::new(RichText::new("CAMBIA").size(16.0)).min_size(Vec2::new(110.0, 38.0))).on_hover_text("APRE LE IMPOSTAZIONI DI STAMPA").clicked() { change = true; }
                if warn && ui.add(Button::new(RichText::new("1 COPIA PER FILE").size(16.0)).min_size(Vec2::new(150.0, 38.0))).on_hover_text("IMPOSTA 1 COPIA PER FILE E STAMPA").clicked() { one = true; }
                if ui.add(Button::new(RichText::new("ANNULLA").size(16.0)).min_size(Vec2::new(110.0, 38.0))).clicked() { cancel = true; }
            });
        });
        let all: Vec<usize> = (0..n).collect();
        if go { self.show_pall = false; self.print_docs(&all, true); }
        else if one { self.show_pall = false; self.st.copies = 1; self.st.save(); self.print_docs(&all, true); }
        else if change { // porta alle impostazioni di stampa: li' "STAMPA TUTTO" stampa tutte le schede con le nuove impostazioni
            self.show_pall = false;
            if !matches!(self.tab, Tab::Doc(_)) { self.tab = Tab::Doc(0); }
            self.show_print = true; self.draft = self.st.clone(); self.prev = 0; self.pv = None; self.pv_raw = None; self.pa_mode = true;
        }
        else if cancel || !open { self.show_pall = false; }
    }

    /// Stampa subito la scheda indicata; con piu' di una copia chiede conferma.
    fn print_one(&mut self, di: usize) {
        let c = self.st.copies;
        if c > 1 && self.st.ask_copies && !confirm("Stampa", &format!("Sicuro di voler stampare {c} copie del file?\n\n{}", self.docs[di].name)) { return; }
        self.print_docs(&[di], false);
    }

    /// Finestra Impostazioni: si modifica una copia e si applica/salva con "Salva".
    fn settings_window(&mut self, ctx: &egui::Context) {
        let (mut open, mut save, mut cancel, mut reset) = (true, false, false, false);
        egui::Window::new("Impostazioni").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_min_width(470.0);
            ui.horizontal(|ui| {
                for (i, n) in ["Generale", "Stampa"].into_iter().enumerate() {
                    if ui.selectable_label(self.cfg_tab == i, RichText::new(n).size(16.0)).clicked() { self.cfg_tab = i; }
                }
            });
            ui.separator();
            if self.cfg_tab == 0 {
                ui.group(|ui| {
                    ui.label(RichText::new("Avvio e finestre").strong().color(ACCENT));
                    ui.checkbox(&mut self.cfg.background, "Apri sempre la finestra in secondo piano (non ruba il focus)");
                    ui.label(RichText::new("Aprendo altri PDF con il programma già aperto, restano nella sua finestra senza portarla davanti.").weak());
                    ui.checkbox(&mut self.cfg.start_max, "Avvia sempre ingrandita");
                });
                ui.group(|ui| {
                    ui.label(RichText::new("Visualizzazione").strong().color(ACCENT));
                    ui.horizontal(|ui| {
                        ui.label("Strumento iniziale:");
                        ui.radio_value(&mut self.cfg.start_pan, false, "Seleziona testo");
                        ui.radio_value(&mut self.cfg.start_pan, true, "Sposta pagina");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Vista iniziale:");
                        ui.radio_value(&mut self.cfg.fit_page, false, "Larghezza");
                        ui.radio_value(&mut self.cfg.fit_page, true, "Pagina intera");
                    });
                });
            } else {
                ui.group(|ui| {
                    ui.label(RichText::new("Stampa").strong().color(ACCENT));
                    ui.horizontal(|ui| {
                        ui.label("Stampante:");
                        egui::ComboBox::from_id_salt("cfg_pr").width(300.0).selected_text(self.cfg.printer.clone()).show_ui(ui, |ui| {
                            for p in &self.printers { ui.selectable_value(&mut self.cfg.printer, p.clone(), p); }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Copie:");
                        ui.add(egui::DragValue::new(&mut self.cfg.copies).range(1..=99));
                        ui.label("Pagine per foglio:");
                        egui::ComboBox::from_id_salt("cfg_nu").width(80.0).selected_text(format!("{} su 1", self.cfg.nup)).show_ui(ui, |ui| {
                            for k in NUPS { ui.selectable_value(&mut self.cfg.nup, k, format!("{k} su 1")); }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Colore:");
                        ui.radio_value(&mut self.cfg.color, ColorMode::Color, "Colori");
                        ui.radio_value(&mut self.cfg.color, ColorMode::Gray, "Scala di grigi");
                        ui.radio_value(&mut self.cfg.color, ColorMode::Bw, "Bianco e nero");
                    });
                    ui.checkbox(&mut self.cfg.ask_copies, "Chiedi conferma quando stampo più di una copia");
                });
            }
            ui.separator();
            ui.label(RichText::new(format!("Salvate in: {}", Settings::path().map(|p| p.display().to_string()).unwrap_or_default())).weak().small());
            ui.horizontal(|ui| {
                if ui.add(Button::new(RichText::new("Salva").strong().color(Color32::WHITE)).fill(ACCENT).min_size(Vec2::new(110.0, 34.0))).clicked() { save = true; }
                if ui.add(Button::new("Annulla").min_size(Vec2::new(90.0, 34.0))).clicked() { cancel = true; }
                if ui.add(Button::new("Ripristina predefinite").min_size(Vec2::new(170.0, 34.0))).clicked() { reset = true; }
            });
        });
        if reset { let p = self.cfg.printer.clone(); self.cfg = Settings::default(); self.cfg.printer = p; }
        if save { self.st = self.cfg.clone(); self.st.save(); self.show_cfg = false; self.toast("Impostazioni salvate".into()); }
        if cancel || !open { self.show_cfg = false; }
    }

    fn print_dialog(&mut self, ctx: &egui::Context, di: usize) {
        let (mut open, mut go, mut cancel, mut drv, mut drv_reset) = (true, false, false, false, false);
        let n = self.docs[di].sizes.len();
        // le impostazioni del driver valgono solo per la stampante per cui sono state fatte
        if self.draft.drv_printer != self.draft.printer { self.draft.drv_mode.clear(); }
        let pk = (self.draft.printer.clone(), self.draft.drv_mode.clone());
        if self.paper_key != pk {
            let dm = if pk.1.is_empty() { None } else { Some(from_hex(&pk.1)) };
            self.paper = spool::paper_pts(&pk.0, dm.as_deref()).unwrap_or(A4);
            self.paper_key = pk;
        }
        let nup = self.draft.nup.max(1) as usize;
        let pages = pages_for(&self.draft, self.docs[di].cur, n);
        let sheets = (pages.len() + nup - 1) / nup;
        let prev = self.prev.min(sheets.saturating_sub(1));
        // anteprima: il foglio si ricompone solo se cambiano pagine/scala/carta; colore e chiaro/scuro si applicano al volo
        let d0 = &self.draft;
        let rk: RawKey = (di, prev, d0.sel, d0.custom.clone(), d0.nup, d0.border, d0.scale, d0.pct, d0.rotate, d0.center, (self.paper.0 as i32, self.paper.1 as i32));
        if self.pv_raw.as_ref().map(|r| &r.0) != Some(&rk) {
            self.pv_gen += 1;
            self.pv_raw = pages.chunks(nup).nth(prev)
                .and_then(|chunk| compose(&self.pdfium, &self.docs[di], chunk, self.paper, &self.draft, 1.5, false))
                .map(|(w, h, b)| (rk, w, h, b));
        }
        let tk: PvKey = (self.pv_gen, self.draft.color, self.draft.bright);
        if self.pv.as_ref().map(|p| &p.0) != Some(&tk) {
            self.pv = self.pv_raw.as_ref().map(|(_, w, h, b)| {
                let mut t = b.clone();
                tone(&mut t, &self.draft);
                (tk, ctx.load_texture("pv", egui::ColorImage::from_rgba_unmultiplied([*w, *h], &t), egui::TextureOptions::LINEAR))
            });
        }
        let maxh = (ctx.screen_rect().height() - 140.0).max(320.0);
        egui::Window::new("Stampa").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            { // pulsanti e testi piu' grandi
                let s = ui.style_mut();
                s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(15.5));
                s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.5));
                s.spacing.button_padding = Vec2::new(12.0, 6.0);
                s.spacing.interact_size.y = 28.0;
                s.spacing.icon_width = 19.0;
                s.spacing.slider_width = 190.0;
            }
            if self.pa_mode {
                ui.label(RichText::new(format!("STAMPA TUTTO: LE IMPOSTAZIONI VALGONO PER TUTTE LE {} SCHEDE APERTE (TUTTE LE PAGINE DI OGNI SCHEDA)", self.docs.len())).strong().color(ACCENT));
                ui.add_space(4.0);
            }
            ui.horizontal_top(|ui| {
                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid(); // barra di scorrimento ben visibile quando serve
                egui::ScrollArea::vertical().max_height(maxh).auto_shrink([true, true]).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(390.0);
                        ui.group(|ui| {
                            ui.label(RichText::new("Stampante").strong().color(ACCENT));
                            egui::ComboBox::from_id_salt("pr").width(350.0).selected_text(self.draft.printer.clone()).show_ui(ui, |ui| {
                                for p in &self.printers { ui.selectable_value(&mut self.draft.printer, p.clone(), p); }
                            });
                            ui.horizontal(|ui| {
                                if ui.add(Button::new("Impostazioni driver stampante…").min_size(Vec2::new(270.0, 34.0)))
                                    .on_hover_text("Apre le proprietà del driver: fronte/retro, vassoio, qualità, colore, finitura...").clicked() { drv = true; }
                                if !self.draft.drv_mode.is_empty() && ui.add(Button::new("Azzera").min_size(Vec2::new(70.0, 34.0))).on_hover_text("Torna alle impostazioni predefinite del driver").clicked() { drv_reset = true; }
                            });
                            if !self.draft.drv_mode.is_empty() { ui.label(RichText::new("Impostazioni del driver attive").weak()); }
                            ui.horizontal(|ui| {
                                ui.label("Copie");
                                if ui.add(Button::new(RichText::new("-").size(20.0)).min_size(Vec2::new(40.0, 34.0))).clicked() { self.draft.copies = self.draft.copies.saturating_sub(1).max(1); }
                                ui.add(egui::DragValue::new(&mut self.draft.copies).range(1..=99));
                                if ui.add(Button::new(RichText::new("+").size(20.0)).min_size(Vec2::new(40.0, 34.0))).clicked() { self.draft.copies = (self.draft.copies + 1).min(99); }
                                ui.checkbox(&mut self.draft.collate, "Fascicola");
                            });
                        });
                        ui.group(|ui| {
                            ui.label(RichText::new(format!("Intervallo pagine · {n} totali")).strong().color(ACCENT));
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.draft.sel, PageSel::All, "Tutto");
                                ui.radio_value(&mut self.draft.sel, PageSel::Current, "Corrente");
                                ui.radio_value(&mut self.draft.sel, PageSel::Custom, "Personalizzato");
                            });
                            if self.draft.sel == PageSel::Custom {
                                ui.text_edit_singleline(&mut self.draft.custom);
                                ui.label(RichText::new("Esempi: 1-3, 5  ·  1,1 (pagina 1 due volte)  ·  2x3 (pagina 2 tre volte)").weak().small());
                            }
                        });
                        ui.group(|ui| {
                            ui.label(RichText::new("Pagine per foglio e scala").strong().color(ACCENT));
                            ui.horizontal(|ui| {
                                ui.label("Pagine per foglio");
                                egui::ComboBox::from_id_salt("nu").width(90.0).selected_text(format!("{} su 1", self.draft.nup)).show_ui(ui, |ui| {
                                    for k in NUPS { ui.selectable_value(&mut self.draft.nup, k, format!("{k} su 1")); }
                                });
                            });
                            ui.add_enabled_ui(self.draft.nup > 1, |ui| ui.checkbox(&mut self.draft.border, "Bordo attorno alle pagine"));
                            ui.add_enabled_ui(self.draft.nup == 1, |ui| {
                                ui.radio_value(&mut self.draft.scale, ScaleMode::Fit, "Adatta alla pagina");
                                ui.radio_value(&mut self.draft.scale, ScaleMode::Actual, "Dimensioni reali (100%)");
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut self.draft.scale, ScaleMode::Custom, "Proporzione personalizzata");
                                    ui.add_enabled(self.draft.scale == ScaleMode::Custom, egui::DragValue::new(&mut self.draft.pct).range(25..=200).suffix(" %"));
                                });
                            });
                            ui.checkbox(&mut self.draft.rotate, "Ruota automaticamente");
                            ui.checkbox(&mut self.draft.center, "Centra automaticamente");
                        });
                        ui.group(|ui| {
                            ui.label(RichText::new("Colore e luminosità").strong().color(ACCENT));
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.draft.color, ColorMode::Color, "Colori");
                                ui.radio_value(&mut self.draft.color, ColorMode::Gray, "Scala di grigi");
                                ui.radio_value(&mut self.draft.color, ColorMode::Bw, "Bianco e nero");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scuro");
                                ui.add(egui::Slider::new(&mut self.draft.bright, -100..=100).show_value(false));
                                ui.label("Chiaro");
                                if ui.add(Button::new("0").min_size(Vec2::new(36.0, 30.0))).on_hover_text("Azzera chiaro/scuro").clicked() { self.draft.bright = 0; }
                            });
                        });
                        ui.label(RichText::new("Le impostazioni vengono ricordate per «Stampa subito».").weak());
                    });
                });
                ui.vertical(|ui| {
                    let (pw, ph) = self.paper;
                    let mm = |p: f32| (p / 72.0 * 25.4).round() as i32;
                    ui.label(format!("Carta: {} × {} mm", mm(pw), mm(ph)));
                    let k = (300.0 / pw).min(424.0 / ph);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(pw * k, ph * k), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
                    if let Some((_, t)) = &self.pv {
                        ui.painter().image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.painter().rect_stroke(rect, 0.0, (1.0, Color32::GRAY));
                    ui.horizontal_wrapped(|ui| {
                        for p in 0..sheets.min(40) { if ui.selectable_label(prev == p, (p + 1).to_string()).clicked() { self.prev = p; } }
                    });
                    ui.label(format!("Foglio {} di {} · {} pag. per foglio", prev + 1, sheets.max(1), nup));
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(Button::new(RichText::new(if self.pa_mode { "STAMPA TUTTO" } else { "Stampa" }).size(17.0).strong().color(Color32::WHITE)).fill(ACCENT).min_size(Vec2::new(if self.pa_mode { 180.0 } else { 140.0 }, 40.0))).clicked() { go = true; }
                if ui.add(Button::new(RichText::new("Annulla").size(17.0)).min_size(Vec2::new(115.0, 40.0))).clicked() { cancel = true; }
            });
        });
        if drv {
            let cur = if self.draft.drv_printer == self.draft.printer && !self.draft.drv_mode.is_empty() { Some(from_hex(&self.draft.drv_mode)) } else { None };
            if let Some(dm) = spool::driver_dialog(&self.draft.printer, cur.as_deref()) {
                self.draft.drv_printer = self.draft.printer.clone();
                self.draft.drv_mode = to_hex(&dm);
            } else if !cfg!(windows) { self.toast("Le impostazioni del driver sono disponibili solo su Windows".into()); }
        }
        if drv_reset { self.draft.drv_mode.clear(); }
        if go {
            let c = self.draft.copies;
            let testo = if self.pa_mode { format!("Sicuro di voler stampare {c} copie dei file aperti?") } else { format!("Sicuro di voler stampare {c} copie del file?

{}", self.docs[di].name) };
            if c > 1 && self.st.ask_copies && !confirm("Stampa", &testo) {
                // risposta "No": resta aperta la finestra di stampa
            } else {
                self.st = self.draft.clone(); self.st.save(); self.show_print = false;
                if self.pa_mode { self.pa_mode = false; let all: Vec<usize> = (0..self.docs.len()).collect(); self.print_docs(&all, true); }
                else { self.print_docs(&[di], false); }
            }
        }
        if cancel || !open { self.show_print = false; self.pa_mode = false; }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ic { Preview, Quick, All, Min, Max, Close, Pan, Sel }

fn icon_btn(ui: &mut egui::Ui, ic: Ic, tip: &str, enabled: bool) -> bool { icon_btn_a(ui, ic, tip, enabled, false) }

/// Pulsante-icona disegnato a mano (nessun font di icone necessario). `active` = strumento attivo (sfondo evidenziato).
fn icon_btn_a(ui: &mut egui::Ui, ic: Ic, tip: &str, enabled: bool, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(30.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let hov = enabled && resp.hovered();
    let p = ui.painter();
    if active { p.rect_filled(rect, 4.0, Color32::from_rgb(250, 218, 218)); }
    if hov { p.rect_filled(rect, 4.0, if ic == Ic::Close { Color32::from_rgb(232, 17, 35) } else { Color32::from_gray(205) }); }
    let col = if !enabled { Color32::from_gray(160) } else if ic == Ic::Close && hov { Color32::WHITE } else if ic == Ic::All || active { ACCENT } else { Color32::from_gray(35) };
    let st = egui::Stroke::new(1.6_f32, col);
    let c = rect.center();
    let v = |x: f32, y: f32| c + Vec2::new(x, y);
    match ic {
        Ic::Sel => { // cursore a I: seleziona testo
            p.line_segment([v(0.0, -9.0), v(0.0, 9.0)], st);
            p.line_segment([v(-4.0, -9.0), v(4.0, -9.0)], st);
            p.line_segment([v(-4.0, 9.0), v(4.0, 9.0)], st);
        }
        Ic::Pan => { // quattro frecce: sposta la pagina
            p.line_segment([v(-9.0, 0.0), v(9.0, 0.0)], st);
            p.line_segment([v(0.0, -9.0), v(0.0, 9.0)], st);
            for (dx, dy) in [(1.0_f32, 0.0_f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let (tx, ty, bx, by, px, py) = (dx * 11.0, dy * 11.0, dx * 6.0, dy * 6.0, -dy * 3.5, dx * 3.5);
                p.add(egui::Shape::convex_polygon(vec![v(tx, ty), v(bx + px, by + py), v(bx - px, by - py)], col, egui::Stroke::NONE));
            }
        }
        Ic::Min => { p.line_segment([v(-5.0, 4.0), v(5.0, 4.0)], st); }
        Ic::Max => { p.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(10.0)), 0.0, st); }
        Ic::Close => { p.line_segment([v(-5.0, -5.0), v(5.0, 5.0)], st); p.line_segment([v(-5.0, 5.0), v(5.0, -5.0)], st); }
        _ => {
            if ic == Ic::All { p.rect_stroke(egui::Rect::from_min_max(v(-2.0, -12.0), v(8.0, -4.0)), 0.0, st); } // pagine impilate
            p.rect_stroke(egui::Rect::from_min_max(v(-5.0, -9.0), v(5.0, -1.0)), 0.0, st);   // foglio in alto
            p.rect_stroke(egui::Rect::from_min_max(v(-10.0, -1.0), v(10.0, 7.0)), 2.0, st);  // corpo stampante
            let out = egui::Rect::from_min_max(v(-5.0, 4.0), v(5.0, 11.0));
            p.rect_filled(out, 0.0, BAR);
            p.rect_stroke(out, 0.0, st);                                                     // foglio in uscita
            match ic {
                Ic::Preview => { p.circle_stroke(v(8.0, 8.0), 3.5, st); p.line_segment([v(10.5, 10.5), v(13.0, 13.0)], st); }
                Ic::Quick => { p.add(egui::Shape::convex_polygon(vec![v(7.0, 3.0), v(7.0, 13.0), v(14.0, 8.0)], col, egui::Stroke::NONE)); }
                _ => {}
            }
        }
    }
    resp.on_hover_text(tip).clicked()
}

/// Scheda con la "×" di chiusura integrata. Restituisce (risposta, chiusura_richiesta).
fn tab_widget(ui: &mut egui::Ui, text: &str, selected: bool, tip: &str, num: Option<usize>) -> (egui::Response, bool) {
    let col = if selected { Color32::from_gray(20) } else { Color32::from_gray(70) };
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::TextStyle::Button.resolve(ui.style()), col);
    let (pad, xw) = (10.0_f32, 18.0_f32);
    let extra = if num.is_some() { 13.0 } else { 0.0 };
    let (full, resp) = ui.allocate_exact_size(Vec2::new(pad + galley.size().x + 8.0 + xw + 4.0, 26.0 + extra), egui::Sense::click_and_drag());
    let rect = egui::Rect::from_min_size(full.min, Vec2::new(full.width(), 26.0));
    let xr = egui::Rect::from_center_size(egui::pos2(rect.right() - 4.0 - xw / 2.0, rect.center().y), Vec2::splat(xw));
    let hover_x = resp.hovered() && ui.input(|i| i.pointer.hover_pos()).map_or(false, |p| xr.contains(p));
    let close = resp.clicked() && resp.interact_pointer_pos().map_or(false, |p| xr.contains(p));
    let p = ui.painter();
    let bg = if selected { Color32::WHITE } else if resp.hovered() { Color32::from_gray(220) } else { Color32::from_gray(205) };
    p.rect_filled(rect, egui::Rounding { nw: 6.0, ne: 6.0, sw: 0.0, se: 0.0 }, bg);
    if selected { p.rect_filled(egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), 2.0)), 0.0, ACCENT); }
    p.galley(egui::pos2(rect.left() + pad, rect.center().y - galley.size().y / 2.0), galley.clone(), col);
    if hover_x { p.rect_filled(xr, 4.0, Color32::from_rgb(232, 17, 35)); }
    let st = egui::Stroke::new(1.4_f32, if hover_x { Color32::WHITE } else { Color32::from_gray(90) });
    let c = xr.center();
    p.line_segment([c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)], st);
    p.line_segment([c + Vec2::new(-3.5, 3.5), c + Vec2::new(3.5, -3.5)], st);
    if let Some(k) = num { p.text(egui::pos2(rect.center().x, rect.bottom() + 7.0), egui::Align2::CENTER_CENTER, k.to_string(), egui::FontId::proportional(10.0), Color32::from_gray(100)); }
    (resp.on_hover_text(tip), close)
}

/// Voce di menu: chiude il menu al clic.
fn mi(ui: &mut egui::Ui, text: &str, shortcut: &str, en: bool) -> bool {
    let c = ui.add_enabled(en, Button::new(text).shortcut_text(shortcut)).clicked();
    if c { ui.close_menu(); }
    c
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        let before = self.st.clone();
        if !self.started { self.started = true; if self.st.start_max && !self.st.background { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true)); } } // si apre sempre ingrandita
        // PDF inviati da un secondo avvio del programma: si aprono qui, in nuove schede
        let msgs: Vec<Ipc> = self.ipc.as_ref().map(|r| r.try_iter().collect::<Vec<Ipc>>()).unwrap_or_default();
        if !msgs.is_empty() && !self.st.background { // in secondo piano: i file si aprono senza portare la finestra davanti
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        for m in msgs { if let Ipc::Open(p) = m { self.open(p); } }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for p in dropped { self.open(p); }
        let (mut o, mut pd, mut q, mut all) = (false, false, false, false);
        ctx.input_mut(|i| {
            o |= i.consume_key(egui::Modifiers::COMMAND, egui::Key::O);
            q |= i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
            pd |= i.consume_key(egui::Modifiers::COMMAND, egui::Key::P);
        });
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) { self.fs = !self.fs; ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fs)); }
        if let Tab::Doc(i) = self.tab {
            if !self.show_print && !ctx.wants_keyboard_input() {
                let (nx, pv, z) = ctx.input(|i| (i.key_pressed(egui::Key::PageDown) || i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::PageUp) || i.key_pressed(egui::Key::ArrowLeft), i.zoom_delta()));
                if let Some(d) = self.docs.get_mut(i) {
                    if nx && d.cur + 1 < d.sizes.len() { d.cur += 1; }
                    if pv && d.cur > 0 { d.cur -= 1; }
                    if z != 1.0 { d.zoom = (d.zoom * z).clamp(0.3, 4.0); }
                }
            }
        }

        // ---- UNICA barra = barra del titolo: [icona][File][schede ............][trascina][stampa][- o x] ----
        let (mut ctrl_w, mut next_t, mut prev_t) = (false, false, false);
        ctx.input_mut(|i| {
            prev_t = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Tab);
            next_t = i.consume_key(egui::Modifiers::COMMAND, egui::Key::Tab);
            ctrl_w = i.consume_key(egui::Modifiers::COMMAND, egui::Key::W);
        });
        let typing = ctx.wants_keyboard_input();
        let mut sel_all = false;
        let mut no_text = false;
        let (mut search_next, mut search_prev) = (false, false);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) { self.search_focus = true; }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) { self.show_cfg = true; self.cfg = self.st.clone(); }
        if !typing {
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::H) { self.pan = true; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::T) { self.pan = false; }
            });
        }
        let mut copy_req = !typing && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        if !typing { ctx.input_mut(|i| { sel_all = i.consume_key(egui::Modifiers::COMMAND, egui::Key::A); }); }
        let n_docs = self.docs.len();
        if n_docs > 0 {
            let i = if let Tab::Doc(i) = self.tab { i } else { 0 };
            if next_t { self.tab = Tab::Doc((i + 1) % n_docs); self.scroll_tab = true; }
            if prev_t { self.tab = Tab::Doc((i + n_docs - 1) % n_docs); self.scroll_tab = true; }
        }
        let has_docs = n_docs > 0;
        let cur_doc = if let Tab::Doc(i) = self.tab { Some(i) } else { None };
        let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let mut close: Option<usize> = None;
        let (mut tabs_w, mut tabs_avail) = (0.0_f32, 0.0_f32);
        let mut reorder: Option<(usize, usize)> = None;
        let (mut close_all, mut show_info, mut quit, mut fs_toggle) = (false, false, false, false);
        let want_scroll = std::mem::take(&mut self.scroll_tab);
        egui::TopBottomPanel::top("top")
            .exact_height(if self.tabs_overflow { 52.0 } else { 38.0 })
            .frame(egui::Frame::none().fill(BAR).inner_margin(egui::Margin::symmetric(6.0, 3.0)))
            .show(ctx, |ui| {
                // tutta la barra e' una maniglia: trascinando negli spazi liberi si sposta la finestra; doppio clic = ingrandisci/ripristina
                let bar = ui.interact(ui.max_rect(), egui::Id::new("titlebar_drag"), egui::Sense::click_and_drag());
                if bar.drag_started() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
                if bar.double_clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed)); }
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_btn(ui, Ic::Close, "Chiudi", true) { quit = true; }
                        if icon_btn(ui, Ic::Max, if maxed { "Ripristina" } else { "Massimizza" }, true) { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed)); }
                        if icon_btn(ui, Ic::Min, "Riduci a icona", true) { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                        ui.separator();
                        all |= icon_btn(ui, Ic::All, "STAMPA TUTTO: tutte le schede aperte, ognuna per intero", has_docs);
                        q |= icon_btn(ui, Ic::Quick, "Stampa subito: scheda corrente, ultime impostazioni (Ctrl+Maiusc+P)", cur_doc.is_some());
                        pd |= icon_btn(ui, Ic::Preview, "Stampa… con anteprima (Ctrl+P)", cur_doc.is_some());
                        ui.separator();
                        if icon_btn_a(ui, Ic::Pan, "Sposta la pagina con il mouse (H)", cur_doc.is_some(), self.pan) { self.pan = true; }
                        if icon_btn_a(ui, Ic::Sel, "Seleziona testo (T)", cur_doc.is_some(), !self.pan) { self.pan = false; }
                        let (_, drag) = ui.allocate_exact_size(Vec2::new(56.0, 28.0), egui::Sense::click_and_drag());
                        if drag.drag_started() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
                        if drag.double_clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed)); }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.image(egui::load::SizedTexture::new(self.logo.id(), Vec2::splat(24.0)));
                            ui.menu_button(RichText::new("File").strong().color(ACCENT), |ui| {
                                ui.set_min_width(260.0);
                                if mi(ui, "Apri…", "Ctrl+O", true) { o = true; }
                                if mi(ui, "Chiudi scheda", "Ctrl+W", cur_doc.is_some()) { ctrl_w = true; }
                                if mi(ui, "Chiudi tutte le schede", "", has_docs) { close_all = true; }
                                ui.separator();
                                if mi(ui, "Copia testo selezionato", "Ctrl+C", cur_doc.is_some()) { copy_req = true; }
                                if mi(ui, "Seleziona tutto il testo", "Ctrl+A", cur_doc.is_some()) { sel_all = true; }
                                ui.separator();
                                if mi(ui, "Stampa…", "Ctrl+P", cur_doc.is_some()) { pd = true; }
                                if mi(ui, "Stampa subito", "Ctrl+Maiusc+P", cur_doc.is_some()) { q = true; }
                                if mi(ui, "STAMPA TUTTO", "", has_docs) { all = true; }
                                ui.menu_button("Stampante", |ui| {
                                    for p in &self.printers {
                                        if ui.selectable_label(self.st.printer == *p, p.as_str()).clicked() { self.st.printer = p.clone(); ui.close_menu(); }
                                    }
                                });
                                ui.menu_button("Pagine per foglio", |ui| {
                                    for k in NUPS {
                                        if ui.selectable_label(self.st.nup == k, format!("{k} su 1")).clicked() { self.st.nup = k; ui.close_menu(); }
                                    }
                                });
                                ui.separator();
                                if mi(ui, "Schermo intero", "F11", true) { fs_toggle = true; }
                                ui.checkbox(&mut self.st.background, "Apri sempre in secondo piano")
                                    .on_hover_text("Aprendo altri PDF la finestra del programma non passa davanti: non perdi il focus su quello che stai usando");
                                ui.separator();
                                if mi(ui, "Impostazioni…", "Ctrl+,", true) { self.show_cfg = true; self.cfg = self.st.clone(); }
                                if mi(ui, "Info", "", true) { show_info = true; }
                                if mi(ui, "Esci", "", true) { quit = true; }
                            });
                            { // barra di scorrimento sotto le schede: compare solo quando servono, occupa 6 px e non le copre
                                let mut sc = egui::style::ScrollStyle::solid();
                                sc.bar_width = 5.0; sc.bar_inner_margin = 1.0; sc.bar_outer_margin = 0.0;
                                ui.spacing_mut().scroll = sc;
                                tabs_avail = ui.available_width();
                            }
                            egui::ScrollArea::horizontal().id_salt("tabs").drag_to_scroll(false).auto_shrink([false, false]).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    let mut rects: Vec<egui::Rect> = vec![];
                                    for i in 0..self.docs.len() {
                                        let sel = self.tab == Tab::Doc(i);
                                        let (r, x) = tab_widget(ui, &short(&self.docs[i].name, 24), sel, &self.docs[i].name, if self.tabs_overflow { Some(i + 1) } else { None });
                                        if x || r.middle_clicked() { close = Some(i); } else if r.clicked() { self.tab = Tab::Doc(i); }
                                        rects.push(r.rect);
                                        if r.drag_started() { self.tab_drag = Some(i); }
                                        if sel && want_scroll { r.scroll_to_me(Some(egui::Align::Center)); }
                                    }
                                    // trascinamento di una scheda: segno di inserimento e spostamento al rilascio
                                    if let Some(from) = self.tab_drag {
                                        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                                        if let (Some(pp), false) = (ctx.input(|i| i.pointer.hover_pos().or(i.pointer.interact_pos())), rects.is_empty()) {
                                            let ins = rects.iter().filter(|r| r.center().x < pp.x).count();
                                            let x = if ins < rects.len() { rects[ins].left() - 1.0 } else { rects[rects.len() - 1].right() + 1.0 };
                                            let top = rects[0].top();
                                            ui.painter().line_segment([egui::pos2(x, top), egui::pos2(x, top + 26.0)], egui::Stroke::new(2.5, ACCENT));
                                            if !ctx.input(|i| i.pointer.primary_down()) {
                                                let to = if ins > from { ins - 1 } else { ins };
                                                if to != from { reorder = Some((from, to)); }
                                                self.tab_drag = None;
                                            }
                                        } else { self.tab_drag = None; }
                                    }
                                    tabs_w = ui.min_rect().width();
                                });
                            });
                        });
                    });
                });
            });
        if quit { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if fs_toggle { self.fs = !self.fs; ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fs)); }
        if show_info { self.show_info = true; }
        if ctrl_w { close = cur_doc; }
        if close_all { self.docs.clear(); self.tab = Tab::Home; self.show_print = false; self.pv = None; self.pv_raw = None; }
        if let Some((from, to)) = reorder { self.move_tab(from, to); }
        if tabs_w > 0.0 { // schede troppe per la barra: compaiono i numeri sotto le schede
            let ov = tabs_w > tabs_avail + 1.0;
            if ov != self.tabs_overflow { self.tabs_overflow = ov; ctx.request_repaint(); }
        }
        if let Some(i) = close.filter(|&i| i < self.docs.len()) {
            self.docs.remove(i);
            self.tab = match self.tab { Tab::Doc(j) if j == i => Tab::Home, Tab::Doc(j) if j > i => Tab::Doc(j - 1), t => t };
            self.show_print = false; self.pv = None; self.pv_raw = None;
        }

        // ---- testo selezionabile: carica i caratteri della pagina e gestisce "seleziona tutto" ----
        if close_all || close.is_some() { self.sel = None; }
        if let Tab::Doc(i) = self.tab {
            if i < self.docs.len() {
                let cur = self.docs[i].cur;
                if self.sel.as_ref().map_or(false, |s| s.doc != i || s.page != cur) { self.sel = None; }
                let searching = !self.search.is_empty();
                if sel_all || searching || ctx.input(|i| i.pointer.primary_pressed()) { self.ensure_chars(i, cur); } // il testo si legge solo quando serve
                if searching { self.ensure_ptext(i, cur); }
                if sel_all {
                    let boxed: Vec<usize> = self.docs[i].text[&cur].iter().enumerate().filter(|(_, c)| c.r[2] - c.r[0] > 0.01 && c.r[3] - c.r[1] > 0.01).map(|(k, _)| k).collect();
                    self.sel = match (boxed.first(), boxed.last()) { (Some(&a), Some(&b)) => Some(Sel { doc: i, page: cur, a, b }), _ => { no_text = true; None } };
                }
            }
        } else { self.sel = None; }

        // ---- area documento: occupa tutto lo spazio rimasto ----
        egui::CentralPanel::default().frame(egui::Frame::none().fill(Color32::from_rgb(107, 114, 128))).show(ctx, |ui| match self.tab {
            Tab::Home => {
                egui::Frame::none().fill(Color32::WHITE).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.image(egui::load::SizedTexture::new(self.logo.id(), Vec2::splat(96.0)));
                        ui.heading("PDF Reader CP");
                        ui.label("Trascina qui un PDF oppure premi Apri.");
                        if ui.button("Apri PDF…").clicked() { o = true; }
                    });
                });
            }
            Tab::Doc(i) => {
                if i >= self.docs.len() { return; }
                let (cur, zoom, fp) = (self.docs[i].cur, self.docs[i].zoom, self.docs[i].fitpage);
                let (pw, ph) = self.docs[i].sizes[cur];
                let av = ui.available_size();
                let base = if fp { (av.x - 16.0).min((av.y - 8.0) * pw / ph) } else { av.x - 16.0 };
                let w = (base * zoom).max(50.0);
                let h = w * ph / pw;
                let sz = (w as i32, h as i32);
                if sz != self.last_size { self.last_size = sz; self.last_change = Instant::now(); }
                let bucket = (((w * ctx.pixels_per_point()) as i32).clamp(800, 3200) + 399) / 400 * 400;
                if let Some(t) = self.get_tex(ctx, i, cur, bucket) {
                    let empty: Vec<TCh> = Vec::new();
                    let loaded = self.docs[i].text.contains_key(&cur);
                    let chars = self.docs[i].text.get(&cur).unwrap_or(&empty);
                    let ptext = self.docs[i].ptext.get(&cur);
                    let sel = &mut self.sel;
                    let pan = self.pan;
                    let shit = self.search_hit;
                    let q: Vec<char> = self.search.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
                    // salto al risultato della ricerca: lo porta in vista
                    let target_y = if self.jump { shit.filter(|(p, _)| *p == cur).and_then(|(_, st)| chars.get(st)).filter(|c| c.r[3] > c.r[1]).map(|c| ((ph - c.r[3]) / ph * h - 100.0).max(0.0)) } else { None };
                    let mut sa = egui::ScrollArea::both().id_salt((i, cur)).auto_shrink([false; 2]).drag_to_scroll(pan);
                    if let Some(y) = target_y { sa = sa.vertical_scroll_offset(y); }
                    else if let Some(bottom) = self.scroll_reset.take() { sa = sa.vertical_scroll_offset(if bottom { 1.0e9 } else { 0.0 }); }
                    let out = sa.show(ui, |ui| {
                        ui.add_space(8.0);
                        let img = egui::Image::new(egui::load::SizedTexture::new(t.id(), Vec2::new(w, h))).sense(if pan { egui::Sense::click() } else { egui::Sense::click_and_drag() });
                        let resp = ui.vertical_centered(|ui| ui.add(img)).inner;
                        let to_pt = |p: egui::Pos2| ((p.x - resp.rect.min.x) / w * pw, ph - (p.y - resp.rect.min.y) / h * ph);
                        let rect_of = |c: &TCh| egui::Rect::from_min_max(
                            egui::pos2(resp.rect.min.x + c.r[0] / pw * w, resp.rect.min.y + (ph - c.r[3]) / ph * h),
                            egui::pos2(resp.rect.min.x + c.r[2] / pw * w, resp.rect.min.y + (ph - c.r[1]) / ph * h));
                        if resp.hovered() {
                            if pan { ctx.set_cursor_icon(if ui.input(|i| i.pointer.primary_down()) { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab }); }
                            else if !loaded || !chars.is_empty() { ctx.set_cursor_icon(egui::CursorIcon::Text); }
                        }
                        if !pan {
                            if resp.drag_started() {
                                if loaded && chars.is_empty() { no_text = true; }
                                *sel = resp.interact_pointer_pos().and_then(|p| { let (x, y) = to_pt(p); hit(chars, x, y) }).map(|k| Sel { doc: i, page: cur, a: k, b: k });
                            } else if resp.dragged() {
                                if let (Some(p), Some(s)) = (resp.interact_pointer_pos(), sel.as_mut()) {
                                    let (x, y) = to_pt(p);
                                    if let Some(k) = hit(chars, x, y) { s.b = k; }
                                }
                            } else if resp.clicked() { *sel = None; }
                        }
                        // risultati della ricerca (giallo; quello corrente arancione)
                        if let (false, Some(tx)) = (q.is_empty(), ptext) {
                            let cur_hit = shit.filter(|(p, _)| *p == cur).map(|(_, st)| st);
                            for st in find_all(tx, &q) {
                                let col = if Some(st) == cur_hit { Color32::from_rgba_unmultiplied(255, 140, 0, 150) } else { Color32::from_rgba_unmultiplied(255, 214, 0, 110) };
                                for c in chars.iter().skip(st).take(q.len()) { if c.r[2] - c.r[0] >= 0.01 { ui.painter().rect_filled(rect_of(c), 0.0, col); } }
                            }
                        }
                        // testo selezionato
                        if let Some(s) = sel.as_ref() {
                            let (lo, hi) = (s.a.min(s.b), s.a.max(s.b));
                            for c in chars.iter().skip(lo).take(hi + 1 - lo) { if c.r[2] - c.r[0] >= 0.01 { ui.painter().rect_filled(rect_of(c), 0.0, Color32::from_rgba_unmultiplied(51, 144, 255, 90)); } }
                        }
                        resp.context_menu(|ui| {
                            if ui.button("Copia").clicked() { copy_req = true; ui.close_menu(); }
                            if ui.button("Seleziona tutto").clicked() {
                                let mut it = chars.iter().enumerate().filter(|(_, c)| c.r[2] - c.r[0] > 0.01 && c.r[3] - c.r[1] > 0.01).map(|(k, _)| k);
                                if let Some(a) = it.next() { let b = it.last().unwrap_or(a); *sel = Some(Sel { doc: i, page: cur, a, b }); }
                                ui.close_menu();
                            }
                        });
                    });
                    if target_y.is_some() { self.jump = false; }
                    // scorrendo oltre il fondo (o l'inizio) della pagina con rotella, touchpad o mano si passa alla pagina successiva (o precedente)
                    let max_off = (out.content_size.y - out.inner_rect.height()).max(0.0);
                    let dy = ctx.input(|i| i.raw_scroll_delta.y + if pan && i.pointer.primary_down() { i.pointer.delta().y } else { 0.0 });
                    if dy != 0.0 {
                        if self.over_at.elapsed() > Duration::from_millis(450) { self.over_acc = 0.0; }
                        self.over_at = Instant::now();
                        if dy < 0.0 && self.edge.1 { self.over_acc += -dy; }
                        else if dy > 0.0 && self.edge.0 { self.over_acc -= dy; }
                        else { self.over_acc = 0.0; }
                    }
                    self.edge = (out.state.offset.y <= 0.5, out.state.offset.y >= max_off - 0.5);
                    let n_pages = self.docs[i].sizes.len();
                    if self.over_acc >= 140.0 && cur + 1 < n_pages {
                        self.docs[i].cur = cur + 1; self.over_acc = 0.0; self.edge = (true, false); ctx.request_repaint();
                    } else if self.over_acc <= -140.0 && cur > 0 {
                        self.docs[i].cur = cur - 1; self.over_acc = 0.0; self.edge = (false, true); self.scroll_reset = Some(true); ctx.request_repaint();
                    } else if self.over_acc.abs() >= 140.0 { self.over_acc = 0.0; }
                }
            }
        });

        if no_text { self.toast("Questa pagina non contiene testo selezionabile (immagine scansionata?)".into()); }
        if copy_req {
            match self.selected_text() {
                Some(t) => { ctx.copy_text(t); self.toast("Testo copiato negli appunti".into()); }
                None => self.toast("Seleziona prima del testo: trascina il mouse sulla pagina".into()),
            }
        }

        // ---- comandi pagina/zoom: barra fluttuante in basso (non toglie spazio al documento) ----
        if let Tab::Doc(i) = self.tab {
            if let Some(d) = self.docs.get_mut(i) {
                egui::Area::new(egui::Id::new("nav")).anchor(egui::Align2::CENTER_BOTTOM, [0.0, -14.0]).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("<").clicked() && d.cur > 0 { d.cur -= 1; }
                            ui.label(format!("{} / {}", d.cur + 1, d.sizes.len()));
                            if ui.button(">").clicked() && d.cur + 1 < d.sizes.len() { d.cur += 1; }
                            ui.separator();
                            if ui.button("-").clicked() { d.zoom = (d.zoom - 0.1).max(0.3); }
                            ui.label(format!("{:.0}%", d.zoom * 100.0));
                            if ui.button("+").clicked() { d.zoom = (d.zoom + 0.1).min(4.0); }
                            if ui.button(if d.fitpage { "Pagina" } else { "Larghezza" }).on_hover_text("Adatta a larghezza / pagina intera").clicked() { d.fitpage = !d.fitpage; d.zoom = 1.0; }
                            ui.separator();
                            let r = ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Cerca nel PDF…").desired_width(170.0));
                            if self.search_focus { r.request_focus(); self.search_focus = false; }
                            if r.changed() { self.search_hit = None; }
                            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if ui.input(|i| i.modifiers.shift) { search_prev = true; } else { search_next = true; }
                                r.request_focus();
                            }
                            if ui.button("Prec.").clicked() { search_prev = true; }
                            if ui.button("Succ.").clicked() { search_next = true; }
                            if !self.search.is_empty() {
                                let q: Vec<char> = self.search.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
                                match d.ptext.get(&d.cur).map(|t| find_all(t, &q).len()) {
                                    Some(0) => { ui.label("nessun risultato in pagina"); }
                                    Some(k) => { ui.label(format!("{k} in pagina")); }
                                    None => {}
                                }
                            }
                        });
                    });
                });
            }
        }
        if search_next || search_prev {
            if let Tab::Doc(i) = self.tab { if i < self.docs.len() { self.search_step(i, search_next); ctx.request_repaint(); } }
        }
        if let Some(t) = self.msg_at {
            if t.elapsed().as_secs_f32() < 4.0 {
                egui::Area::new(egui::Id::new("toast")).anchor(egui::Align2::LEFT_BOTTOM, [14.0, -14.0]).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| { ui.label(&self.msg); });
                });
                ctx.request_repaint_after(Duration::from_millis(500));
            } else { self.msg_at = None; }
        }

        // ---- azioni ----
        if o { self.pick(); }
        if let Some(i) = cur_doc.filter(|&i| i < self.docs.len()) {
            if pd { self.show_print = true; self.draft = self.st.clone(); self.prev = 0; self.pv = None; self.pv_raw = None; }
            if q { self.print_one(i); }
        }
        if all { self.print_all(); }
        // ---- stampa in corso: cerchio di caricamento finche' il lavoro non e' in coda nella stampante ----
        let mut cancel_print = false;
        if let Some(t) = &self.task {
            let testo = format!("Invio alla stampante…  {}/{} fogli · scheda {}/{}", t.done.min(t.total), t.total, (t.di + 1).min(t.ids.len()), t.ids.len());
            egui::Area::new(egui::Id::new("printing")).order(egui::Order::Foreground).anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0]).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(20.0));
                        ui.label(RichText::new(testo).strong());
                        if ui.small_button("Annulla").clicked() { cancel_print = true; }
                    });
                });
            });
        }
        if cancel_print { if let Some(t) = self.task.as_mut() { t.cancel = true; } }
        if self.task.is_some() { self.print_step(ctx); }
        if self.show_pall { self.pall_window(ctx); }
        if self.show_cfg { self.settings_window(ctx); }
        if !self.show_print { self.pa_mode = false; }
        if self.show_print { if let Tab::Doc(i) = self.tab { self.print_dialog(ctx, i); } else { self.show_print = false; } }
        if self.show_info {
            let mut open = true;
            egui::Window::new("Informazioni").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.image(egui::load::SizedTexture::new(self.logo.id(), Vec2::splat(72.0)));
                    ui.heading("PDF Reader CP");
                    ui.label("Programma opensource progettato da Peppe CP con l'aiuto della IA.");
                    ui.label(RichText::new("Versione 2.2").weak());
                    ui.add_space(8.0);
                    ui.separator();
                    ui.scope(|ui| {
                        ui.set_max_width(430.0);
                        ui.label("Se vuoi contribuire al mio lavoro ti lascio in basso il pulsante per contribuire anche con una piccola donazione simbolica visto il tempo che ti farò risparmiare per stampare grossi quantitativi di pdf ;-)");
                    });
                    ui.add_space(6.0);
                    if ui.add(Button::new(RichText::new("Dona con PayPal").strong().color(Color32::WHITE)).fill(Color32::from_rgb(0, 112, 186)).min_size(Vec2::new(220.0, 38.0)))
                        .on_hover_text(DONATE_URL).clicked() {
                        ctx.output_mut(|o| o.open_url = Some(egui::OpenUrl::new_tab(DONATE_URL)));
                    }
                    ui.add_space(4.0);
                    ui.hyperlink_to("github.com/gsepp86/Pdf-Reader-CP", GITHUB_URL);
                });
            });
            if !open { self.show_info = false; }
        }
        // contorno nero attorno alla finestra (la finestra non ha la cornice di Windows)
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("window_outline")))
            .rect_stroke(ctx.screen_rect().shrink(0.75), 0.0, egui::Stroke::new(1.5, Color32::from_gray(75)));
        if self.st != before { self.st.save(); } // stampante / "N su 1" scelti nella barra restano per "Stampa subito"
    }
}

// ---------- istanza unica: un secondo avvio passa i PDF al programma gia' aperto ----------
const IPC_ADDR: &str = "127.0.0.1:47653"; // solo localhost
const IPC_HELLO: &str = "PDFReaderCP/1";
enum Ipc { Open(PathBuf), Focus }

/// Ok(Some(listener)) = siamo la prima istanza. Err(()) = c'e' gia' un'istanza e le abbiamo passato i file: uscire.
fn single_instance(args: &[PathBuf]) -> Result<Option<std::net::TcpListener>, ()> {
    match std::net::TcpListener::bind(IPC_ADDR) {
        Ok(l) => Ok(Some(l)),
        Err(_) => if forward(args) { Err(()) } else { Ok(None) }, // porta occupata da altro: si parte comunque
    }
}

fn forward(args: &[PathBuf]) -> bool {
    let Ok(addr) = IPC_ADDR.parse::<std::net::SocketAddr>() else { return false };
    let Ok(mut s) = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(800)) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_millis(2000)));
    #[cfg(windows)]
    unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(u32::MAX); } // permette all'altra finestra di portarsi in primo piano
    let mut msg = format!("{IPC_HELLO}\n");
    if args.is_empty() { msg.push_str("FOCUS\n"); }
    for a in args { msg.push_str(&format!("OPEN\t{}\n", a.to_string_lossy())); }
    msg.push_str("END\n");
    if s.write_all(msg.as_bytes()).is_err() { return false; }
    let mut r = String::new();
    BufReader::new(s).read_line(&mut r).is_ok() && r.trim() == IPC_HELLO
}

fn serve(lst: std::net::TcpListener, tx: std::sync::mpsc::Sender<Ipc>, ctx: egui::Context) {
    std::thread::spawn(move || {
        for s in lst.incoming().flatten() {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            let mut rd = BufReader::new(&s);
            let mut line = String::new();
            if rd.read_line(&mut line).is_err() || line.trim() != IPC_HELLO { continue; }
            loop {
                line.clear();
                if rd.read_line(&mut line).unwrap_or(0) == 0 { break; }
                let l = line.trim_end_matches(['\r', '\n']);
                if l == "END" { break; }
                if l == "FOCUS" { let _ = tx.send(Ipc::Focus); }
                else if let Some(p) = l.strip_prefix("OPEN\t") {
                    let p = PathBuf::from(p);
                    if p.is_file() { let _ = tx.send(Ipc::Open(p)); }
                }
            }
            let _ = (&s).write_all(format!("{IPC_HELLO}\n").as_bytes());
            ctx.request_repaint();
        }
    });
}

fn main() -> eframe::Result<()> {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(|a| { let p = PathBuf::from(a); std::path::absolute(&p).unwrap_or(p) }).collect();
    // se il programma e' gia' aperto, i PDF si aprono li' (in nuove schede) e questo processo termina
    let listener = match single_instance(&args) { Ok(l) => l, Err(()) => return Ok(()) };
    let cfg0 = Settings::load();
    let icon = eframe::icon_data::from_png_bytes(ICON).expect("icona");
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 760.0]).with_min_inner_size([800.0, 500.0]).with_maximized(cfg0.start_max).with_active(!cfg0.background).with_decorations(false).with_icon(icon).with_title("PDF Reader CP"),
        ..Default::default()
    };
    eframe::run_native("PDF Reader CP", opts, Box::new(move |cc| Ok(Box::new(App::new(cc, listener, args)))))
}
