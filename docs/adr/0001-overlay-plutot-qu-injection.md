# Overlay transparent plutôt qu'injection dans explorer.exe

sneakyXmas dessine ses Décorations dans des fenêtres transparentes insensibles aux clics (Overlay) au lieu d'injecter du code dans `explorer.exe` (à la manière de Windhawk). L'Overlay est stable entre les mises à jour de Windows 11 et ne peut pas faire planter l'explorateur, au prix de Décorations « posées par-dessus » et non de vrais éléments de l'interface.

Condition: l'Overlay ne doit jamais perturber les applications en Mode plein écran (il est masqué et ne dessine plus). Si un jeu en plein écran exclusif subit un effet de bord malgré cela, on réévalue l'injection.

## Considered Options

- **Injection / hook dans `explorer.exe`**: rendu réellement intégré, mais fragile à chaque mise à jour de Windows, risqué (plantages, antivirus) et difficile à partager à des amis.
