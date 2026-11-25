//! Scènes de décoration: compositions de pièces posées sur la neige.
/// Source d'une image de décor: collection d'icônes (numérotée comme dans la planche, de 1 à 40),
/// personnages/objets debout, sapins, ou un bonhomme de neige au hasard (selon la variante).
#[derive(Clone, Copy)]
pub(crate) enum Src {
    Ico(usize),
    Chr(usize),
    Tree(usize),
    Snowman,
}
pub(crate) use Src::*;

/// Une pièce d'une scène: hauteur (fraction de la hauteur de scène) et décalage horizontal (en hauteurs de scène).
#[derive(Clone, Copy)]
pub(crate) struct Piece {
    pub(crate) src: Src,
    pub(crate) h: f32,
    pub(crate) dx: f32,
}
pub(crate) const fn pc(src: Src, h: f32, dx: f32) -> Piece {
    Piece { src, h, dx }
}

/// Compositions pensées: la pièce principale d'abord (dessinée devant), puis les accessoires de chaque côté.
pub(crate) static SCENES: [&[Piece]; 8] = [
    // Bonhomme de neige avec un cadeau et une lanterne
    &[pc(Snowman, 1.0, 0.0), pc(Ico(4), 0.30, -0.52), pc(Ico(37), 0.55, 0.50)],
    // Pile de cadeaux, canne en sucre
    &[pc(Chr(12), 0.62, 0.0), pc(Ico(18), 0.55, -0.42), pc(Ico(6), 0.28, 0.42)],
    // Petite forêt: sapin enneigé, sapin simple, jeune renne
    &[pc(Tree(0), 1.0, 0.0), pc(Ico(3), 0.70, -0.55), pc(Chr(5), 0.60, 0.55)],
    // Pôle: ours, pingouin, enseigne
    &[pc(Chr(4), 0.80, 0.0), pc(Chr(3), 0.62, -0.55), pc(Ico(36), 0.55, 0.50)],
    // Maison en pain d'épices, bougie, biscuit
    &[pc(Ico(20), 0.70, 0.0), pc(Ico(21), 0.40, -0.50), pc(Ico(19), 0.45, 0.48)],
    // Cheminée, cadeau, lanterne
    &[pc(Ico(39), 0.65, 0.0), pc(Ico(4), 0.30, -0.45), pc(Ico(37), 0.55, 0.50)],
    // Sapin décoré, clôture, cadeau vert
    &[pc(Tree(1), 1.0, 0.0), pc(Ico(38), 0.40, 0.55), pc(Ico(5), 0.30, -0.50)],
    // Bonhomme à bonnet rouge, pingouin, cadeau
    &[pc(Chr(1), 1.0, 0.0), pc(Chr(3), 0.60, -0.55), pc(Ico(5), 0.28, 0.50)],
];

/// Scènes fixes de la barre des tâches (fournies une fois pour toutes, sans tirage au hasard).
pub(crate) static TB_LEFT: [Piece; 3] = [pc(Tree(1), 1.0, 0.0), pc(Ico(37), 0.55, -0.52), pc(Ico(20), 0.55, 0.55)]; // sapin, lanterne, maison
pub(crate) static TB_RIGHT: [Piece; 2] = [pc(Chr(0), 1.0, 0.0), pc(Ico(4), 0.30, -0.50)]; // bonhomme de neige, cadeau

/// Pièces isolées (un seul accent) pour les fenêtres: un seul accent, jamais une scène complète.
pub(crate) static ACCENTS: [(Src, f32); 16] = [
    (Ico(18), 0.60), // canne en sucre
    (Ico(30), 0.55), // cloche
    (Ico(10), 0.40), // poinsettia
    (Ico(16), 0.50), // étoile
    (Ico(28), 0.45), // moufle
    (Ico(27), 0.60), // chaussette
    (Ico(21), 0.50), // bougie
    (Ico(37), 0.70), // lanterne
    (Ico(36), 0.70), // enseigne
    (Ico(19), 0.60), // pain d'épices
    (Ico(11), 0.40), // houx
    (Ico(8), 0.42),  // boule dorée
    (Chr(3), 0.80),  // pingouin
    (Chr(5), 0.80),  // renne
    (Ico(4), 0.50),  // cadeau
    (Ico(20), 0.80), // maison en pain d'épices
];

