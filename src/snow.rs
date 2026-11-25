//! La neige: flocons, tas accumulés, paquets détachés.
use crate::prelude::*;

pub(crate) const BUMP_SIGMA: f32 = 5.0; // largeur (en colonnes) de la bosse déposée par un flocon
pub(crate) const BUMP_AMP: f32 = 0.5; // hauteur (px) de cette bosse
pub(crate) const REPOSE: f32 = 1.0; // pente maximale entre colonnes voisines (angle d'éboulement)


pub(crate) struct Flake {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) r: f32,
    pub(crate) speed: f32,
    pub(crate) phase: f32,
}

/// Paquet de neige détaché d'un tas quand sa fenêtre bouge.
pub(crate) struct Debris {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) r: f32,
    pub(crate) a: f32,
}

/// La neige glisse de la fenêtre: le tas devient des paquets qui tombent.
pub(crate) fn shake_off(a: &mut Anchor, debris: &mut Vec<Debris>, rng: &mut Rng) {
    for (i, h) in a.pile.iter_mut().enumerate() {
        if *h > 1.5 && i % 4 == 0 && debris.len() < 600 {
            debris.push(Debris {
                x: a.l + i as f32 * COL_W,
                y: a.t - *h,
                vx: (rng.next() - 0.5) * 120.0,
                vy: rng.next() * 60.0,
                r: 1.5 + rng.next() * 2.5,
                a: 1.0,
            });
        }
        *h = 0.0;
    }
}

/// Fine couche de neige de départ (≈ 1 à 4 px), ondulée de façon douce et propre à chaque graine.
pub(crate) fn floor_h(seed: u32, i: usize) -> f32 {
    let s1 = (seed & 0xFF) as f32 * 0.1;
    let s2 = ((seed >> 8) & 0xFF) as f32 * 0.1;
    let x = i as f32;
    (2.4 + 1.1 * (x * 0.045 + s1).sin() + 0.7 * (x * 0.13 + s2).sin()).max(1.2)
}

