//! Dessin d'une image de l'Overlay.
use crate::prelude::*;

impl App {
    pub(crate) unsafe fn dot(&self, x: f32, y: f32, rx: f32, ry: f32, c: D2D1_COLOR_F) {
        self.pen.SetColor(&c);
        self.ctx.FillEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x, y }, radiusX: rx, radiusY: ry }, &self.pen);
    }
    pub(crate) unsafe fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: D2D1_COLOR_F) {
        self.pen.SetColor(&c);
        self.ctx.DrawLine(D2D_POINT_2F { x: x0, y: y0 }, D2D_POINT_2F { x: x1, y: y1 }, &self.pen, w, &self.round);
    }
    pub(crate) unsafe fn place(&self, x: f32, y: f32, sx: f32, sy: f32) {
        self.ctx.SetTransform(&windows::Foundation::Numerics::Matrix3x2 { M11: sx, M12: 0.0, M21: 0.0, M22: sy, M31: x - self.origin.get().0, M32: y - self.origin.get().1 });
    }
    pub(crate) unsafe fn reset(&self) {
        self.place(0.0, 0.0, 1.0, 1.0);
    }

    pub(crate) unsafe fn place_rot(&self, x: f32, y: f32, sx: f32, sy: f32, rot: f32) {
        let (c, s) = (rot.cos(), rot.sin());
        self.ctx.SetTransform(&windows::Foundation::Numerics::Matrix3x2 { M11: sx * c, M12: sx * s, M21: -sy * s, M22: sy * c, M31: x - self.origin.get().0, M32: y - self.origin.get().1 });
    }

    /// Dessine une image (coin haut-gauche en (x, y), échelle s) dans l'espace courant.
    pub(crate) unsafe fn spr(&self, b: &ID2D1Bitmap1, x: f32, y: f32, s: f32) {
        self.spr_op(b, x, y, s, 1.0);
    }

    pub(crate) unsafe fn spr_op(&self, b: &ID2D1Bitmap1, x: f32, y: f32, s: f32, op: f32) {
        let sz = b.GetPixelSize();
        let r = D2D_RECT_F { left: x, top: y, right: x + sz.width as f32 * s, bottom: y + sz.height as f32 * s };
        self.ctx.DrawBitmap(b, Some(&r), op, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, None, None);
    }

    pub(crate) unsafe fn bmp(&self, src: Src, variant: u32) -> &ID2D1Bitmap1 {
        match src {
            Ico(i) => &self.spr.ico[(i - 1).min(self.spr.ico.len() - 1)],
            Chr(i) => &self.spr.chars[i % self.spr.chars.len()],
            Tree(i) => &self.spr.trees[i % 3],
            Snowman => &self.spr.chars[[0usize, 1, 2, 6, 7, 8][variant as usize % 6]],
        }
    }

    /// Sprite debout de hauteur `h` px, centre-bas en (x, base_y); se balance et « respire » légèrement.
    pub(crate) unsafe fn draw_h(&self, b: &ID2D1Bitmap1, x: f32, base_y: f32, h: f32, op: f32) {
        let sz = b.GetPixelSize();
        let k = h / sz.height as f32;
        let phase = x * 0.013 + h * 0.05;
        let sway = (self.t * 0.9 + phase).sin() * 0.012;
        let breathe = 1.0 + (self.t * 1.6 + phase).sin() * 0.012;
        self.place_rot(x, base_y, k, k * breathe, sway);
        self.spr_op(b, -(sz.width as f32) / 2.0, -(sz.height as f32), 1.0, op);
        self.reset();
    }

    /// Lueur vacillante (lanterne, bougie).
    pub(crate) unsafe fn glow(&self, x: f32, y: f32, r: f32, op: f32) {
        let flick = 0.5 + 0.5 * (self.t * 7.0 + x).sin() * (self.t * 3.1).sin();
        self.dot(x, y, r * 1.8, r * 1.8, D2D1_COLOR_F { r: 1.0, g: 0.72, b: 0.3, a: (0.05 + 0.04 * flick) * op });
        self.dot(x, y, r, r, D2D1_COLOR_F { r: 1.0, g: 0.8, b: 0.4, a: (0.10 + 0.06 * flick) * op });
    }

    /// Dessine une scène: pièce principale au centre (devant), accessoires de chaque côté, posés sur une congère.
    /// `hh` = hauteur de la scène en px, `mirror` inverse la composition. Renvoie l'étendue horizontale.
    pub(crate) unsafe fn draw_scene(&self, pieces: &[Piece], variant: u32, x: f32, base_y: f32, hh: f32, mirror: bool, op: f32, z: Option<usize>, cover_y: f32) -> (f32, f32) {
        let sgn = if mirror { -1.0 } else { 1.0 };
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut placed: Vec<(Src, f32, f32)> = Vec::new();
        for p in pieces {
            let sz = self.bmp(p.src, variant).GetPixelSize();
            let h = p.h * hh;
            let w = sz.width as f32 * h / sz.height as f32;
            let cx = x + sgn * p.dx * hh;
            lo = lo.min(cx - w / 2.0);
            hi = hi.max(cx + w / 2.0);
            placed.push((p.src, cx, h));
        }
        // Congère sous la scène: un décor posé sur la neige est ancré, pas flottant.
        let mid = (lo + hi) / 2.0;
        if !covered(&self.wins, z, mid, cover_y) {
            let pile = &self.spr.ico[30];
            let psz = pile.GetPixelSize();
            let sx = (hi - lo) * 1.3 / psz.width as f32;
            self.place(mid, base_y + 4.0, sx, sx * 0.55);
            self.spr_op(pile, -(psz.width as f32) / 2.0, -(psz.height as f32), 1.0, op);
            self.reset();
        }
        for (src, cx, h) in placed.iter().rev() {
            if covered(&self.wins, z, *cx, cover_y) {
                continue;
            }
            self.draw_h(self.bmp(*src, variant), *cx, base_y, *h, op);
            match src {
                Ico(37) => self.glow(*cx, base_y - h * 0.60, h * 0.45, op),
                Ico(21) => self.glow(*cx, base_y - h * 0.62, h * 0.40, op),
                _ => {}
            }
        }
        (lo, hi)
    }

    /// Chemin lisse: remplit de la base jusqu'à la courbe passant par les points (courbes quadratiques aux milieux).
    pub(crate) unsafe fn smooth_path(&self, pts: &[(f32, f32)], base: f32) -> Option<ID2D1PathGeometry> {
        if pts.len() < 2 {
            return None;
        }
        let g = self.factory.CreatePathGeometry().ok()?;
        let sink = g.Open().ok()?;
        let p = |x: f32, y: f32| D2D_POINT_2F { x, y };
        sink.BeginFigure(p(pts[0].0, base), D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(p(pts[0].0, pts[0].1));
        for i in 1..pts.len() {
            let mid = ((pts[i - 1].0 + pts[i].0) / 2.0, (pts[i - 1].1 + pts[i].1) / 2.0);
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT { point1: p(pts[i - 1].0, pts[i - 1].1), point2: p(mid.0, mid.1) });
        }
        let last = pts[pts.len() - 1];
        sink.AddLine(p(last.0, last.1));
        sink.AddLine(p(last.0, base));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close().ok()?;
        g.cast().ok()
    }

    /// Tas de neige: forme arrondie qui s'affine aux extrémités, avec une ombre bleutée en bas.
    pub(crate) unsafe fn draw_pile(&self, key: isize, a: &Anchor) {
        let step = 3usize;
        let base = a.t + 1.0;
        // Tronçons visibles du tas (interrompus là où une fenêtre au-dessus le recouvre).
        let mut runs: Vec<Vec<(f32, f32)>> = vec![Vec::new()];
        let mut i = 0;
        while i < a.pile.len() {
            let x = a.l + i as f32 * COL_W;
            if x > a.r {
                break;
            }
            // Les extrémités s'affinent (coins de fenêtre arrondis).
            let edge = ((x - a.l).min(a.r - x) / 16.0).clamp(0.0, 1.0);
            let taper = edge * edge * (3.0 - 2.0 * edge);
            if covered(&self.wins, a.z, x, a.t - 1.0) {
                if !runs.last().is_some_and(|r| r.is_empty()) {
                    runs.push(Vec::new());
                }
            } else {
                runs.last_mut().unwrap().push((x, a.pile[i] * taper));
            }
            i += step;
        }
        runs.retain(|r| r.len() >= 2 && r.iter().any(|p| p.1 > 0.5));
        // Empreinte de la forme (hauteurs au quart de pixel près): la géométrie n'est refaite que si elle change.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ a.t.to_bits() as u64;
        for r in &runs {
            for p in r {
                hash = (hash ^ p.0.to_bits() as u64).wrapping_mul(0x100_0000_01b3);
                hash = (hash ^ (p.1 * 2.0) as u64).wrapping_mul(0x100_0000_01b3);
            }
            hash = (hash ^ 0xff).wrapping_mul(0x100_0000_01b3);
        }
        let mut cache = self.pile_cache.borrow_mut();
        let stale = match cache.get(&key) {
            None => true,
            Some(c) => c.0 != hash && self.t - c.1 >= 0.2,
        };
        if stale {
            let geos = runs
                .iter()
                .filter_map(|r| {
                    let top: Vec<(f32, f32)> = r.iter().map(|p| (p.0, a.t - p.1)).collect();
                    let shade: Vec<(f32, f32)> = r.iter().map(|p| (p.0, a.t - p.1 * 0.38)).collect();
                    Some((self.smooth_path(&top, base)?, self.smooth_path(&shade, base)?))
                })
                .collect();
            cache.insert(key, (hash, self.t, geos));
        }
        for (top, shade) in &cache[&key].2 {
            self.pen.SetColor(&D2D1_COLOR_F { r: 0.97, g: 0.98, b: 1.0, a: 0.98 });
            self.ctx.FillGeometry(top, &self.pen, None);
            self.pen.SetColor(&D2D1_COLOR_F { r: 0.78, g: 0.86, b: 0.96, a: 0.9 });
            self.ctx.FillGeometry(shade, &self.pen, None);
        }
        if cache.len() > self.anchors.len() + 4 {
            let live: Vec<isize> = self.anchors.keys().copied().collect();
            cache.retain(|k, _| live.contains(k));
        }
    }

    pub(crate) unsafe fn draw_sleigh(&self, x: f32, dir: f32) {
        let y = self.sleigh_y();
        let k = SLEIGH_K;
        let tilt = (self.t * 2.2 + 0.6).sin() * 0.025;
        let step = (self.t * 9.0) as usize;
        let seq = [0usize, 1, 2, 3, 2, 1];
        let ds = DEER_DS;
        let b1 = (self.t * 9.0).sin() * 3.0;
        let b2 = (self.t * 9.0 + 1.3).sin() * 3.0;
        let (o1, o2) = ((610.0, 100.0 + b1), (840.0, 84.0 + b2));
        let bell = (245.0 * ds, 182.0 * ds); // collier du renne, en pixels du traîneau
        let hand = (516.0, 222.0);

        self.place_rot(x, y, dir * k, k, tilt);
        self.spr(&self.spr.sleigh, 0.0, 0.0, 1.0);
        let rein = |from: (f32, f32), to: (f32, f32)| {
            let c = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0 + 22.0);
            let mut prev = from;
            for j in 1..=14 {
                let u = j as f32 / 14.0;
                let p = ((1.0 - u) * (1.0 - u) * from.0 + 2.0 * u * (1.0 - u) * c.0 + u * u * to.0,
                         (1.0 - u) * (1.0 - u) * from.1 + 2.0 * u * (1.0 - u) * c.1 + u * u * to.1);
                self.line(prev.0, prev.1, p.0, p.1, 4.5, col(112, 66, 40));
                prev = p;
            }
        };
        rein(hand, (o2.0 + bell.0, o2.1 + bell.1));
        self.spr(&self.spr.deer[seq[(step + 2) % 6]], o2.0, o2.1, ds);
        rein(hand, (o1.0 + bell.0, o1.1 + bell.1));
        self.spr(&self.spr.deer[seq[step % 6]], o1.0, o1.1, ds);
        self.reset();
    }

    pub(crate) unsafe fn draw_santa(&self, x: f32, y: f32) {
        // Ombre douce au sol.
        self.dot(x, y + 1.0, 20.0, 3.5, D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.05, a: 0.18 });
        if self.santa_pause > 0.0 {
            let b = &self.spr.santa_wave;
            let sz = b.GetPixelSize();
            let k = SANTA_K * 0.80;
            self.place_rot(x, y, k, k, (self.t * 3.0).sin() * 0.02);
            self.spr(b, -(sz.width as f32) * 0.5, -(sz.height as f32), 1.0);
        } else {
            let idx = ((self.santa_dist / SANTA_STRIDE) * 8.0) as usize % 8;
            let b = &self.spr.santa_walk[idx];
            let sz = b.GetPixelSize();
            self.place(x, y, SANTA_K, SANTA_K);
            self.spr(b, -(sz.width as f32) * 0.5, -(sz.height as f32), 1.0);
        }
        self.reset();
    }

    /// Dessine une image sur chaque écran visible.
    pub(crate) fn draw(&self) -> Result<()> {
        for s in &self.surfaces {
            if !s.hidden {
                unsafe { self.draw_surface(s)? };
            }
        }
        Ok(())
    }

    /// Dessine le monde vu par un écran: le monde est translaté de -origine, et on saute ce qui est hors champ.
    unsafe fn draw_surface(&self, s: &Surface) -> Result<()> {
        let (vl, vt, vr, vb) = s.rect;
        self.origin.set((vl, vt));
        let near = |a: &Anchor| a.r > vl - 200.0 && a.l < vr + 200.0 && a.t > vt - 150.0 && a.t < vb + 300.0;
        let surface: IDXGISurface = s.swap.GetBuffer(0)?;
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            colorContext: std::mem::ManuallyDrop::new(None),
        };
        let bitmap = self.ctx.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
        self.ctx.SetTarget(&bitmap);
        self.ctx.BeginDraw();
        self.reset();
        self.ctx.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
        // La neige accumulée d'abord: les décors et personnages se posent par-dessus, jamais enfouis.
        for (key, a) in self.anchors.iter().filter(|(_, a)| near(a)) {
            self.draw_pile(*key, a);
        }
        // Barres des tâches (de chaque écran): décor fixe; le père Noël marche sur celle de l'écran principal.
        if self.cfg.decor {
            for (_, a) in self.anchors.iter().filter(|(k, a)| **k <= 0 && near(a)) {
                let width = a.r - a.l;
                self.draw_scene(&TB_LEFT, 0, a.l + width * 0.11, a.t + 2.0, 96.0, false, 1.0, a.z, a.t + 2.0);
                self.draw_scene(&TB_RIGHT, 0, a.l + width * 0.82, a.t + 2.0, 74.0, true, 1.0, a.z, a.t + 2.0);
            }
        }
        if let (Some(a), Some(sx)) = (self.anchors.get(&0), self.santa_x) {
            if near(a) && sx > vl - 120.0 && sx < vr + 120.0 && !covered(&self.wins, a.z, sx, a.t + 2.0) {
                self.draw_santa(sx, a.t - 1.0);
            }
        }
        // Fenêtres: chacune garde la décoration tirée à sa création (scène ou accent), quand elle est immobile.
        for (key, a) in self.anchors.iter().filter(|(_, a)| near(a)) {
            let width = a.r - a.l;
            if !self.cfg.decor || *key <= 0 || a.deco < 0.02 || width < 150.0 {
                continue;
            }
            let hh = a.hh;
            if a.kind < SCENES.len() {
                // Fenêtre étroite: seule la pièce principale de la scène.
                let pieces = if width >= 300.0 { SCENES[a.kind] } else { &SCENES[a.kind][..1] };
                let cx = if a.left { a.l + 16.0 + hh } else { a.r - 16.0 - hh };
                self.draw_scene(pieces, a.variant, cx, a.t + 1.0, hh, !a.left, a.deco, a.z, a.t - 1.0);
            } else {
                let (src, rel) = ACCENTS[a.kind - SCENES.len()];
                let cx = if a.left { a.l + 40.0 } else { a.r - 40.0 };
                self.draw_scene(&[pc(src, rel, 0.0)], a.variant, cx, a.t + 1.0, hh, false, a.deco, a.z, a.t - 1.0);
            }
        }
        // Un flocon n'est visible que sur le bureau: jamais devant une fenêtre (ni la barre des tâches).
        let nw = self.wins.len();
        for f in self.flakes.iter().take(self.flake_count()) {
            if f.x < vl - 6.0 || f.x > vr + 6.0 || f.y < vt - 6.0 || f.y > vb + 6.0 || covered(&self.wins, Some(nw), f.x, f.y) {
                continue;
            }
            let e = D2D1_ELLIPSE { point: D2D_POINT_2F { x: f.x, y: f.y }, radiusX: f.r, radiusY: f.r };
            self.ctx.FillEllipse(&e, &self.brush);
        }
        for sp in &self.sparks {
            let a = sp.life.clamp(0.0, 1.0);
            self.dot(sp.x, sp.y, sp.r * 2.6, sp.r * 2.6, D2D1_COLOR_F { r: 1.0 * 0.35 * a, g: 0.85 * 0.35 * a, b: 0.5 * 0.35 * a, a: 0.35 * a });
            self.dot(sp.x, sp.y, sp.r, sp.r, D2D1_COLOR_F { r: 1.0 * a, g: 0.93 * a, b: 0.65 * a, a });
        }
        if let Some((sx, dir)) = self.sleigh {
            if sx > vl - 560.0 && sx < vr + 560.0 {
                self.draw_sleigh(sx, dir);
            }
        }
        for d in &self.debris {
            self.brush.SetOpacity(d.a.clamp(0.0, 1.0));
            let e = D2D1_ELLIPSE { point: D2D_POINT_2F { x: d.x, y: d.y }, radiusX: d.r, radiusY: d.r };
            self.ctx.FillEllipse(&e, &self.brush);
        }
        self.brush.SetOpacity(1.0);
        self.ctx.EndDraw(None, None)?;
        self.ctx.SetTarget(None);
        // Libère toute référence au tampon avant Present (sinon scintillement).
        drop(bitmap);
        drop(surface);
        s.swap.Present(2, DXGI_PRESENT(0)).ok()?;
        Ok(())
    }

    pub(crate) fn apply_visibility(&self) {
        unsafe {
            for s in &self.surfaces {
                let _ = ShowWindow(s.hwnd, if self.active && !s.hidden { SW_SHOWNOACTIVATE } else { SW_HIDE });
            }
        }
    }
}
