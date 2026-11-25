//! Père Noël, traîneau et rennes: constantes et étincelles.
pub(crate) const SLEIGH_K: f32 = 0.42; // échelle du traîneau (pixels d'image -> pixels d'écran)
/// Étincelle de la traînée du traîneau.
pub(crate) struct Spark {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) r: f32,
    pub(crate) life: f32,
}

pub(crate) const SANTA_K: f32 = 0.40; // père Noël (images de marche)
pub(crate) const SANTA_STRIDE: f32 = 80.0; // distance parcourue pendant un cycle complet de 8 images
pub(crate) const SANTA_SPEED: f32 = 120.0;
pub(crate) const DEER_DS: f32 = 0.55; // taille des rennes par rapport au traîneau

