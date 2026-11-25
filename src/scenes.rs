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


/// Élément du plan de la barre des tâches.
#[derive(Clone, Copy)]
pub(crate) enum TbKind {
    Left,
    Right,
    Scene(usize),
    Accent(usize),
}

pub(crate) struct TbItem {
    /// Abscisse depuis le début du terrain.
    pub(crate) x: f32,
    pub(crate) kind: TbKind,
    pub(crate) hh: f32,
    pub(crate) mirror: bool,
    pub(crate) variant: u32,
}

/// Sac mélangé: on pioche tout avant de reprendre le même élément, et jamais deux fois de suite le même.
struct Bag {
    items: Vec<usize>,
    n: usize,
    last: usize,
}
impl Bag {
    fn new(n: usize) -> Bag {
        Bag { items: Vec::new(), n, last: usize::MAX }
    }
    fn draw(&mut self, rng: &mut crate::util::Rng) -> usize {
        if self.items.is_empty() {
            self.items = (0..self.n).collect();
            for i in (1..self.items.len()).rev() {
                let j = (rng.next() * (i + 1) as f32) as usize % (i + 1);
                self.items.swap(i, j);
            }
            if self.items.last() == Some(&self.last) && self.items.len() > 1 {
                let k = self.items.len();
                self.items.swap(0, k - 1);
            }
        }
        let v = self.items.pop().unwrap();
        self.last = v;
        v
    }
}

/// Plan d'un terrain de longueur `len`, pensé comme un petit plan d'urbanisme:
/// un repère de chaque côté, au milieu des groupes (un repère seul, ou accompagné de deux petits accents)
/// séparés par des vides de longueur variable, avec de temps en temps une longue clairière.
/// Déterministe: le même terrain donne toujours le même plan.
pub(crate) fn plan_terrain(len: f32) -> Vec<TbItem> {
    let mut rng = crate::util::Rng(0x5EED_CAFE_1234_ABCD ^ (len as u64).wrapping_mul(0x9E37_79B9));
    let mut items = vec![TbItem { x: 170.0, kind: TbKind::Left, hh: 96.0, mirror: false, variant: 0 }];
    let end = len - 280.0;
    let mut scenes = Bag::new(SCENES.len());
    let mut accents = Bag::new(ACCENTS.len());
    let mut x = 170.0 + 420.0 + rng.next() * 160.0;
    while x < end - 420.0 {
        // Le repère du groupe: grande scène, taille variable.
        let sc = scenes.draw(&mut rng);
        let hh = 74.0 + rng.next() * 34.0;
        let mirror = rng.next() < 0.5;
        items.push(TbItem { x, kind: TbKind::Scene(sc), hh, mirror, variant: (rng.next() * 1000.0) as u32 });
        // Groupe de trois (repère + deux accents) une fois sur deux, sinon le repère reste seul.
        if rng.next() < 0.5 {
            let off = 130.0 + rng.next() * 40.0;
            for side in [-1.0f32, 1.0] {
                let ac = accents.draw(&mut rng);
                items.push(TbItem {
                    x: x + side * (off + rng.next() * 25.0),
                    kind: TbKind::Accent(ac),
                    hh: 82.0 + rng.next() * 24.0,
                    mirror: false,
                    variant: (rng.next() * 1000.0) as u32,
                });
            }
        }
        // Vide avant le prochain groupe: souvent court, parfois une longue clairière.
        x += if rng.next() < 0.25 { 950.0 + rng.next() * 550.0 } else { 440.0 + rng.next() * 400.0 };
    }
    items.push(TbItem { x: end, kind: TbKind::Right, hh: 74.0, mirror: true, variant: 0 });
    items
}
