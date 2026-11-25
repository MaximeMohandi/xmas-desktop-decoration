//! Simulation: fenêtres, neige, personnages.
use crate::prelude::*;

impl App {
    pub(crate) fn is_rendering(&self) -> bool {
        self.active && !self.hidden_for_fullscreen
    }

    /// Clé 0 = barre des tâches (en bas de l'écran principal uniquement pour l'instant).
    pub(crate) fn update_taskbar(&mut self) {
        unsafe {
            let mut r = RECT::default();
            if let Ok(tb) = FindWindowW(w!("Shell_TrayWnd"), None) {
                if GetWindowRect(tb, &mut r).is_ok()
                    && (r.top as f32) < self.h - 2.0
                    && r.top as f32 > self.h * 0.5
                    && r.left < r.right
                {
                    let a = self.anchors.entry(0).or_insert(Anchor { l: 0.0, t: 0.0, r: 0.0, pile: Vec::new(), z: None, px: f32::NAN, py: f32::NAN, still: 0.0, seed: 0, kind: 0, variant: 0, left: false, hh: 60.0, deco: 0.0 });
                    a.set_geometry(r.left as f32, r.top as f32, r.right as f32);
                    a.still = 99.0; // la barre ne bouge pas: couche de neige dès le départ
                    return;
                }
            }
            self.anchors.remove(&0); // masquée ou ailleurs: pas d'accumulation
        }
    }

