//! Un overlay par écran: fenêtre transparente, chaîne d'affichage et visuel DirectComposition.
use crate::prelude::*;

pub(crate) struct Surface {
    pub(crate) hwnd: HWND,
    pub(crate) swap: IDXGISwapChain1,
    /// Rectangle de l'écran dans le bureau virtuel: gauche, haut, droite, bas.
    pub(crate) rect: (f32, f32, f32, f32),
    /// Une application plein écran occupe cet écran: l'overlay y est masqué.
    pub(crate) hidden: bool,
    pub(crate) _target: IDCompositionTarget,
    pub(crate) _visual: IDCompositionVisual,
}

unsafe extern "system" fn monitor_cb(hmon: HMONITOR, _: HDC, r: *mut RECT, lp: LPARAM) -> BOOL {
    let list = &mut *(lp.0 as *mut Vec<(RECT, bool)>);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let primary = GetMonitorInfoW(hmon, &mut mi).as_bool() && mi.dwFlags & 1 != 0;
    list.push((*r, primary));
    BOOL(1)
}

/// Écrans actifs (rectangle physique, principal?).
fn monitors() -> Vec<(RECT, bool)> {
    let mut list: Vec<(RECT, bool)> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(monitor_cb), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

impl App {
    /// (Re)crée un overlay par écran, puis replace la neige sur le nouveau bureau virtuel.
    pub(crate) fn rebuild_surfaces(&mut self) -> Result<()> {
        unsafe {
            let old = std::mem::take(&mut self.surfaces);
            for s in &old {
                let _ = DestroyWindow(s.hwnd);
            }
            drop(old);

            let mut mons = monitors();
            if mons.is_empty() {
                mons.push((RECT { left: 0, top: 0, right: GetSystemMetrics(SM_CXSCREEN), bottom: GetSystemMetrics(SM_CYSCREEN) }, true));
            }
            let class = w!("SneakyXmasOverlay");
            let (mut l, mut t, mut r, mut b) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for (m, _) in &mons {
                l = l.min(m.left);
                t = t.min(m.top);
                r = r.max(m.right);
                b = b.max(m.bottom);
            }
            let prim = mons.iter().find(|m| m.1).unwrap_or(&mons[0]).0;
            for (m, _) in &mons {
                let (w, h) = (m.right - m.left, m.bottom - m.top);
                let hwnd = CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_NOREDIRECTIONBITMAP,
                    class,
                    w!("sneakyXmas"),
                    WS_POPUP,
                    m.left,
                    m.top,
                    w,
                    h,
                    None,
                    None,
                    self.hinst,
                    None,
                )?;
                let desc = DXGI_SWAP_CHAIN_DESC1 {
                    Width: w as u32,
                    Height: h as u32,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    Stereo: false.into(),
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_STRETCH,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                    AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                    Flags: 0,
                };
                let swap = self.dxgi_factory.CreateSwapChainForComposition(&self.dxgi_dev, &desc, None)?;
                let target = self.dcomp.CreateTargetForHwnd(hwnd, true)?;
                let visual = self.dcomp.CreateVisual()?;
                visual.SetContent(&swap)?;
                target.SetRoot(&visual)?;
                self.surfaces.push(Surface {
                    hwnd,
                    swap,
                    rect: (m.left as f32, m.top as f32, m.right as f32, m.bottom as f32),
                    hidden: false,
                    _target: target,
                    _visual: visual,
                });
            }
            self.dcomp.Commit()?;

            self.vx = l as f32;
            self.vy = t as f32;
            self.w = (r - l) as f32;
            self.h = (b - t) as f32;
            self.pm = (prim.left as f32, prim.top as f32, prim.right as f32, prim.bottom as f32);
            self.density = (mons.iter().map(|m| (m.0.right - m.0.left) as f32).sum::<f32>() / (prim.right - prim.left).max(1) as f32).max(1.0);
            // La neige repart sur tout le bureau virtuel; les ancres sont recalculées au prochain tour.
            let (vx, vy, w, h) = (self.vx, self.vy, self.w, self.h);
            for f in &mut self.flakes {
                f.x = vx + self.rng.next() * w;
                f.y = vy + self.rng.next() * h;
            }
            self.anchors.clear();
            self.wins.clear();
            self.pile_cache.borrow_mut().clear();
            self.santa_x = None;
            self.sleigh = None;
            self.apply_visibility();
        }
        Ok(())
    }

    /// Vrai si la disposition réelle des écrans diffère de celle des overlays existants.
    /// Windows diffuse `WM_SETTINGCHANGE` pour tout et n'importe quoi (thème, rafraîchissement d'une fenêtre...):
    /// on ne reconstruit les overlays (et on ne perd la neige) que si les écrans ont vraiment changé.
    pub(crate) fn layout_changed(&self) -> bool {
        let mut now: Vec<[i32; 4]> = monitors().iter().map(|(r, _)| [r.left, r.top, r.right, r.bottom]).collect();
        let mut cur: Vec<[i32; 4]> = self.surfaces.iter().map(|s| [s.rect.0 as i32, s.rect.1 as i32, s.rect.2 as i32, s.rect.3 as i32]).collect();
        now.sort();
        cur.sort();
        now != cur
    }

    /// Détruit tous les overlays (après un échec de reconstruction: on réessaiera au prochain tour).
    pub(crate) fn drop_surfaces(&mut self) {
        for s in std::mem::take(&mut self.surfaces) {
            unsafe {
                let _ = DestroyWindow(s.hwnd);
            }
        }
    }

    /// Écran (rectangle) qui contient le point, dans le bureau virtuel.
    pub(crate) fn monitor_at(&self, x: f32, y: f32) -> Option<(f32, f32, f32, f32)> {
        self.surfaces.iter().map(|s| s.rect).find(|r| x >= r.0 && x < r.2 && y >= r.1 && y < r.3)
    }
}
