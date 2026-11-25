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
    /// Fenêtre maximisée ordinaire (avec barre de titre): ce n'est pas une application plein écran.
    pub(crate) normal_max: bool,
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
    /// Secondes depuis que la fenêtre n'est plus énumérée (0 = présente). Elle garde sa décoration pendant `GRACE`.
    pub(crate) missing: f32,
}

/// Une fenêtre qui disparaît un instant (rafraîchissement, bascule de bureau) garde son ancre ce temps-là.
pub(crate) const GRACE: f32 = 1.5;

impl Anchor {
    pub(crate) fn new() -> Anchor {
        Anchor { l: 0.0, t: 0.0, r: 0.0, pile: Vec::new(), z: None, px: f32::NAN, py: f32::NAN, still: 0.0, seed: 0, kind: 0, variant: 0, left: false, hh: 60.0, deco: 0.0, missing: 0.0 }
    }
    /// Vrai si la fenêtre est actuellement visible (sinon: ni dessin ni neige, en attendant son retour).
    pub(crate) fn present(&self) -> bool {
        self.missing == 0.0
    }
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

/// Met à jour les ancres de fenêtres (clé > 0) après une énumération: les absentes sont gardées `GRACE` secondes.
pub(crate) fn expire_anchors(anchors: &mut HashMap<isize, Anchor>, keep: &[isize], dt: f32) {
    for (k, a) in anchors.iter_mut().filter(|(k, _)| **k > 0) {
        a.missing = if keep.contains(k) { 0.0 } else { (a.missing + dt).max(f32::MIN_POSITIVE) };
    }
    anchors.retain(|k, a| *k <= 0 || a.missing <= GRACE);
}

/// Une fenêtre qui recouvre tout l'écran masque l'overlay (jeu, vidéo, F11), sauf une fenêtre maximisée ordinaire:
/// avec une barre des tâches en masquage automatique, elle recouvre aussi tout l'écran sans être un plein écran.
pub(crate) fn hides_overlay(win: (f32, f32, f32, f32), screen: (f32, f32, f32, f32), normal_max: bool) -> bool {
    !normal_max && win.0 <= screen.0 + 1.0 && win.1 <= screen.1 + 1.0 && win.2 >= screen.2 - 1.0 && win.3 >= screen.3 - 1.0
}

/// Fenêtre maximisée avec sa barre de titre (≠ F11 ou vidéo plein écran, qui n'en ont plus).
pub(crate) fn is_normal_maximized(hwnd: HWND) -> bool {
    unsafe { IsZoomed(hwnd).as_bool() && (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0) == WS_CAPTION.0 }
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
    let normal_max = is_normal_maximized(hwnd);
    ENUM_WINS.with(|w| w.borrow_mut().push(WinInfo { hwnd: hwnd.0 as isize, l, t, r, b, snowy, normal_max }));
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
        // Une fenêtre maximisée ordinaire n'est pas un plein écran (F11 / vidéo plein écran n'ont plus de barre de titre).
        if !IsWindowVisible(fg).as_bool() || is_normal_maximized(fg) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn book(keys: &[isize]) -> HashMap<isize, Anchor> {
        keys.iter().map(|k| (*k, Anchor::new())).collect()
    }

    #[test]
    fn anchor_survives_a_short_absence_then_expires() {
        let mut m = book(&[0, 5]);
        expire_anchors(&mut m, &[0], 0.5);
        assert!(m.contains_key(&5) && !m[&5].present());
        expire_anchors(&mut m, &[0, 5], 0.1);
        assert!(m[&5].present(), "revenue: elle doit redevenir présente");
        expire_anchors(&mut m, &[0], 1.0);
        expire_anchors(&mut m, &[0], 1.0);
        assert!(!m.contains_key(&5), "absente trop longtemps: supprimée");
    }

    #[test]
    fn taskbar_anchors_are_never_expired_by_the_window_grace() {
        let mut m = book(&[0, -7]);
        for _ in 0..10 {
            expire_anchors(&mut m, &[], 1.0);
        }
        assert!(m.contains_key(&0) && m.contains_key(&-7));
    }

    const SCREEN: (f32, f32, f32, f32) = (0.0, 0.0, 1920.0, 1080.0);

    #[test]
    fn fullscreen_video_hides_the_overlay() {
        assert!(hides_overlay((0.0, 0.0, 1920.0, 1080.0), SCREEN, false));
        assert!(hides_overlay((-8.0, -8.0, 1928.0, 1088.0), SCREEN, false));
    }

    /// Régression: fenêtre maximisée ordinaire + barre des tâches en masquage automatique = pas un plein écran.
    #[test]
    fn normal_maximized_window_does_not_hide_the_overlay() {
        assert!(!hides_overlay((0.0, 0.0, 1920.0, 1080.0), SCREEN, true));
    }

    #[test]
    fn window_smaller_than_the_screen_does_not_hide_the_overlay() {
        assert!(!hides_overlay((0.0, 0.0, 1920.0, 1040.0), SCREEN, false));
        assert!(!hides_overlay((100.0, 0.0, 1920.0, 1080.0), SCREEN, false));
    }
}
