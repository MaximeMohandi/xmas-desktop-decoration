# 🎄 sneakyXmas

🇫🇷 Français · [🇬🇧 English](README.en.md)

**Noël s'invite discrètement sur ton bureau Windows 11.**

sneakyXmas est une petite application résidente qui pose par-dessus ton bureau une couche de magie hivernale : la neige tombe, s'accumule doucement sur la barre des tâches et sur le bord de tes fenêtres, des petites scènes de Noël s'installent comme dans un village miniature, le Père Noël se promène sur la barre des tâches et son traîneau traverse l'écran.

Elle est discrète : les clics passent au travers, et dès qu'un film ou un jeu passe en plein écran, tout disparaît sans le perturber. ❄️

---

## ✨ Ce que tu vois

- ❄️ **Neige douce** qui tombe sur le bureau, jamais devant tes fenêtres, et s'accumule en petits monticules arrondis sur la barre des tâches et le haut de chaque fenêtre. Déplace une fenêtre : sa neige glisse et tombe.
- 🏘️ **Un village sur la barre des tâches** : un plan d'urbanisme généré avec des repères, des groupes de trois, des vides et des clairières. Un sapin avec sa lanterne et sa maison en pain d'épices à gauche, un bonhomme de neige avec son cadeau à droite, et entre les deux des scènes toutes différentes (cheminée, forêt, pôle, cadeaux, clôture…).
- 🎁 **Une décoration par fenêtre** : chaque nouvelle fenêtre reçoit la sienne, tirée une seule fois, qu'elle garde tant qu'elle existe.
- 🎅 **Le Père Noël** se promène sur la barre des tâches, s'arrête pour saluer, puis repart.
- 🦌 **Le traîneau et ses rennes** traversent le haut de l'écran, avec une traînée d'étincelles.
- 🖥️ **Plusieurs écrans** : un overlay par écran, les barres des tâches de tous les écrans forment un seul terrain continu.
- 🎮 **Respect du plein écran** : un film ou un jeu en plein écran sur un écran masque l'overlay de cet écran, les autres continuent.

## 🎛️ Réglages

Clic droit sur l'icône de la zone de notification :

| Menu | Choix |
|---|---|
| **Mode Noël** | active ou désactive tout |
| **Neige** | Aucune · Légère · Normale · Forte |
| **Décorations** | affiche ou masque les scènes |
| **Père Noël et traîneau automatiques** | ou bien lancés à la demande |
| **Neige accumulée** | Fine · Normale · Épaisse |
| **Fréquence des passages** | Rare · Normale · Fréquente |
| **Lancer le Père Noël / le traîneau** | un passage tout de suite |
| **Lancer avec Windows** | démarrage automatique |
| **Quitter** | |

Les réglages sont mémorisés dans `sneakyxmas.ini`, à côté de l'exécutable. Une seule instance tourne à la fois.

## 🚀 Utilisation

Lance `sneakyxmas.exe` : l'icône apparaît dans la zone de notification et la neige commence à tomber. Aucune installation, aucune dépendance.

## 🔧 Compiler

Prérequis : Windows 11, [Rust](https://rustup.rs) avec la chaîne MSVC (Visual Studio Build Tools).

```
cargo build --release
```

L'exécutable est produit dans `target\release\sneakyxmas.exe`. Les images sont embarquées dans l'exécutable.

## 🧩 Comment ça marche

- **Overlay** : une fenêtre transparente, au premier plan et insensible aux clics, par écran. Rendu en Direct2D sur une chaîne d'affichage DirectComposition, 30 images par seconde.
- **Ancres** : la barre des tâches et chaque fenêtre ouverte sont suivies (position, ordre d'affichage) pour accrocher la neige et les décors, et pour les masquer quand une autre fenêtre passe devant.
- **Rien n'est injecté** dans Explorer ni dans tes applications : tout est dessiné par-dessus.
- Code en Rust, découpé en modules (`app`, `desktop`, `snow`, `scenes`, `sprites`, `settings`, `tray`).

```
src/
├── main.rs        démarrage, instance unique
├── app/           état, simulation (sim) et dessin (render), un overlay par écran (surface)
├── desktop.rs     fenêtres, ancres, plein écran
├── snow.rs        flocons, tas, couche de départ
├── scenes.rs      scènes, accents, plan de la barre des tâches
├── sprites.rs     chargement des images
├── settings.rs    fichier ini, démarrage avec Windows
└── tray.rs        icône de notification et menu
```

## 🗺️ Au programme

- 🔔 Un schéma de sons de Noël pour Windows, avec sauvegarde et restauration automatique de tes sons d'origine.
- ⚙️ Réglages supplémentaires.

## 🖼️ Images

Les illustrations ont été générées par IA par l'auteur pour un usage personnel et entre amis. Elles ne sont pas libres de droits : ne les redistribue pas à des fins commerciales.

---

*Joyeuses fêtes, et bonne neige !* 🎅❄️🎄