    /// Réénumère les fenêtres (ordre d'affichage) et crée/supprime les Ancres.
    pub(crate) fn refresh_windows(&mut self) {
        ENUM_WINS.with(|w| w.borrow_mut().clear());
        TASKBAR_Z.with(|z| z.set(None));
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM(0));
        }
        self.wins = ENUM_WINS.with(|w| std::mem::take(&mut *w.borrow_mut()));
        let mut keep = vec![0isize];
        for (z, w) in self.wins.iter().enumerate() {
            // Place pour le tas au-dessus de la fenêtre.
            if w.snowy && w.t > PILE_MAXS[self.cfg.pile.min(2)] + 4.0 && w.t < self.h {
                keep.push(w.hwnd);
                // Décoration tirée une seule fois, à la création de la fenêtre, en évitant celles déjà utilisées.
                let fresh = !self.anchors.contains_key(&w.hwnd);
                let pick = if fresh {
                    let n = SCENES.len() + ACCENTS.len();
                    let used: Vec<usize> = self.anchors.iter().filter(|(k, _)| **k != 0).map(|(_, a)| a.kind).collect();
                    let free: Vec<usize> = (0..n).filter(|k| !used.contains(k)).collect();
                    let kind = if free.is_empty() {
                        (self.rng.next() * n as f32) as usize % n
                    } else {
                        free[(self.rng.next() * free.len() as f32) as usize % free.len()]
                    };
                    (kind, (self.rng.next() * 1000.0) as u32, self.rng.next() < 0.5, 56.0 + self.rng.next() * 16.0)
                } else {
                    (0, 0, false, 0.0)
                };
                let a = self.anchors.entry(w.hwnd).or_insert(Anchor { l: 0.0, t: 0.0, r: 0.0, pile: Vec::new(), z: None, px: f32::NAN, py: f32::NAN, still: 0.0, seed: 0, kind: 0, variant: 0, left: false, hh: 60.0, deco: 0.0 });
                a.set_geometry(w.l, w.t, w.r);
                a.z = Some(z);
                if a.seed == 0 {
                    a.seed = ((w.hwnd as u32).wrapping_mul(2654435761) ^ self.salt) | 1;
                    (a.kind, a.variant, a.left, a.hh) = pick;
                }
            }
        }
        self.anchors.retain(|k, _| keep.contains(k));
        let tz = TASKBAR_Z.with(|z| z.get());
        if let Some(a) = self.anchors.get_mut(&0) {
            a.z = tz;
        }
    }

    /// Suit les fenêtres suivies; si l'une bouge, sa neige se détache.
    pub(crate) fn track_windows(&mut self, dt: f32) {
        for w in self.wins.iter_mut().filter(|w| w.snowy) {
            let Some((l, t, r, b)) = frame_bounds(HWND(w.hwnd as *mut _)) else { continue };
            (w.l, w.t, w.r, w.b) = (l, t, r, b);
            if let Some(a) = self.anchors.get_mut(&w.hwnd) {
                a.set_geometry(l, t, r);
                if !a.px.is_nan() && (l - a.px).abs() + (t - a.py).abs() > 1.0 {
                    shake_off(a, &mut self.debris, &mut self.rng);
                    a.still = 0.0;
                } else {
                    a.still += dt;
                }
                (a.px, a.py) = (l, t);
            }
        }
    }

    pub(crate) fn tick(&mut self) {
        self.ticks += 1;
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
        self.last = now;

        // Mode plein écran / écran verrouillé: on masque et on ne dessine plus.
        if now.duration_since(self.last_fs_check).as_secs_f32() >= 0.5 {
            self.last_fs_check = now;
            let fullscreen = unsafe {
                matches!(
                    SHQueryUserNotificationState(),
                    Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
                ) || foreground_covers_monitor(self.hwnd)
            };
            if fullscreen != self.hidden_for_fullscreen {
                self.hidden_for_fullscreen = fullscreen;
                self.apply_visibility();
            }
        }
        if !self.is_rendering() {
            return;
        }
        if self.ticks % 30 == 0 {
            self.update_taskbar();
        }
        if self.ticks % 3 == 0 || self.wins.is_empty() {
            self.refresh_windows();
        }
        self.track_windows(dt);
        // Les décorations d'une fenêtre apparaissent en fondu quand elle est immobile.
        for a in self.anchors.values_mut() {
            let target = if a.still > 0.25 { 1.0 } else { 0.0 };
            let step = 3.0 * dt;
            a.deco += (target - a.deco).clamp(-step, step);
        }

        self.t += dt;
        self.update_characters(dt);
        let nflakes = SNOW_COUNT[self.cfg.snow.min(3)];
        for f in self.flakes.iter_mut().take(nflakes) {
            let prev_y = f.y;
            f.y += f.speed * dt;
            f.x += (self.t * 0.8 + f.phase).sin() * 20.0 * dt;
            let mut landed = false;
            for (key, a) in self.anchors.iter_mut() {
                let Some(c) = a.col(f.x) else { continue };
                let level = a.t - a.pile[c];
                let is_taskbar = *key == 0;
                // Fenêtre: le flocon doit arriver d'au-dessus du bord, loin des coins.
                // Barre des tâches: tout flocon qui atteint le tas se pose (sinon il traverserait la barre).
                let from_above = prev_y + f.r <= level + 1.0;
                let reaches = f.y + f.r >= level && (from_above || (is_taskbar && f.y < a.t + 60.0));
                let away_from_corner = is_taskbar || (f.x - a.l > 8.0 && a.r - f.x > 8.0);
                if reaches && away_from_corner && !covered(&self.wins, a.z, f.x, a.t - 1.0) {
                    // Une bosse arrondie (gaussienne) plutôt qu'une colonne: le tas grossit en douceur.
                    let rad = (BUMP_SIGMA * 2.6) as i32;
                    for d in -rad..=rad {
                        let idx = c as i32 + d;
                        if idx >= 0 && (idx as usize) < a.pile.len() {
                            let wgt = (-((d * d) as f32) / (2.0 * BUMP_SIGMA * BUMP_SIGMA)).exp();
                            a.pile[idx as usize] = (a.pile[idx as usize] + BUMP_AMP * wgt).min(PILE_MAXS[self.cfg.pile.min(2)]);
                        }
                    }
                    landed = true;
                    break;
                }
            }
            if landed || f.y > self.h + 5.0 {
                f.y = -5.0;
                f.x = self.rng.next() * self.w;
            }
        }
        // Angle d'éboulement: la neige ne tient pas en pente raide, elle glisse vers le côté plus bas.
        for a in self.anchors.values_mut() {
            let n = a.pile.len();
            for _ in 0..2 {
                for i in 0..n.saturating_sub(1) {
                    let d = a.pile[i] - a.pile[i + 1];
                    if d > REPOSE {
                        let m = (d - REPOSE) * 0.5;
                        a.pile[i] -= m;
                        a.pile[i + 1] += m;
                    } else if d < -REPOSE {
                        let m = (-d - REPOSE) * 0.5;
                        a.pile[i] += m;
                        a.pile[i + 1] -= m;
                    }
                }
            }
        }
        // Fine couche de neige de départ: elle revient peu après qu'une fenêtre se soit arrêtée.
        for a in self.anchors.values_mut() {
            let scale = ((a.still - 0.3) / 1.5).clamp(0.0, 1.0);
            let sd = a.seed | 1;
            for (i, h) in a.pile.iter_mut().enumerate() {
                let f = floor_h(sd, i) * scale;
                if *h < f {
                    *h = f;
                }
            }
        }
        // Chute des paquets de neige détachés.
        for d in &mut self.debris {
            d.vy += 900.0 * dt;
            d.x += d.vx * dt;
            d.y += d.vy * dt;
            d.a -= 1.1 * dt;
        }
        let h = self.h;
        self.debris.retain(|d| d.a > 0.0 && d.y < h);
        if let Err(e) = self.draw() {
            // Une erreur de dessin ne doit jamais faire tourner la boucle à vide: on trace une fois.
            if !self.draw_failed {
                self.draw_failed = true;
                let _ = std::fs::write("sneakyxmas.log", format!("draw: {e:?}\n"));
            }
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    }

    /// Père Noël qui marche sur la barre des tâches, traîneau qui traverse le haut de l'écran.
    pub(crate) fn update_characters(&mut self, dt: f32) {
        let tb = self.anchors.get(&0).map(|a| (a.l, a.r));
        if let Some((al, ar)) = tb {
            match self.santa_x {
                Some(x) => {
                    if self.santa_pause > 0.0 {
                        self.santa_pause -= dt;
                    } else {
                        let nx = x + SANTA_SPEED * dt;
                        self.santa_dist += SANTA_SPEED * dt;
                        if nx > ar + 60.0 {
                            self.santa_x = None;
                            self.next_santa = self.t + (120.0 + self.rng.next() * 60.0) * FREQ_MUL[self.cfg.freq.min(2)];
                        } else {
                            self.santa_x = Some(nx);
                            // À mi-parcours il s'arrête pour saluer.
                            if !self.santa_waved && nx >= self.santa_wave_x {
                                self.santa_waved = true;
                                self.santa_pause = 2.6;
                            }
                        }
                    }
                }
                None if self.t >= self.next_santa && (self.cfg.auto_chars || self.next_santa == 0.0) => {
                    self.santa_x = Some(al - 60.0);
                    self.santa_dist = 0.0;
                    self.santa_pause = 0.0;
                    self.santa_waved = false;
                    self.santa_wave_x = al + (ar - al) * (0.35 + self.rng.next() * 0.3);
                }
                None => {}
            }
        }
        match self.sleigh {
            Some((x, dir)) => {
                let nx = x + dir * (self.w + 1000.0) / 11.0 * dt;
                if nx < -520.0 || nx > self.w + 520.0 {
                    self.sleigh = None;
                    self.next_sleigh = self.t + (240.0 + self.rng.next() * 120.0) * FREQ_MUL[self.cfg.freq.min(2)];
                } else {
                    self.sleigh = Some((nx, dir));
                }
            }
            None if self.t >= self.next_sleigh && (self.cfg.auto_chars || self.next_sleigh == 0.0) => {
                let dir = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                self.sleigh = Some((if dir > 0.0 { -500.0 } else { self.w + 500.0 }, dir));
            }
            None => {}
        }
        // Traînée d'étincelles derrière le traîneau.
        if let Some((x, dir)) = self.sleigh {
            let y = self.sleigh_y();
            self.spark_acc += dt;
            while self.spark_acc > 0.02 {
                self.spark_acc -= 0.02;
                if self.sparks.len() < 400 {
                    self.sparks.push(Spark {
                        x: x + (self.rng.next() - 0.5) * 14.0,
                        y: y + 90.0 + self.rng.next() * 70.0,
                        vx: -dir * (30.0 + self.rng.next() * 70.0),
                        vy: (self.rng.next() - 0.3) * 40.0,
                        r: 1.2 + self.rng.next() * 2.6,
                        life: 1.0,
                    });
                }
            }
        } else {
            self.spark_acc = 0.0;
        }
        for sp in &mut self.sparks {
            sp.x += sp.vx * dt;
            sp.y += sp.vy * dt;
            sp.vy += 25.0 * dt;
            sp.life -= 0.9 * dt;
        }
        self.sparks.retain(|sp| sp.life > 0.0);
    }

    pub(crate) fn sleigh_y(&self) -> f32 {
        self.h * 0.12 + (self.t * 2.2).sin() * 10.0
    }
}
