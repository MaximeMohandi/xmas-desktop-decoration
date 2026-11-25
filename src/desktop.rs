//! Le bureau Windows: fenêtres visibles, ancres (barre des tâches, fenêtres) et détection du plein écran.
use crate::prelude::*;

pub(crate) const COL_W: f32 = 2.0; // largeur d'une colonne de neige accumulée

/// Fenêtre visible de premier niveau (pour l'ordre d'affichage et le recouvrement).
pub(crate) struct WinInfo {
    pub(crate) hwnd: isize,
    pub(crate) l: f32,
    pub(crate) t: f32,
    pub(crate) r: f32,
    pub(crate) b: f32,
    pub(crate) snowy: bool,
}

/// Ancre: un bord supérieur sur lequel la neige s'accumule (barre des tâches ou fenêtre).
pub(crate) struct Anchor {
    pub(crate) l: f32,
    pub(crate) t: f32,
    pub(crate) r: f32,
    pub(crate) pile: Vec<f32>,
    /// Position dans l'ordre d'affichage (0 = au-dessus de tout). None: jamais recouverte.
    pub(crate) z: Option<usize>,
    /// Position à la fin de l'image précédente (NaN = inconnue), pour détecter un déplacement.
    pub(crate) px: f32,
    pub(crate) py: f32,
    /// Secondes écoulées depuis le dernier déplacement (les décorations réapparaissent ensuite).
    pub(crate) still: f32,
    /// Graine stable pour varier les décorations d'une fenêtre à l'autre.
    pub(crate) seed: u32,
    /// Décoration de la fenêtre, tirée une seule fois à sa création: indice (scènes puis accents), variante,
    /// coin (gauche ou droite) et hauteur. `deco` = visibilité (fondu 0..1).
    pub(crate) kind: usize,
    pub(crate) variant: u32,
    pub(crate) left: bool,
    pub(crate) hh: f32,
    pub(crate) deco: f32,
}

impl Anchor {
    pub(crate) fn set_geometry(&mut self, l: f32, t: f32, r: f32) {
        self.l = l;
        self.t = t;
        self.r = r;
        let n = ((r - l) / COL_W) as usize + 2;
        if self.pile.len() != n {
            self.pile.resize(n, 0.0);
        }
    }
    pub(crate) fn col(&self, x: f32) -> Option<usize> {
        if x >= self.l && x < self.r {
            Some(((x - self.l) / COL_W) as usize).filter(|c| *c < self.pile.len())
        } else {
            None
        }
    }
}

/// Vrai si une fenêtre située au-dessus (indice < z) recouvre le point.
pub(crate) fn covered(wins: &[WinInfo], z: Option<usize>, x: f32, y: f32) -> bool {
    let Some(z) = z else { return false };
    wins.iter().take(z).any(|w| x >= w.l && x < w.r && y >= w.t && y < w.b)
}

thread_local! { pub(crate) static TASKBAR_Z: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
thread_local! { pub(crate) static SEC_TB_Z: RefCell<Vec<(isize, usize)>> = const { RefCell::new(Vec::new()) }; }
thread_local! { pub(crate) static ENUM_WINS: RefCell<Vec<WinInfo>> = const { RefCell::new(Vec::new()) }; }

pub(crate) fn class_name(hwnd: HWND) -> String {
    let mut cls = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut cls) } as usize;
    String::from_utf16_lossy(&cls[..n])
}

/// Rectangle visible réel (sans les bordures invisibles de Windows 10/11).
pub(crate) fn frame_bounds(hwnd: HWND) -> Option<(f32, f32, f32, f32)> {
    unsafe {
        let mut r = RECT::default();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_ok();
        if !ok && GetWindowRect(hwnd, &mut r).is_err() {
            return None;
        }
        Some((r.left as f32, r.top as f32, r.right as f32, r.bottom as f32))
    }
}

pub(crate) unsafe extern "system" fn enum_cb(hwnd: HWND, _: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return BOOL(1);
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0 {
        return BOOL(1);
    }
    let cls = class_name(hwnd);
    if cls == "Shell_SecondaryTrayWnd" {
        // Barres des tâches des autres écrans: leur position dans l'ordre d'affichage.
        let n = ENUM_WINS.with(|w| w.borrow().len());
        SEC_TB_Z.with(|z| z.borrow_mut().push((hwnd.0 as isize, n)));
        return BOOL(1);
    }
    if cls == "Shell_TrayWnd" {
        // Position de la barre des tâches dans l'ordre d'affichage (pour son recouvrement).
        let n = ENUM_WINS.with(|w| w.borrow().len());
        TASKBAR_Z.with(|z| z.set(Some(n)));
        return BOOL(1);
    }
    if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
        return BOOL(1);
    }
    if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_SecondaryTrayWnd" | "SneakyXmasOverlay") {
        return BOOL(1);
    }
    let Some((l, t, r, b)) = frame_bounds(hwnd) else { return BOOL(1) };
    if r - l < 50.0 || b - t < 30.0 {
        return BOOL(1);
    }
    let snowy = !IsZoomed(hwnd).as_bool(); // toutes les fenêtres non maximisées
    ENUM_WINS.with(|w| w.borrow_mut().push(WinInfo { hwnd: hwnd.0 as isize, l, t, r, b, snowy }));
    BOOL(1)
}

/// Vrai si la fenêtre au premier plan (hors Overlay, bureau, barre des tâches)
/// couvre entièrement son écran sans être maximisée: jeu, film, F11 navigateur.
/// Note: SHQueryUserNotificationState ne convient pas seul car il détecte l'Overlay
/// lui-même (il couvre tout l'écran) comme une application plein écran.
pub(crate) fn foreground_covers_monitor(own: HWND) -> Option<RECT> {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg == own {
            return None;
        }
        let mut cls = [0u16; 64];
        let n = GetClassNameW(fg, &mut cls) as usize;
        let cls = String::from_utf16_lossy(&cls[..n]);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "SneakyXmasOverlay"
        ) {
            return None;
        }
        // Pas d'exclusion des fenêtres maximisées: F11 / vidéo plein écran le restent pour Windows.
        if !IsWindowVisible(fg).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return None;
        }
        let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return None;
        }
        let m = mi.rcMonitor;
        (r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom).then_some(m)
    }
}


/// Écran occupé par une application plein écran (jeu, film, F11), s'il y en a une.
/// `RECT::default()` (tout à zéro) signifie « tous les écrans » (mode présentation sans fenêtre identifiable).
pub(crate) fn fullscreen_monitor(own: HWND) -> Option<RECT> {
    if let Some(m) = foreground_covers_monitor(own) {
        return Some(m);
    }
    unsafe {
        if matches!(SHQueryUserNotificationState(), Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)) {
            let fg = GetForegroundWindow();
            if !fg.0.is_null() {
                let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
                let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                if GetMonitorInfoW(mon, &mut mi).as_bool() {
                    return Some(mi.rcMonitor);
                }
            }
            return Some(RECT::default());
        }
    }
    None
}
